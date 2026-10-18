use super::*;
use crate::persistence;
use redb::{StorageBackend, backends::FileBackend};
use std::io;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::time::Duration;

#[derive(Debug)]
struct SyncGate {
    entered: mpsc::Sender<()>,
    release: mpsc::Receiver<()>,
    rescued: Arc<AtomicBool>,
}

#[derive(Debug)]
struct GatedFile {
    file: FileBackend,
    gate: Arc<Mutex<Option<SyncGate>>>,
}

impl StorageBackend for GatedFile {
    fn len(&self) -> io::Result<u64> {
        self.file.len()
    }
    fn read(&self, offset: u64, out: &mut [u8]) -> io::Result<()> {
        self.file.read(offset, out)
    }
    fn set_len(&self, len: u64) -> io::Result<()> {
        self.file.set_len(len)
    }
    fn write(&self, offset: u64, data: &[u8]) -> io::Result<()> {
        self.file.write(offset, data)
    }
    fn sync_data(&self) -> io::Result<()> {
        let gate = self.gate.lock().unwrap().take();
        if let Some(gate) = gate {
            gate.entered.send(()).unwrap();
            // Deadlock rescue only: the assertion fails if this releases the writer.
            if gate.release.recv_timeout(Duration::from_secs(20)).is_err() {
                gate.rescued.store(true, Ordering::SeqCst);
            }
        }
        self.file.sync_data()
    }
}

struct HeldSync {
    entered: mpsc::Receiver<()>,
    release: mpsc::Sender<()>,
    rescued: Arc<AtomicBool>,
}

impl HeldSync {
    fn arm(gate: &Mutex<Option<SyncGate>>) -> Self {
        let (entered_tx, entered) = mpsc::channel();
        let (release, release_rx) = mpsc::channel();
        let rescued = Arc::new(AtomicBool::new(false));
        assert!(
            gate.lock()
                .unwrap()
                .replace(SyncGate {
                    entered: entered_tx,
                    release: release_rx,
                    rescued: rescued.clone(),
                })
                .is_none()
        );
        Self {
            entered,
            release,
            rescued,
        }
    }

    fn wait(&self) {
        self.entered
            .recv_timeout(Duration::from_secs(25))
            .expect("real sync reached");
    }

    fn assert_held(&self) {
        assert!(
            !self.rescued.load(Ordering::SeqCst),
            "fixed ticks waited for sync deadlock rescue"
        );
    }
}

impl Drop for HeldSync {
    fn drop(&mut self) {
        let _ = self.release.send(());
    }
}

enum Step {
    Tick,
    Drain,
}

#[derive(Debug)]
struct Observed {
    ticks: u32,
    last_cycle: u64,
    auctions: usize,
    durable_cycle: u64,
}

#[test]
fn economy_allocates_ids_without_blocking_fixed_ticks_on_real_sync() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("economy.redb");
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(&path)
        .unwrap();
    let gate = Arc::new(Mutex::new(None));
    let db = redb::Builder::new()
        .create_with_backend(GatedFile {
            file: FileBackend::new(file).unwrap(),
            gate: gate.clone(),
        })
        .unwrap();
    persistence::create_tables(&db);
    store::init_state_table(&db).unwrap();
    let (config, state) = super::tests::fixture();
    store::commit_market(&db, Some(&state), &store::MarketChanges::default()).unwrap();
    let (commands, steps) = mpsc::channel();
    let (observed_tx, observed) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(GameDatabase::new(db));
        app.insert_resource(config);
        app.insert_resource(Economy::from_state(state));
        app.insert_resource(AuctionHouseState {
            initialized: true,
            ..Default::default()
        });
        app.init_resource::<crate::gold::GoldUpdateQueue>();
        app.add_systems(FixedUpdate, update_economy);
        let mut ticks = 0;
        for step in steps {
            match step {
                Step::Tick => {
                    app.world_mut().run_schedule(FixedUpdate);
                    ticks += 1;
                }
                Step::Drain => {
                    crate::test_data::drain_persistence(app.world().resource::<GameDatabase>())
                }
            }
            let state = &app.world().resource::<Economy>().state;
            let database = &app.world().resource::<GameDatabase>().db;
            observed_tx
                .send(Observed {
                    ticks,
                    last_cycle: state.last_cycle,
                    auctions: app.world().resource::<AuctionHouseState>().auctions.len(),
                    durable_cycle: store::load_state(database).unwrap().unwrap().last_cycle,
                })
                .unwrap();
        }
        app.world_mut()
            .remove_resource::<GameDatabase>()
            .unwrap()
            .into_database()
    });
    let receive = || {
        observed
            .recv_timeout(Duration::from_secs(25))
            .expect("fixed step completed")
    };
    let reservation = HeldSync::arm(&gate);
    commands.send(Step::Tick).unwrap();
    reservation.wait();
    let first = receive();
    commands.send(Step::Tick).unwrap();
    let second = receive();
    reservation.assert_held();
    assert_eq!((first.ticks, second.ticks), (1, 2));
    assert_eq!(
        (second.last_cycle, second.durable_cycle, second.auctions),
        (0, 0, 0)
    );
    drop(reservation);
    commands.send(Step::Drain).unwrap();
    receive();

    let commit = HeldSync::arm(&gate);
    commands.send(Step::Tick).unwrap();
    commit.wait();
    let prepared = receive();
    commands.send(Step::Tick).unwrap();
    let progressing = receive();
    commit.assert_held();
    assert_eq!((prepared.ticks, progressing.ticks), (3, 4));
    assert_eq!(
        (
            progressing.last_cycle,
            progressing.durable_cycle,
            progressing.auctions
        ),
        (0, 0, 0)
    );
    drop(commit);
    commands.send(Step::Drain).unwrap();
    receive();
    commands.send(Step::Tick).unwrap();
    let published = receive();
    assert_ne!(published.last_cycle, 0);
    assert_eq!(published.last_cycle, published.durable_cycle);
    assert!(published.auctions > 0);
    drop(commands);
    let db = worker.join().unwrap();
    let prior = persistence::next_id(&db, "item_instance");
    drop(db);
    let reopened = redb::Database::open(&path).unwrap();
    assert!(persistence::next_id(&reopened, "item_instance") > prior);
    assert!(!persistence::load_auctions(&reopened).is_empty());
}

