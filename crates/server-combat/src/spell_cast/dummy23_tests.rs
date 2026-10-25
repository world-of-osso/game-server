//! Native Shadow Blades combo modifiers; its damage Dummy remains separate.
use super::test_harness::*;
use shared::components::PowerType;
use shared::protocol::CombatLogKind;

fn rune_weapon() -> Harness {
    let ids = [49028, 81256];
    let mut h = harness(Vec::new(), &ids, 6).unwrap();
    h.app.insert_resource(crate::world_data::load_world_data(
        &world_db().unwrap().powers_db,
    ));
    let source = rusqlite::Connection::open_with_flags(
        &world_db().unwrap().powers_db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(super::summon::load_summon_properties(&source).unwrap());
    h
}

fn cast_rune_weapon(h: &mut Harness) {
    let caster = h.caster;
    h.cast_and_deliver(49028, Some(caster));
    h.advance(4);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.inbox.go.iter().any(|event| event.spell_id == 49028));
}

#[test]
fn dummy23_rune_weapon_completed_root_grants_owner_native_parry() {
    let mut h = rune_weapon();
    let ally = spawn_player(&mut h.app, 6, 3.0);
    cast_rune_weapon(&mut h);
    let world = h.app.world_mut();
    let summons: Vec<_> = world
        .query::<&super::summon::SummonedCreature>()
        .iter(world)
        .filter(|s| s.spell_id == 49028)
        .collect();
    assert_eq!(summons.len(), 1);
    assert_eq!(summons[0].owner, h.caster);
    assert!(crate::auras::has_aura(h.app.world(), h.caster, 81256));
    assert_eq!(
        h.app
            .world()
            .get::<crate::stats::StatModifiers>(h.caster)
            .unwrap()
            .parry_pct,
        30.0
    );
    assert!(!crate::auras::has_aura(h.app.world(), ally, 81256));
}

#[test]
fn dummy23_rune_weapon_native_lifetime_ends_summon_and_owner_parry() {
    let mut h = rune_weapon();
    cast_rune_weapon(&mut h);
    assert!(crate::auras::has_aura(h.app.world(), h.caster, 81256));
    h.advance(170);
    assert!(!crate::auras::has_aura(h.app.world(), h.caster, 81256));
    assert_eq!(
        h.app
            .world()
            .get::<crate::stats::StatModifiers>(h.caster)
            .unwrap()
            .parry_pct,
        0.0
    );
    let world = h.app.world_mut();
    assert_eq!(
        world
            .query::<&super::summon::SummonedCreature>()
            .iter(world)
            .filter(|s| s.spell_id == 49028)
            .count(),
        0
    );
}

#[test]
fn dummy23_rune_weapon_missing_real_template_spawns_nothing_without_parry() {
    let mut h = rune_weapon();
    h.app
        .world_mut()
        .resource_mut::<crate::world_data::WorldData>()
        .templates
        .remove(&27893);
    let caster = h.caster;
    h.cast_and_deliver(49028, Some(caster));
    h.advance(4);
    assert!(!crate::auras::has_aura(h.app.world(), caster, 81256));
    let world = h.app.world_mut();
    assert_eq!(
        world
            .query::<&super::summon::SummonedCreature>()
            .iter(world)
            .count(),
        0
    );
}

#[test]
fn dummy23_rune_weapon_early_despawn_removes_owner_parry() {
    let mut h = rune_weapon();
    cast_rune_weapon(&mut h);
    assert!(crate::auras::has_aura(h.app.world(), h.caster, 81256));
    let world = h.app.world_mut();
    let actor = world.query_filtered::<bevy::prelude::Entity, bevy::prelude::With<super::summon::SummonedCreature>>().single(world).unwrap();
    super::summon::despawn_summon(world, actor);
    h.advance(4);
    assert!(!crate::auras::has_aura(h.app.world(), h.caster, 81256));
    assert_eq!(
        h.app
            .world()
            .get::<crate::stats::StatModifiers>(h.caster)
            .unwrap()
            .parry_pct,
        0.0
    );
}

fn fixture() -> Harness {
    let ids = [121471, 53, 185438, 197835];
    let spells = ids
        .iter()
        .map(|id| {
            let mut spell = world_db().unwrap().store.get(*id).unwrap().clone();
            for effect in &mut spell.effects {
                effect.variance = 0.0;
            }
            spell
        })
        .collect();
    let mut h = harness(spells, &ids, 4).unwrap();
    h.set_power(PowerType::Energy, 100.0);
    h.set_power(PowerType::ComboPoints, 0.0);
    h
}

