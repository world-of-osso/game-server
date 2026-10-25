use shared::concentration::{ConcentrationCatalog, ConcentrationOriginGap};
use shared::currency::CurrencyWallet;
use std::path::Path;

#[test]
fn authored_leatherworking_currency_preserves_cap_and_millisecond_recharge() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/active");
    let catalog = ConcentrationCatalog::load(&dir).unwrap();
    let currency = catalog.for_skill(2830).unwrap();
    assert_eq!(currency.id, 3049);
    assert_eq!(currency.name, "Leatherworking Concentration");
    assert_eq!(currency.icon, 5747318);
    assert_eq!(currency.cap, 1000);
    assert_eq!(currency.recharge_amount, 1);
    assert_eq!(currency.recharge_cycle_ms, 345600);
    assert!(catalog.for_skill(2540).is_none());
}

#[test]
fn missing_wallet_entry_is_unknown_not_an_initial_zero_or_full_grant() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/active");
    let catalog = ConcentrationCatalog::load(&dir).unwrap();
    let wallet = CurrencyWallet::default();
    let snapshot = catalog.snapshot(2830, &wallet).unwrap();
    assert_eq!(snapshot.quantity, None);
    assert_eq!(snapshot.recharge_origin, Err(ConcentrationOriginGap));
    assert!(
        snapshot
            .recharge_origin
            .unwrap_err()
            .to_string()
            .starts_with("CRAFTING_CONCENTRATION_ORIGIN:")
    );
    assert!(wallet.entries.is_empty());
}

#[test]
fn existing_quantity_is_read_only_and_does_not_establish_recharge_origin() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/active");
    let catalog = ConcentrationCatalog::load(&dir).unwrap();
    let wallet = CurrencyWallet {
        entries: vec![shared::currency::CurrencyEntry {
            id: 3049,
            amount: 137,
            week_earned: 0,
        }],
    };
    let snapshot = catalog.snapshot(2830, &wallet).unwrap();
    assert_eq!(snapshot.quantity, Some(137));
    assert_eq!(snapshot.recharge_origin, Err(ConcentrationOriginGap));
    assert_eq!(wallet.get(3049), 137);
}

#[test]
fn concentration_query_requires_learned_line_and_never_creates_wallet_entries() {
    use super::*;
    use crate::currency::PlayerCurrencies;
    use shared::protocol::{ProfessionConcentrationRequest, ProfessionConcentrationUpdate};
    let mut h = harness_with(80, |app| {
        add_inbox::<ProfessionConcentrationUpdate>(app);
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/active");
        app.insert_resource(ConcentrationCatalog::load(&dir).unwrap());
    })
    .unwrap();
    h.app.world_mut().entity_mut(h.connection).insert((
        pair::<ProfessionConcentrationRequest>(),
        pair::<ProfessionConcentrationUpdate>(),
    ));
    h.app
        .world_mut()
        .entity_mut(h.player)
        .insert(PlayerCurrencies::default());
    h.send::<_, ProfessionChannel>(ProfessionConcentrationRequest { skill_line: 2830 });
    let rejected = h
        .take::<ProfessionConcentrationUpdate>()
        .pop()
        .expect("unlearned-line reply");
    assert_eq!(
        rejected.error.as_deref(),
        Some("Profession skill line is not learned")
    );
    assert!(rejected.snapshot.is_none());
    let mut professions = h.professions();
    professions
        .lines
        .push(shared::profession::ProfessionSkillLine {
            skill_line: 2830,
            step: 1,
            rank: 25,
            max_rank: 100,
        });
    h.app.world_mut().entity_mut(h.player).insert(professions);
    h.send::<_, ProfessionChannel>(ProfessionConcentrationRequest { skill_line: 2830 });
    let update = h
        .take::<ProfessionConcentrationUpdate>()
        .pop()
        .expect("learned-line reply");
    assert_eq!(update.error, None);
    let snapshot = update.snapshot.unwrap();
    assert_eq!(snapshot.currency.cap, 1000);
    assert_eq!(snapshot.currency.recharge_cycle_ms, 345600);
    assert_eq!(snapshot.quantity, None);
    assert_eq!(snapshot.recharge_origin, Err(ConcentrationOriginGap));
    assert!(
        h.app
            .world()
            .get::<PlayerCurrencies>(h.player)
            .unwrap()
            .0
            .entries
            .is_empty()
    );
    assert!(
        persistence::load_currencies(h.db(), h.character_id)
            .entries
            .is_empty()
    );
}