pub(crate) fn advance_fixture(app: &mut App, now: u64) {
    fn advance(
        In(now): In<u64>,
        db: Res<GameDatabase>,
        config: Res<Config>,
        mut economy: ResMut<Economy>,
        mut auctions: ResMut<AuctionHouseState>,
        mut gold: ResMut<crate::gold::GoldUpdateQueue>,
    ) {
        publication::advance_cycle(&db, &config, &mut economy, &mut auctions, &mut gold, now);
    }
    app.world_mut()
        .run_system_cached_with(advance, now)
        .unwrap();
}

#[test]
fn economy_rejects_mailbox_replacement_after_real_concurrent_auction_buyout() {
    use crate::auction_house::tests::criteria::prepare_auction_players;
    use shared::protocol::{AuctionChannel, AuctionDuration, BuyoutAuction, CreateAuction};
    let file = tempfile::NamedTempFile::new().unwrap();
    let (mut h, buyer, _) = prepare_auction_players(file.path());
    let seller = h.character_id;
    let mut inventory = persistence::load_inventory(h.db(), seller);
    let guid = persistence::next_id(h.db(), "item_instance");
    inventory.items.push(persistence::InventoryItemData {
        item_guid: guid,
        item_id: 2318,
        name: "Light Leather".into(),
        quality: 1,
        required_level: 0,
        stack_count: 1,
        vendor_sell_price: 1,
        unique_auction: false,
    });
    persistence::save_inventory(h.db(), seller, &inventory);
    h.send::<_, AuctionChannel>(CreateAuction {
        item_guid: guid,
        stack_count: 1,
        min_bid: 10,
        buyout_price: Some(10),
        duration: AuctionDuration::Short,
    });
    let auction = h
        .app
        .world()
        .resource::<AuctionHouseState>()
        .auctions
        .values()
        .next()
        .unwrap()
        .clone();
    let gold_before = persistence::load_inventory(h.db(), seller).gold;
    let (mut config, mut state) = super::tests::fixture();
    config.merchants.clear();
    config.gatherer_listing_limit = 1;
    config.gatherer_capital = inventory.gold + 1000;
    state.merchants.clear();
    state.gatherers.insert(2318, seller);
    store::init_state_table(h.db()).unwrap();
    store::commit_market(h.db(), Some(&state), &store::MarketChanges::default()).unwrap();
    h.app
        .insert_resource(config)
        .insert_resource(Economy::from_state(state));
    let now = crate::auction_house::unix_now();
    advance_fixture(&mut h.app, now);
    crate::test_data::drain_persistence(h.app.world().resource::<GameDatabase>());
    let (ready, entered) = mpsc::channel();
    let (release, held) = mpsc::channel();
    h.app.world().resource::<GameDatabase>().writer.enqueue(
        "hold before cycle mailbox commit",
        move |_| {
            ready.send(()).unwrap();
            held.recv().unwrap();
            Ok(())
        },
    );
    entered.recv_timeout(Duration::from_secs(20)).unwrap();
    advance_fixture(&mut h.app, now);
    assert!(h.app.world().resource::<Economy>().market_commit_pending());
    h.send_from::<_, AuctionChannel>(
        buyer,
        BuyoutAuction {
            auction_id: auction.auction_id,
        },
    );
    let sold_mail = persistence::mail_store::load_mailbox(h.db(), seller);
    assert_eq!(sold_mail.len(), 1, "real buyout must append seller mail");
    assert!(sold_mail[0].money > 0);
    assert!(persistence::load_auctions(h.db()).is_empty());
    release.send(()).unwrap();
    crate::test_data::drain_persistence(h.app.world().resource::<GameDatabase>());
    advance_fixture(&mut h.app, now);
    assert_eq!(
        persistence::mail_store::load_mailbox(h.db(), seller),
        sold_mail,
        "prepared cycle erased actual buyout proceeds"
    );
    assert_eq!(h.app.world().resource::<Economy>().state.last_cycle, 0);
    assert!(!h.app.world().resource::<Economy>().market_commit_pending());
    for _ in 0..3 {
        advance_fixture(&mut h.app, now + 60);
        crate::test_data::drain_persistence(h.app.world().resource::<GameDatabase>());
    }
    assert_eq!(
        h.app.world().resource::<Economy>().state.last_cycle,
        now + 60
    );
    let listings = persistence::load_auctions(h.db());
    assert_eq!(listings.len(), 1);
    assert_eq!(
        persistence::load_inventory(h.db(), seller).gold,
        gold_before + sold_mail[0].money - listings[0].deposit
    );
    assert!(persistence::mail_store::load_mailbox(h.db(), seller).is_empty());
}

