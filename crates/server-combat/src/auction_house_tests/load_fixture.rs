//! Disposable load boundary, not a production listing policy.
use crate::auction_house::{browse::browse_for_viewer, queries::search_auctions, unix_now};
use crate::economy::{
    Config,
    model::quote_price,
    store::{MarketChanges, commit_market},
};
use crate::item_templates::{ItemTemplates, load_item_templates};
use crate::persistence::{self, AuctionData, InventoryItemData, NEXT_IDS};
use redb::{Database, ReadableDatabase};
use shared::protocol::AuctionSearchQuery;
use std::{
    collections::{BTreeSet, HashMap},
    fs::OpenOptions,
    path::{Path, PathBuf},
};

type FixtureResult<T> = Result<T, Box<dyn std::error::Error>>;
const OFFER_COUNT: u64 = 300_000;
const BATCH_SIZE: u64 = 10_000;
const SELLER_ID: u64 = 1;
const OWNER: &str = "owned-disposable-ah-load-fixture";

#[test]
fn load_fixture_refuses_existing_database_without_changing_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("existing.redb");
    std::fs::write(&path, b"user-owned sentinel").unwrap();
    assert!(claim_database_path(&path).is_err());
    assert_eq!(std::fs::read(path).unwrap(), b"user-owned sentinel");
}

#[test]
fn load_fixture_claim_reopens_and_preserves_reserved_counters() -> FixtureResult<()> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("owned.redb");
    claim_database_path(&path)?;
    let db = Database::create(&path)?;
    persistence::create_tables(&db);
    reserve_fixture_ids(&db)?;
    drop(db);
    let reopened = Database::open(&path)?;
    assert_eq!(persistence::next_id(&reopened, "auction"), OFFER_COUNT + 1);
    assert_eq!(
        persistence::next_id(&reopened, "item_instance"),
        OFFER_COUNT + 1
    );
    assert_eq!(persistence::next_id(&reopened, "character"), SELLER_ID + 1);
    assert!(persistence::load_auctions(&reopened).is_empty());
    Ok(())
}

#[test]
#[ignore = "explicit owned DB/catalog/world/expiry env inputs required; writes 300k offers"]
fn produce_owned_300k_auction_fixture() -> FixtureResult<()> {
    assert_eq!(std::env::var("AH_FIXTURE_OWNER")?, OWNER);
    let path = required_absolute_path("AH_FIXTURE_DB")?;
    let catalog_path = required_absolute_path("AH_FIXTURE_CATALOG")?;
    let world_path = required_absolute_path("AH_FIXTURE_WORLD")?;
    let expiry: u64 = std::env::var("AH_FIXTURE_EXPIRY_START")?.parse()?;
    assert!(
        expiry > unix_now() + 3600,
        "expiry must be over an hour in the future"
    );
    expiry
        .checked_add(OFFER_COUNT - 1)
        .ok_or("expiry range overflows u64")?;
    let config: Config = serde_json::from_slice(&std::fs::read(&catalog_path)?)?;
    let templates = load_item_templates(&world_path)?;
    let item_ids = eligible_item_ids(&config, &templates)?;
    claim_database_path(&path)?;
    let db = Database::create(&path)?;
    persistence::create_tables(&db);
    reserve_fixture_ids(&db)?;
    write_fixture_offers(&db, &config, &templates, &item_ids, expiry)?;
    drop(db);
    assert_reopened_fixture(&path, &config, &templates, &item_ids, expiry)?;
    println!(
        "AH_FIXTURE owner={OWNER} db={} catalog={} world={} offers={OFFER_COUNT} distinct_items={} seller_id={SELLER_ID} expiry_start={expiry} expiry_end={} next_auction=300002 next_item_instance=300002 next_character=3",
        path.display(),
        catalog_path.display(),
        world_path.display(),
        item_ids.len(),
        expiry + OFFER_COUNT - 1
    );
    Ok(())
}

fn required_absolute_path(name: &str) -> FixtureResult<PathBuf> {
    let path = PathBuf::from(std::env::var(name)?);
    if !path.is_absolute() {
        return Err(format!("{name} must be an absolute path").into());
    }
    Ok(path)
}