fn blades(h: &mut Harness) {
    let caster = h.caster;
    h.cast_and_deliver(121471, Some(caster));
    h.advance(35);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(crate::auras::has_aura(h.app.world(), caster, 121471));
    h.inbox.log.clear();
}

fn builder(h: &mut Harness, id: u32) {
    let npc = h.npc;
    h.set_power(PowerType::Energy, 100.0);
    h.cast_and_deliver(id, Some(npc));
    h.advance(4);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.inbox.go.iter().any(|event| event.spell_id == id));
}

#[test]
fn dummy23_shadow_blades_native_backstab_grants_double_combo_points() {
    let mut h = fixture();
    builder(&mut h, 53);
    assert_eq!(h.power(PowerType::ComboPoints), 1.0);
    h.advance(35);
    h.set_power(PowerType::ComboPoints, 0.0);
    blades(&mut h);
    builder(&mut h, 53);
    assert_eq!(h.power(PowerType::ComboPoints), 2.0);
    let grants: Vec<_> = h
        .inbox
        .log
        .iter()
        .filter(|e| e.spell_id == Some(53) && e.kind == CombatLogKind::Energize)
        .map(|e| (e.amount, e.overflow))
        .collect();
    assert_eq!(grants, vec![(2, 0)]);
}

#[test]
fn dummy23_shadow_blades_native_backstab_cap_keeps_overflow() {
    let mut h = fixture();
    blades(&mut h);
    h.set_power(PowerType::ComboPoints, 4.0);
    builder(&mut h, 53);
    assert_eq!(h.power(PowerType::ComboPoints), 5.0);
    let grants: Vec<_> = h
        .inbox
        .log
        .iter()
        .filter(|e| e.spell_id == Some(53) && e.kind == CombatLogKind::Energize)
        .map(|e| (e.amount, e.overflow))
        .collect();
    assert_eq!(grants, vec![(2, 1)]);
}

#[test]
fn dummy23_shadow_blades_native_expiry_restores_backstab() {
    let mut h = fixture();
    blades(&mut h);
    h.advance(500);
    assert!(!crate::auras::has_aura(h.app.world(), h.caster, 121471));
    builder(&mut h, 53);
    assert_eq!(h.power(PowerType::ComboPoints), 1.0);
}

#[test]
fn dummy23_shadow_blades_native_storm_doubles_each_enemy_not_once() {
    let mut h = fixture();
    spawn_npc(&mut h.app, 3.0);
    blades(&mut h);
    builder(&mut h, 197835);
    assert_eq!(h.power(PowerType::ComboPoints), 4.0);
}

#[test]
fn dummy23_shadow_blades_storm_caps_the_native_pool() {
    let mut h = fixture();
    spawn_npc(&mut h.app, 3.0);
    blades(&mut h);
    h.set_power(PowerType::ComboPoints, 4.0);
    builder(&mut h, 197835);
    assert_eq!(h.power(PowerType::ComboPoints), 5.0);
}

#[test]
fn dummy23_storm_without_blades_retains_one_point_per_enemy() {
    let mut h = fixture();
    spawn_npc(&mut h.app, 3.0);
    builder(&mut h, 197835);
    assert_eq!(h.power(PowerType::ComboPoints), 2.0);
}

#[test]
fn dummy23_shadow_blades_expiry_restores_scripted_storm() {
    let mut h = fixture();
    spawn_npc(&mut h.app, 3.0);
    blades(&mut h);
    h.advance(500);
    assert!(!crate::auras::has_aura(h.app.world(), h.caster, 121471));
    builder(&mut h, 197835);
    assert_eq!(h.power(PowerType::ComboPoints), 2.0);
}

#[test]
fn dummy23_shadow_blades_rejected_friendly_backstab_grants_nothing() {
    let mut h = fixture();
    blades(&mut h);
    let ally = spawn_player(&mut h.app, 4, 2.0);
    h.cast_and_deliver(53, Some(ally));
    h.advance(4);
    assert!(!h.inbox.failed.is_empty());
    assert_eq!(h.power(PowerType::ComboPoints), 0.0);
}