#[test]
fn economy_failed_cycle_does_not_retry_and_next_scheduled_cycle_commits() {
    let db = persistence::temp_database();
    store::init_state_table(&db).unwrap();
    let (config, mut state) = super::tests::fixture();
    let seller = persistence::next_id(&db, "character");
    state.gatherers.insert(2318, seller);
    persistence::save_inventory(
        &db,
        seller,
        &persistence::InventoryData {
            gold: 1000,
            items: Vec::new(),
        },
    );
    store::commit_market(&db, Some(&state), &store::MarketChanges::default()).unwrap();
    let mut app = App::new();
    app.insert_resource(GameDatabase::new(db));
    app.insert_resource(config);
    app.insert_resource(Economy::from_state(state.clone()));
    app.init_resource::<AuctionHouseState>();
    app.init_resource::<crate::gold::GoldUpdateQueue>();
    advance_fixture(&mut app, 1000);
    crate::test_data::drain_persistence(app.world().resource::<GameDatabase>());
    let (entered_tx, entered) = mpsc::channel();
    let (release, held) = mpsc::channel();
    app.world()
        .resource::<GameDatabase>()
        .writer
        .enqueue("hold cycle behind fixture", move |_| {
            entered_tx.send(()).unwrap();
            held.recv().unwrap();
            Ok(())
        });
    entered.recv_timeout(Duration::from_secs(20)).unwrap();
    advance_fixture(&mut app, 1000);
    assert!(app.world().resource::<Economy>().market_commit_pending());
    let db = &app.world().resource::<GameDatabase>().db;
    persistence::save_inventory(
        db,
        seller,
        &persistence::InventoryData {
            gold: 999,
            items: Vec::new(),
        },
    );
    release.send(()).unwrap();
    crate::test_data::drain_persistence(app.world().resource::<GameDatabase>());
    advance_fixture(&mut app, 1000);
    assert!(!app.world().resource::<Economy>().market_commit_pending());
    assert_eq!(app.world().resource::<Economy>().state, state);
    assert!(
        app.world()
            .resource::<AuctionHouseState>()
            .auctions
            .is_empty()
    );
    advance_fixture(&mut app, 1001);
    assert!(app.world().resource::<Economy>().pending.is_none());
    for _ in 0..3 {
        advance_fixture(&mut app, 1060);
        crate::test_data::drain_persistence(app.world().resource::<GameDatabase>());
    }
    assert_eq!(app.world().resource::<Economy>().state.last_cycle, 1060);
    assert_eq!(
        store::load_state(&app.world().resource::<GameDatabase>().db)
            .unwrap()
            .unwrap()
            .last_cycle,
        1060
    );
}