fn claim_database_path(path: &Path) -> FixtureResult<()> {
    if path.file_name().is_some_and(|name| name == "game.redb") {
        return Err("fixture may never target game.redb".into());
    }
    // Atomic ownership claim: existing files and symlinks are never opened for writing.
    OpenOptions::new().write(true).create_new(true).open(path)?;
    Ok(())
}

fn eligible_item_ids(config: &Config, templates: &ItemTemplates) -> FixtureResult<Vec<u32>> {
    let mut eligible: BTreeSet<u32> = config.gatherables.iter().copied().collect();
    for recipe in config
        .merchants
        .iter()
        .flat_map(|merchant| &merchant.recipes)
    {
        eligible.insert(recipe.output_item_id);
        eligible.extend(recipe.ingredients.iter().map(|&(id, _)| id));
    }
    if eligible.len() as u64 > OFFER_COUNT / 51 {
        return Err("catalog has too many distinct items for a second offer page per item".into());
    }
    if eligible.len() <= 50 {
        return Err(
            "catalog needs over 50 real eligible items for distinct second-page proof".into(),
        );
    }
    for &id in &eligible {
        let market = config
            .markets
            .get(&id)
            .ok_or_else(|| format!("missing market {id}"))?;
        if market.item_id != id || !config.items.contains_key(&id) {
            return Err(format!("invalid catalog metadata for {id}").into());
        }
        let template = templates
            .get(shared::item_data::ItemDefinitionSource::Retail, id)
            .ok_or_else(|| format!("missing world template {id}"))?;
        if template.max_stack == 0 {
            return Err(format!("zero max_stack for {id}").into());
        }
        quote_price(market, 0)?;
    }
    Ok(eligible.into_iter().collect())
}

fn reserve_fixture_ids(db: &Database) -> FixtureResult<()> {
    let txn = db.begin_write()?;
    {
        let mut counters = txn.open_table(NEXT_IDS)?;
        counters.insert("auction", OFFER_COUNT)?;
        counters.insert("item_instance", OFFER_COUNT)?;
        counters.insert("character", SELLER_ID)?;
    }
    txn.commit()?;
    Ok(())
}

fn fixture_offer(
    config: &Config,
    templates: &ItemTemplates,
    item_ids: &[u32],
    id: u64,
    expiry: u64,
) -> FixtureResult<AuctionData> {
    let item_id = item_ids[((id - 1) % item_ids.len() as u64) as usize];
    let template = templates
        .get(shared::item_data::ItemDefinitionSource::Retail, item_id)
        .expect("validated template");
    let metadata = &config.items[&item_id];
    let market = &config.markets[&item_id];
    let draw = (id - 1) / item_ids.len() as u64;
    let unit_price = quote_price(market, draw)?;
    let quantity = 1 + (draw % u64::from(template.max_stack)) as u32;
    let total = unit_price
        .checked_mul(u64::from(quantity))
        .ok_or("stack price overflows u64")?;
    Ok(AuctionData {
        auction_id: id,
        seller_character_id: SELLER_ID,
        seller_name: "AH Load Fixture".into(),
        item: InventoryItemData {
            item_guid: id,
            item_id,
            name: metadata.name.clone(),
            quality: metadata.quality,
            required_level: metadata.required_level,
            stack_count: quantity,
            vendor_sell_price: metadata.vendor_sell_price,
            unique_auction: false,
        },
        stack_count: quantity,
        min_bid: total,
        current_bid: None,
        current_bidder_character_id: None,
        buyout_price: Some(total),
        expires_at_unix_secs: expiry + id - 1,
        faction: 0,
        deposit: 0,
    })
}

fn write_fixture_offers(
    db: &Database,
    config: &Config,
    templates: &ItemTemplates,
    item_ids: &[u32],
    expiry: u64,
) -> FixtureResult<()> {
    for start in (1..=OFFER_COUNT).step_by(BATCH_SIZE as usize) {
        let end = (start + BATCH_SIZE - 1).min(OFFER_COUNT);
        let auctions = (start..=end)
            .map(|id| fixture_offer(config, templates, item_ids, id, expiry))
            .collect::<FixtureResult<Vec<_>>>()?;
        commit_market(
            db,
            None,
            &MarketChanges {
                auctions,
                ..Default::default()
            },
        )?;
    }
    Ok(())
}

