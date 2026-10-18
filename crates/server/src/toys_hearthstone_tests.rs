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

#[test]
fn toyfx165_holographic_hearthstone_learned_use_returns_to_bind_and_blocks_brewfest() {
    assert_toy_hearthstone_bind_and_shared_charge(168907, 298068, 166747);
}

#[test]
fn toyfx190_eternal_traveler_hearthstone_learned_use_returns_to_bind_and_blocks_holographic() {
    assert_toy_hearthstone_bind_and_shared_charge(172179, 308742, 168907);
}

#[test]
fn toyfx225_necrolord_hearthstone_learned_use_returns_to_bind_and_blocks_eternal_traveler() {
    assert_toy_hearthstone_bind_and_shared_charge(182773, 340200, 172179);
}

#[test]
fn toyfx311_dominated_hearthstone_learned_use_returns_to_bind_and_blocks_necrolord() {
    assert_toy_hearthstone_bind_and_shared_charge(188952, 363799, 182773);
}

#[test]
fn toyfx313_enlightened_hearthstone_learned_use_returns_to_bind_and_blocks_dominated() {
    assert_toy_hearthstone_bind_and_shared_charge(190196, 366945, 188952);
}

#[test]
fn toyfx314_broker_matrix_learned_use_returns_to_bind_and_blocks_enlightened() {
    assert_toy_hearthstone_bind_and_shared_charge(190237, 367013, 190196);
}

#[test]
fn toyfx333_timewalker_hearthstone_learned_use_returns_to_bind_and_blocks_broker_matrix() {
    assert_toy_hearthstone_bind_and_shared_charge(193588, 375357, 190237);
}

#[test]
fn toyfx393_ohnir_windsage_hearthstone_learned_use_returns_to_bind_and_blocks_timewalker() {
    assert_toy_hearthstone_bind_and_shared_charge(200630, 391042, 193588);
}

#[test]
fn toyfx457_path_of_naaru_learned_use_returns_to_bind_and_blocks_ohnir_windsage() {
    assert_toy_hearthstone_bind_and_shared_charge(206195, 412555, 200630);
}

#[test]
fn toyfx477_deepdweller_hearthstone_learned_use_returns_to_bind_and_blocks_path_of_naaru() {
    assert_toy_hearthstone_bind_and_shared_charge(208704, 420418, 206195);
}

#[test]
fn toyfx482_flame_hearthstone_learned_use_returns_to_bind_and_blocks_deepdweller() {
    assert_toy_hearthstone_bind_and_shared_charge(209035, 422284, 208704);
}

#[test]
fn toyfx503_stone_of_hearth_learned_use_returns_to_bind_and_blocks_flame() {
    assert_toy_hearthstone_bind_and_shared_charge(212337, 401802, 209035);
}

#[test]
fn toyfx621_post_express_hearthstone_learned_use_returns_to_bind_and_blocks_stone() {
    assert_toy_hearthstone_bind_and_shared_charge(245970, 1240219, 212337);
}

#[test]
fn toyfx623_cosmic_hearthstone_learned_use_returns_to_bind_and_blocks_post_express() {
    assert_toy_hearthstone_bind_and_shared_charge(246565, 1242509, 245970);
}

#[test]
fn toyfx645_lightcalled_hearthstone_learned_use_returns_to_bind_and_blocks_cosmic() {
    assert_toy_hearthstone_bind_and_shared_charge(257736, 1261979, 246565);
}

#[test]
fn toyfx659_naaru_enfold_learned_use_returns_to_bind_and_blocks_lightcalled() {
    assert_toy_hearthstone_bind_and_shared_charge(263489, 1270583, 257736);
}

#[test]
fn toyfx661_preyseeker_hearthstone_learned_use_returns_to_bind_and_blocks_naaru() {
    assert_toy_hearthstone_bind_and_shared_charge(263933, 1270814, 263489);
}

#[test]
fn toyfx673_corewarden_hearthstone_learned_use_returns_to_bind_and_blocks_preyseeker() {
    assert_toy_hearthstone_bind_and_shared_charge(265100, 1273401, 263933);
}

#[test]
fn toyfx664_mycomancer_learned_root_triggers_homebind_after_cast_and_blocks_corewarden() {
    use shared::components::{Health, ModelDisplay};
    const ITEM: u32 = 264367;
    const ROOT: u32 = 1299014;
    const SIBLING: u32 = 265100;
    const CHARGE_CATEGORY: u32 = 2309;
    const RECHARGE_MS: u32 = 1_800_000;
    let directory = tempfile::tempdir().unwrap();
    let (mut h, character) = setup(&directory.path().join("mycomancer.redb"));
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
    .expect("character creation persists the home bind used by child1286909");
    let caster = h.caster;
    let health = *h.app.world().get::<Health>(caster).unwrap();
    let display = h.app.world().get::<ModelDisplay>(caster).copied();
    h.app.world_mut().entity_mut(caster).insert(bind);
    let mut origin = bind.0.bevy_position();
    origin.x += 30.0;
    *h.app.world_mut().get_mut::<Position>(caster).unwrap() = origin;
    learn_effect_toy(&mut h, character, ITEM);
    learn_effect_toy(&mut h, character, SIBLING);

    // The learned root casts for10s; its64 child performs252, not a substituted root.
    assert_eq!(activate(&mut h, ITEM)[0].error, None);
    h.advance(100);
    assert_eq!(h.app.world().get::<Position>(caster), Some(&origin));
    assert!(!h.inbox.go.iter().any(|go| go.spell_id == ROOT));
    h.advance(110);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(
        h.inbox.go.iter().filter(|go| go.spell_id == ROOT).count(),
        1
    );
    assert_eq!(
        h.app.world().get::<Position>(caster),
        Some(&bind.0.bevy_position())
    );
    let charge = h
        .inbox
        .charges
        .iter()
        .find(|charge| charge.spell_id == ROOT && charge.category == CHARGE_CATEGORY)
        .expect("the learned root publishes its shared hearthstone charge");
    assert_eq!(charge.current, 0);
    assert_eq!(charge.max, 1);
    assert_eq!(charge.recharge_ms, RECHARGE_MS);
    let cast_count = h.inbox.go.len();
    assert!(activate(&mut h, SIBLING)[0].error.is_some());
    h.advance(220);
    assert_eq!(h.inbox.go.len(), cast_count);
    assert_eq!(
        h.app.world().get::<Position>(caster),
        Some(&bind.0.bevy_position())
    );
    assert_eq!(h.app.world().get::<Health>(caster), Some(&health));
    assert_eq!(h.app.world().get::<ModelDisplay>(caster).copied(), display);
}
