//! Approved player-spec bridge: real kicks cancel actual enemy casts.
use super::profile_behavior_tests::spawn_profile_fixture;
use super::update_ai;
use shared::casting::CastState;
use shared::components::Position;
use shared::protocol::CombatLogKind;

pub(super) fn start_enemy_fireball(
    h: &mut crate::spell_cast::test_harness::Harness,
    unit: bevy::prelude::Entity,
) {
    let enemy = h.npc;
    prepare_enemy_spec(h, enemy, 63);
    let at = *h.app.world().get::<Position>(enemy).unwrap();
    h.app.world_mut().entity_mut(unit).insert(Position {
        x: at.x - 2.0,
        ..at
    });
    crate::spell_cast::creature_cast(h.app.world_mut(), enemy, 133, Some(unit), false).unwrap();
    assert!(h.app.world().get::<CastState>(enemy).is_some());
}

pub(super) fn prepare_enemy_spec(
    h: &mut crate::spell_cast::test_harness::Harness,
    enemy: bevy::prelude::Entity,
    spec: u32,
) {
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let mage = super::catalog::load_catalog(&db)
        .unwrap()
        .profiles
        .into_iter()
        .find(|profile| profile.spec_id == spec)
        .unwrap();
    let (stats, pools, passives) = super::spawning::class_stats(h.app.world(), &mage, 12).unwrap();
    let mana = shared::components::Mana {
        current: stats.base_mana,
        max: stats.base_mana,
    };
    h.app
        .world_mut()
        .entity_mut(enemy)
        .insert((stats, pools, mana, shared::components::Level(12)));
    crate::auras::sync_passives(
        h.app.world_mut(),
        enemy,
        crate::auras::AuraSource::KnownSpell,
        &passives,
    );
}

pub(super) fn protect_enemy_cast_with_native_interrupt_immunity(
    h: &mut crate::spell_cast::test_harness::Harness,
    target: bevy::prelude::Entity,
) {
    const UNENDING_RESOLVE: u32 = 104773;
    let spell = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(UNENDING_RESOLVE)
        .unwrap();
    assert!(
        spell
            .effects
            .iter()
            .any(|effect| effect.aura == 77 && effect.misc_values[0] == 26)
    );
    let effect_mask = crate::auras::aura_effect_mask(spell);
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: target,
            target,
            spell_id: UNENDING_RESOLVE,
            effect_mask,
            source: crate::auras::AuraSource::Cast,
        },
    );
    assert!(crate::auras::has_aura(
        h.app.world(),
        target,
        UNENDING_RESOLVE
    ));
}

fn assert_follower_interrupt(entry: u32, interrupt: u32) {
    let (mut h, unit, _) = spawn_profile_fixture(entry);
    start_enemy_fireball(&mut h, unit);
    update_ai(h.app.world_mut());
    h.advance(3);
    assert!(
        h.app.world().get::<CastState>(h.npc).is_none(),
        "enemy cast must be interrupted"
    );
    assert!(
        h.inbox.log.iter().any(|event| {
            event.kind == CombatLogKind::Interrupt
                && event.source == Some(unit.to_bits())
                && event.target == Some(h.npc.to_bits())
                && event.spell_id == Some(interrupt)
                && event.extra_spell_id == Some(133)
        }),
        "real interrupt combat log missing: {:?}",
        h.inbox.log
    );
}

macro_rules! interrupt_case {
    ($test:ident, $entry:literal, $interrupt:literal) => {
        #[test]
        fn $test() {
            assert_follower_interrupt($entry, $interrupt);
        }
    };
}
interrupt_case!(follower_garrick_rebuke_interrupts, 209057, 96231);
interrupt_case!(follower_breka_pummel_interrupts, 215517, 6552);
interrupt_case!(follower_meredy_counterspell_interrupts, 209059, 2139);
interrupt_case!(follower_shuja_wind_shear_interrupts, 209053, 57994);
interrupt_case!(follower_austin_counter_shot_interrupts, 209065, 147362);
interrupt_case!(follower_kybel_quell_interrupts, 215240, 351338);
interrupt_case!(follower_bo_spear_hand_strike_interrupts, 227734, 116705);
interrupt_case!(follower_ralia_skull_bash_interrupts, 232800, 93985);

#[test]
fn follower_meredy_stops_her_cast_for_a_ready_counterspell() {
    let (mut h, unit, _) = spawn_profile_fixture(209059);
    crate::spell_cast::creature_cast(h.app.world_mut(), unit, 133, Some(h.npc), false).unwrap();
    start_enemy_fireball(&mut h, unit);
    update_ai(h.app.world_mut());
    h.advance(3);
    assert!(h.app.world().get::<CastState>(h.npc).is_none());
    assert!(h.app.world().get::<CastState>(unit).is_none());
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.kind == CombatLogKind::Interrupt && event.spell_id == Some(2139))
    );
}

#[test]
fn follower_counterspell_cooldown_preserves_damage_priority() {
    let (mut h, unit, _) = spawn_profile_fixture(209059);
    start_enemy_fireball(&mut h, unit);
    update_ai(h.app.world_mut());
    h.advance(160);
    start_enemy_fireball(&mut h, unit);
    update_ai(h.app.world_mut());
    h.advance(3);
    assert!(h.app.world().get::<CastState>(h.npc).is_some());
    assert_eq!(
        h.inbox
            .go
            .iter()
            .filter(|cast| cast.caster == unit.to_bits() && cast.spell_id == 2139)
            .count(),
        1
    );
    assert!(
        h.inbox
            .go
            .iter()
            .any(|cast| cast.caster == unit.to_bits() && cast.spell_id == 108853)
    );
}

#[test]
fn follower_unready_interrupt_does_not_cancel_her_current_cast() {
    let (mut h, unit, _) = spawn_profile_fixture(209059);
    start_enemy_fireball(&mut h, unit);
    update_ai(h.app.world_mut());
    h.advance(160);
    crate::spell_cast::creature_cast(h.app.world_mut(), unit, 133, Some(h.npc), false).unwrap();
    start_enemy_fireball(&mut h, unit);
    update_ai(h.app.world_mut());
    h.advance(3);
    assert_eq!(h.app.world().get::<CastState>(unit).unwrap().spell_id, 133);
    assert!(h.app.world().get::<CastState>(h.npc).is_some());
    assert_eq!(
        h.inbox
            .go
            .iter()
            .filter(|cast| cast.caster == unit.to_bits() && cast.spell_id == 2139)
            .count(),
        1
    );
}

#[test]
fn follower_does_not_waste_counterspell_on_non_interruptible_cast() {
    let (mut h, unit, _) = spawn_profile_fixture(209059);
    start_enemy_fireball(&mut h, unit);
    let enemy = h.npc;
    protect_enemy_cast_with_native_interrupt_immunity(&mut h, enemy);
    update_ai(h.app.world_mut());
    h.advance(3);
    assert!(h.app.world().get::<CastState>(h.npc).is_some());
    assert!(
        !h.inbox
            .go
            .iter()
            .any(|cast| cast.caster == unit.to_bits() && cast.spell_id == 2139)
    );
}
