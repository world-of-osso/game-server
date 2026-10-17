//! Real learned toy hearthstones: persisted bind, authored cast and shared charges.
use super::*;

fn assert_toy_hearthstone_bind_and_shared_charge(item: u32, spell: u32, sibling: u32) {
    use shared::components::{Health, ModelDisplay};
    const HEARTH_CHARGE_CATEGORY: u32 = 2309;
    const CHARGE_RECOVERY_MS: u32 = 1_800_000;
    let directory = tempfile::tempdir().unwrap();
    let (mut h, character) = setup(&directory.path().join("hearth.redb"));
    let conn = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(crate::spell_info::load_spell_data(&conn).unwrap().0);
    let bind = crate::spell_cast::homebind::load_bind(
        &h.app.world().resource::<GameDatabase>().db,
        character,
    )
    .expect("character creation persists its real home bind");
    let caster = h.caster;
    let health = *h.app.world().get::<Health>(caster).unwrap();
    let display = h.app.world().get::<ModelDisplay>(caster).copied();
    learn_effect_toy(&mut h, character, item);
    learn_effect_toy(&mut h, character, sibling);

    // The headless fixture has no login hydration: absent bind must refuse,
    // not spend the shared charge or fabricate a race/logout destination.
    h.app
        .world_mut()
        .entity_mut(caster)
        .remove::<crate::spell_cast::homebind::HomeBind>();
    assert!(activate(&mut h, item)[0].error.is_some());
    assert!(!h.inbox.go.iter().any(|go| go.spell_id == spell));
    assert!(h.inbox.charges.is_empty());
    h.inbox.failed.clear();
    h.app.world_mut().entity_mut(caster).insert(bind);
    let mut origin = bind.0.bevy_position();
    origin.x += 30.0;
    *h.app.world_mut().get_mut::<Position>(caster).unwrap() = origin;

    assert_eq!(activate(&mut h, item)[0].error, None);
    h.advance(100); // 5s plus activation frames: source10s cast still pending.
    assert_eq!(h.app.world().get::<Position>(caster), Some(&origin));
    assert!(!h.inbox.go.iter().any(|go| go.spell_id == spell));
    h.advance(110);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.inbox.go.iter().any(|go| go.spell_id == spell));
    assert_eq!(
        h.app.world().get::<Position>(caster),
        Some(&bind.0.bevy_position())
    );
    let spent_charge = h
        .inbox
        .charges
        .iter()
        .find(|charge| charge.spell_id == spell && charge.category == HEARTH_CHARGE_CATEGORY)
        .expect("successful hearthstone publishes its shared charge");
    assert_eq!(spent_charge.current, 0);
    assert_eq!(spent_charge.max, 1);
    assert_eq!(spent_charge.recharge_ms, CHARGE_RECOVERY_MS);
    let cast_count = h.inbox.go.len();
    assert!(activate(&mut h, sibling)[0].error.is_some());
    h.advance(220);
    assert_eq!(
        h.inbox.go.len(),
        cast_count,
        "shared charge blocks sibling toy"
    );
    assert_eq!(
        h.app.world().get::<Position>(caster),
        Some(&bind.0.bevy_position())
    );
    assert_eq!(h.app.world().get::<Health>(caster), Some(&health));
    assert_eq!(h.app.world().get::<ModelDisplay>(caster).copied(), display);
}

#[test]
fn toyfx78_greatfather_hearthstone_learned_use_returns_to_bind_and_spends_shared_charge() {
    assert_toy_hearthstone_bind_and_shared_charge(162973, 278244, 163045);
}

#[test]
fn toyfx79_headless_hearthstone_learned_use_returns_to_bind_and_spends_shared_charge() {
    assert_toy_hearthstone_bind_and_shared_charge(163045, 278559, 162973);
}

#[test]
fn toyfx116_lunar_elder_hearthstone_learned_use_returns_to_bind_and_blocks_peddlefeet() {
    assert_toy_hearthstone_bind_and_shared_charge(165669, 285362, 165670);
}

#[test]
fn toyfx117_peddlefeet_hearthstone_learned_use_returns_to_bind_and_blocks_lunar_elder() {
    assert_toy_hearthstone_bind_and_shared_charge(165670, 285424, 165669);
}

#[test]
fn toyfx125_noble_gardener_hearthstone_learned_use_returns_to_bind_and_blocks_lunar_elder() {
    assert_toy_hearthstone_bind_and_shared_charge(165802, 286031, 165669);
}

#[test]
fn toyfx139_fire_eater_hearthstone_learned_use_returns_to_bind_and_blocks_noble_gardener() {
    assert_toy_hearthstone_bind_and_shared_charge(166746, 286331, 165802);
}

#[test]
fn toyfx140_brewfest_reveler_hearthstone_learned_use_returns_to_bind_and_blocks_fire_eater() {
    assert_toy_hearthstone_bind_and_shared_charge(166747, 286353, 166746);
}