fn assert_reopened_fixture(
    path: &Path,
    config: &Config,
    templates: &ItemTemplates,
    item_ids: &[u32],
    expiry: u64,
) -> FixtureResult<()> {
    let db = Database::open(path)?;
    let rows = persistence::load_auctions(&db);
    assert_eq!(rows.len() as u64, OFFER_COUNT);
    let auctions: HashMap<_, _> = rows
        .into_iter()
        .map(|auction| (auction.auction_id, auction))
        .collect();
    assert_eq!(auctions.len() as u64, OFFER_COUNT);
    let distinct: BTreeSet<_> = auctions
        .values()
        .map(|auction| auction.item.item_id)
        .collect();
    assert_eq!(distinct, item_ids.iter().copied().collect());
    for id in 1..=OFFER_COUNT {
        assert_eq!(
            auctions[&id],
            fixture_offer(config, templates, item_ids, id, expiry)?
        );
    }
    assert_fixture_pages(&auctions, item_ids.len());
    {
        let txn = db.begin_read()?;
        let counters = txn.open_table(NEXT_IDS)?;
        assert_eq!(counters.get("auction")?.unwrap().value(), OFFER_COUNT);
        assert_eq!(counters.get("item_instance")?.unwrap().value(), OFFER_COUNT);
        assert_eq!(counters.get("character")?.unwrap().value(), SELLER_ID);
    }
    assert_eq!(persistence::next_id(&db, "auction"), OFFER_COUNT + 1);
    assert_eq!(persistence::next_id(&db, "item_instance"), OFFER_COUNT + 1);
    assert_eq!(persistence::next_id(&db, "character"), SELLER_ID + 1);
    Ok(())
}

fn assert_fixture_pages(auctions: &HashMap<u64, AuctionData>, distinct: usize) {
    let first_query = AuctionSearchQuery {
        page_size: 50,
        ..Default::default()
    };
    let second_query = AuctionSearchQuery {
        page: 1,
        ..first_query.clone()
    };
    let first = browse_for_viewer(auctions, &first_query, None, None, &|_| {
        shared::item_data::ItemDefinitionSource::Retail
    });
    let second = browse_for_viewer(auctions, &second_query, None, None, &|_| {
        shared::item_data::ItemDefinitionSource::Retail
    });
    assert_eq!(first.total_results as usize, distinct);
    assert_eq!(second.total_results as usize, distinct);
    assert_eq!(first.items.len(), 50);
    assert_eq!(second.items.len(), (distinct - 50).min(50));
    let first_ids: BTreeSet<_> = first.items.iter().map(|item| item.item_id).collect();
    assert!(
        second
            .items
            .iter()
            .all(|item| !first_ids.contains(&item.item_id))
    );
    let item_query = AuctionSearchQuery {
        item_id: Some(first.items[0].item_id),
        ..first_query
    };
    let item_second_query = AuctionSearchQuery {
        page: 1,
        ..item_query.clone()
    };
    let offers = search_auctions(auctions, &item_query, &|_| {
        shared::item_data::ItemDefinitionSource::Retail
    });
    let offers_second = search_auctions(auctions, &item_second_query, &|_| {
        shared::item_data::ItemDefinitionSource::Retail
    });
    assert!(offers.total_results > 50);
    assert_eq!(offers_second.total_results, offers.total_results);
    assert!(!offers_second.results.is_empty());
    let first_offers: BTreeSet<_> = offers
        .results
        .iter()
        .map(|offer| offer.auction_id)
        .collect();
    assert!(
        offers_second
            .results
            .iter()
            .all(|offer| !first_offers.contains(&offer.auction_id))
    );
    println!(
        "AH_FIXTURE_PAGES grouped_total={distinct} first={} second={} item_id={} offers_total={} offers_second={}",
        first.items.len(),
        second.items.len(),
        item_query.item_id.unwrap(),
        offers.total_results,
        offers_second.results.len()
    );
}
