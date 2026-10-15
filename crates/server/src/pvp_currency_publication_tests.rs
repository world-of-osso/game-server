use super::*;
use shared::currency::{CurrencyWallet, HONOR_CURRENCY_ID, currency_by_id};
use shared::protocol::{CurrencyChannel, CurrencyStateUpdate, SpendCurrency};
use std::time::{Duration, Instant};

#[derive(Clone, Copy)]
enum Preference {
    Talent,
    WarMode,
}

#[test]
fn pvp_talent_receipt_does_not_restore_honor_older_than_a_later_currency_publication() {
    assert_later_currency_publication(Preference::Talent);
}

#[test]
fn war_mode_receipt_does_not_restore_honor_older_than_a_later_currency_publication() {
    assert_later_currency_publication(Preference::WarMode);
}

fn assert_later_currency_publication(preference: Preference) {
    let (mut h, character) = selection_harness(SelectionCase::Fresh);
    let mut wallet = CurrencyWallet::default();
    wallet
        .add(
            HONOR_CURRENCY_ID,
            1000,
            currency_by_id(HONOR_CURRENCY_ID).unwrap(),
        )
        .unwrap();
    let db = &h.app.world().resource::<GameDatabase>().db;
    persistence::save_currencies(db, character, &wallet);
    let mut progress = persistence::load_pvp(db, character);
    progress.honor = 1000;
    persistence::save_pvp(db, character, &progress);
    h.app
        .world_mut()
        .get_mut::<PlayerPvpState>(h.caster)
        .unwrap()
        .progress = progress;
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::currency::PlayerCurrencies(wallet));
    h.app.world_mut().entity_mut(h.connection).insert((
        MessageSender::<SpendCurrency>::default(),
        MessageReceiver::<SpendCurrency>::default(),
        MessageSender::<CurrencyStateUpdate>::default(),
        MessageReceiver::<CurrencyStateUpdate>::default(),
    ));
    Plugin::build(&crate::currency::CurrencyPlugin, &mut h.app);
    h.advance(6);
    h.app
        .world_mut()
        .get_mut::<MessageReceiver<PvpStateUpdate>>(h.connection)
        .unwrap()
        .receive()
        .for_each(drop);
    let held = h
        .app
        .world()
        .resource::<GameDatabase>()
        .db
        .begin_write()
        .unwrap();
    let release = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(250));
        held.abort().unwrap();
    });
    match preference {
        Preference::Talent => send_selection(&mut h, 752),
        Preference::WarMode => {
            h.app
                .world_mut()
                .get_mut::<MessageSender<SetWarModeDesired>>(h.connection)
                .unwrap()
                .send::<PvpChannel>(SetWarModeDesired { enabled: true });
        }
    }
    let started = Instant::now();
    h.advance(3);
    h.app
        .world_mut()
        .get_mut::<MessageSender<SpendCurrency>>(h.connection)
        .unwrap()
        .send::<CurrencyChannel>(SpendCurrency {
            currency_id: HONOR_CURRENCY_ID,
            amount: 223,
        });
    h.advance(3);
    let elapsed = started.elapsed();
    let fence = h
        .app
        .world()
        .resource::<GameDatabase>()
        .writer
        .enqueue("currency/PvP publication fence", |_| Ok(()));
    release.join().unwrap();
    assert!(
        elapsed < Duration::from_millis(50),
        "preference/currency ticks waited: {elapsed:?}"
    );
    // Both durable jobs must finish before the next frame: production Currency Update
    // then PvP PostUpdate exercise the ordering boundary, not a convenient poll order.
    let deadline = Instant::now() + Duration::from_secs(3);
    while fence.result().is_none() {
        assert!(
            Instant::now() < deadline,
            "currency/PvP FIFO did not finish"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
    h.advance(6);
    assert_eq!(
        h.app
            .world()
            .get::<crate::currency::PlayerCurrencies>(h.caster)
            .unwrap()
            .0
            .get(HONOR_CURRENCY_ID),
        777
    );
    let durable = persistence::load_pvp(&h.app.world().resource::<GameDatabase>().db, character);
    assert_eq!(durable.honor, 777);
    match preference {
        Preference::Talent => assert_eq!(durable.pvp_talents[&70][&1], 752),
        Preference::WarMode => assert!(durable.war_mode_desired),
    }
    let live = h.app.world().get::<PlayerPvpState>(h.caster).unwrap();
    assert_eq!(
        live.progress.honor, 777,
        "older PvP receipt regressed the later currency publication"
    );
    match preference {
        Preference::Talent => assert_eq!(live.progress.pvp_talents[&70][&1], 752),
        Preference::WarMode => assert!(live.progress.war_mode_desired),
    }
    let replies: Vec<_> = h
        .app
        .world_mut()
        .get_mut::<MessageReceiver<PvpStateUpdate>>(h.connection)
        .unwrap()
        .receive()
        .collect();
    assert_eq!(
        replies.last().unwrap().snapshot.as_ref().unwrap().honor,
        777
    );
}
