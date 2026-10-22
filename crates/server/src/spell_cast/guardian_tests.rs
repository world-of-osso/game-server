//! Authentic Grimoire Imp / archive Firebolt binding, not a Wild Imp substitution.
use super::test_harness::*;
use bevy::prelude::*;
use shared::components::Health;

fn imp(h: &mut Harness) -> Entity {
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
    // TDB1210 creature_template_spell: (416,0,3110,41031),(416,2,3110,44325).
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    conn.execute_batch("CREATE TABLE guardian_spells (CreatureID INTEGER, SpellIndex INTEGER, Spell INTEGER, VerifiedBuild INTEGER); INSERT INTO guardian_spells VALUES (416,0,3110,41031),(416,2,3110,44325);").unwrap();
    h.app
        .insert_resource(super::guardian::load_guardian_spells(&conn).unwrap());
    h.select(Some(h.npc));
    h.advance(4);
    // Root Dummy0 remains unbound; exercise its authentic effect28 directly.
    let spell = world_db().unwrap().store.get(111859).unwrap();
    let effect = spell
        .effects
        .iter()
        .find(|effect| effect.effect == 28)
        .unwrap();
    let dest =
        super::targets::effect_dest(h.app.world(), h.caster, Some(h.npc), effect, None).unwrap();
    let ctx = super::effects::EffectCtx {
        caster: h.caster,
        spell,
        depth: 0,
        dest: Some(dest),
        chain_multiplier: 1.0,
    };
    super::summon::summon_creature(h.app.world_mut(), &ctx, effect);
    let world = h.app.world_mut();
    world
        .query_filtered::<Entity, With<super::summon::SummonedCreature>>()
        .single(world)
        .unwrap()
}

#[test]
fn guardians_imp_casts_with_native_time_retargets_and_expires() {
    let mut h = harness(Vec::new(), &[111859, 3110], 9).unwrap();
    let guardian = imp(&mut h);
    h.advance(10);
    assert_eq!(
        h.health(h.npc),
        5000.0,
        "1750ms Firebolt cannot resolve immediately"
    );
    h.advance(35);
    assert!(
        h.health(h.npc) < 5000.0,
        "guardian must cast its archive spell"
    );
    let other = spawn_npc(&mut h.app, 3.0);
    h.select(Some(other));
    h.advance(85);
    assert!(
        h.health(other) < 5000.0,
        "next cast inherits changed selection"
    );
    h.advance(600);
    assert!(h.app.world().get_entity(guardian).is_err());
}

#[test]
fn guardians_imp_does_not_attack_with_no_target_or_after_owner_death() {
    let mut h = harness(Vec::new(), &[111859, 3110], 9).unwrap();
    let guardian = imp(&mut h);
    h.select(None);
    h.advance(60);
    let before = h.health(h.npc);
    h.advance(60);
    assert_eq!(h.health(h.npc), before);
    h.app
        .world_mut()
        .get_mut::<Health>(h.caster)
        .unwrap()
        .current = 0.0;
    h.advance(2);
    assert!(h.app.world().get_entity(guardian).is_err());
}

fn sacrifice() -> (Harness, Entity) {
    let mut h = harness(Vec::new(), &[6940, 108853, 17, 642], 2).unwrap();
    let ally = spawn_player(&mut h.app, 2, 0.0);
    super::cast_triggered(h.app.world_mut(), h.caster, 6940, Some(ally), 0);
    h.app
        .world_mut()
        .entity_mut(h.npc)
        .insert(crate::stats::CharacterStats {
            spell_power: 100.0,
            ..default()
        });
    (h, ally)
}

#[test]
fn guardians_sacrifice_splits_spell_damage_and_stops_on_native_health_floor() {
    let (mut h, ally) = sacrifice();
    let before = h.health(ally);
    super::cast_triggered(h.app.world_mut(), h.npc, 108853, Some(ally), 1);
    h.advance(5);
    // The split runs on unrounded damage and each recipient's health loss is
    // rounded separately, so the shares may each be off by one point.
    let (ally_damage, caster_damage) = (before - h.health(ally), 1000.0 - h.health(h.caster));
    let total = ally_damage + caster_damage;
    let spread = varied(197.25);
    assert!(
        total >= spread.start() - 1.0 && total <= spread.end() + 1.0,
        "total {total}"
    );
    assert!(
        (caster_damage - total * 0.3).abs() <= 1.0,
        "ally {ally_damage} caster {caster_damage}"
    );
    h.app
        .world_mut()
        .get_mut::<Health>(h.caster)
        .unwrap()
        .current = 210.0;
    super::cast_triggered(h.app.world_mut(), h.npc, 108853, Some(ally), 1);
    h.advance(5);
    assert_eq!(h.health(h.caster), 200.0);
    assert!(!crate::auras::has_aura(h.app.world(), ally, 6940));
}

#[test]
fn guardians_sacrifice_periodic_absorb_immunity_and_expiry() {
    let (mut h, ally) = sacrifice();
    let spell = world_db().unwrap().store.get(108853).unwrap();
    let ctx = super::effects::EffectCtx {
        caster: h.npc,
        spell,
        depth: 1,
        dest: None,
        chain_multiplier: 1.0,
    };
    super::effects::deal_damage(h.app.world_mut(), &ctx, ally, 100.0, false, true);
    assert_eq!(h.health(ally), 930.0);
    assert_eq!(h.health(h.caster), 970.0);
    super::cast_triggered(h.app.world_mut(), h.caster, 17, Some(h.caster), 0);
    crate::combat::deal_direct_damage(h.app.world_mut(), h.npc, ally, 100.0);
    assert_eq!(
        h.health(h.caster),
        970.0,
        "recipient absorb consumes the transferred share"
    );
    assert_eq!(h.health(ally), 860.0);
    super::cast_triggered(h.app.world_mut(), h.caster, 642, Some(h.caster), 0);
    crate::combat::deal_direct_damage(h.app.world_mut(), h.npc, ally, 100.0);
    assert_eq!(
        h.health(ally),
        760.0,
        "immune recipients do not receive splits"
    );
    h.advance(260);
    assert!(!crate::auras::has_aura(h.app.world(), ally, 6940));
    let before = h.health(ally);
    crate::combat::deal_direct_damage(h.app.world_mut(), h.npc, ally, 100.0);
    assert_eq!(h.health(ally), before - 100.0);
}

#[test]
fn guardians_sacrifice_splits_real_auto_attack_damage() {
    let (mut h, ally) = sacrifice();
    h.app
        .world_mut()
        .entity_mut(h.npc)
        .remove::<crate::stats::CharacterStats>()
        .remove::<shared::components::Npc>()
        .insert(crate::combat::AutoAttack::new(ally));
    h.advance(40);
    let ally_loss = 1000.0 - h.health(ally);
    let owner_loss = 1000.0 - h.health(h.caster);
    assert!(ally_loss > 0.0, "real swings must land");
    assert!(
        (owner_loss - ally_loss * 3.0 / 7.0).abs() < 0.1,
        "melee must split too: {ally_loss}/{owner_loss}"
    );
}

#[test]
fn guardians_boundary_dead_victim_cannot_transfer_direct_damage() {
    let (mut h, ally) = sacrifice();
    h.app.world_mut().get_mut::<Health>(ally).unwrap().current = 0.0;
    crate::combat::deal_direct_damage(h.app.world_mut(), h.npc, ally, 100.0);
    assert_eq!(h.health(h.caster), 1000.0);
}

#[test]
fn guardians_boundary_same_tick_swings_share_one_native_health_floor_budget() {
    let (mut h, ally) = sacrifice();
    h.app
        .world_mut()
        .get_mut::<Health>(h.caster)
        .unwrap()
        .current = 210.0;
    // Untyped melee units use the deterministic default hit table: this proves
    // deferred health routing, independently of creature crit/miss RNG.
    let mut attackers = vec![h.npc];
    for _ in 0..3 {
        attackers.push(spawn_npc(&mut h.app, 3.0));
    }
    for attacker in attackers {
        h.app
            .world_mut()
            .entity_mut(attacker)
            .remove::<crate::stats::CharacterStats>()
            .remove::<shared::components::Npc>()
            .insert(crate::combat::AutoAttack::new(ally));
    }
    // Execute the actual batched swing system and deferred commands without
    // intervening player health regeneration changing the seeded budget.
    use bevy::ecs::system::RunSystemOnce;
    h.app
        .world_mut()
        .run_system_once(crate::combat::auto_attack_system)
        .unwrap();
    assert_eq!(
        h.health(h.caster),
        200.0,
        "queued swings must not each spend the same floor budget"
    );
    assert_eq!(h.health(ally), 970.0);
    assert!(!crate::auras::has_aura(h.app.world(), ally, 6940));
}

#[test]
fn guardians_sacrifice_splits_direct_damage_and_excludes_foreign_map() {
    let (mut h, ally) = sacrifice();
    crate::combat::deal_direct_damage(h.app.world_mut(), h.npc, ally, 100.0);
    assert_eq!(h.health(ally), 930.0);
    assert_eq!(h.health(h.caster), 970.0);
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::maps::InMap::continent(1));
    crate::combat::deal_direct_damage(h.app.world_mut(), h.npc, ally, 100.0);
    assert_eq!(h.health(ally), 830.0);
    assert_eq!(h.health(h.caster), 970.0);
}

// Tests load the real prepared overlay, never a substituted Imp template.
fn spawn_darkglare(h: &mut Harness) -> Entity {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("prepared.sqlite");
    std::fs::copy(&world_db().unwrap().powers_db, &path).unwrap();
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../scripts/add_guardian_templates.py");
    let result = std::process::Command::new("python3")
        .arg(script)
        .arg("--db")
        .arg(&path)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    h.app.insert_resource(crate::world_data::load_world_data(
        &directory.path().join("prepared.sqlite"),
    ));
    let source = rusqlite::Connection::open_with_flags(
        &world_db().unwrap().powers_db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(super::summon::load_summon_properties(&source).unwrap());
    // Archive has no Darkglare spell rows: the native root description is the binding.
    h.app
        .insert_resource(super::guardian::GuardianSpells::default());
    let world = h.app.world_mut();
    world
        .get_mut::<shared::components::Position>(h.npc)
        .unwrap()
        .x = 20.0;
    world
        .get_mut::<crate::stats::CharacterStats>(h.caster)
        .unwrap()
        .spell_power = 1000.0;
    world
        .get_mut::<crate::networking::CombatTarget>(h.caster)
        .unwrap()
        .0 = Some(h.npc);
    let spell = world_db().unwrap().store.get(205180).unwrap();
    let effect = spell
        .effects
        .iter()
        .find(|effect| effect.effect == 28)
        .unwrap();
    let ctx = super::effects::EffectCtx {
        caster: h.caster,
        spell,
        depth: 0,
        dest: Some(super::targets::Dest {
            position: Vec3::ZERO,
            orientation: 0.0,
        }),
        chain_multiplier: 1.0,
    };
    super::summon::summon_creature(world, &ctx, effect);
    world
        .query_filtered::<Entity, With<super::summon::SummonedCreature>>()
        .single(world)
        .unwrap()
}

fn build_darkglare_harness() -> Harness {
    let mut beam = world_db().unwrap().store.get(205231).unwrap().clone();
    // Pin only variance to exercise exact damage and power changes, not RNG.
    beam.effects[0].variance = 0.0;
    harness(vec![beam], &[205180], 9).unwrap()
}

#[test]
fn guardians2_darkglare_native_beam_cast_time_and_live_demon_power() {
    let mut h = build_darkglare_harness();
    let actor = spawn_darkglare(&mut h);
    h.advance(2);
    assert_eq!(
        h.app
            .world()
            .get::<shared::casting::CastState>(actor)
            .unwrap()
            .spell_id,
        205231
    );
    assert_eq!(h.health(h.npc), 5000.0);
    h.advance(43);
    assert_eq!(h.health(h.npc), 4269.0, "4.87044 * inherited150SP rounded");
    h.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(h.caster)
        .unwrap()
        .spell_power = 2000.0;
    h.advance(40);
    assert_eq!(
        h.health(h.npc),
        2808.0,
        "next beam reads changed owner SP, not a spawn snapshot"
    );
}

#[test]
fn guardians2_darkglare_retargets_stops_and_expires_without_pet_replacement() {
    let mut h = build_darkglare_harness();
    let pet = spawn_player(&mut h.app, 9, 2.0);
    h.app
        .world_mut()
        .entity_mut(pet)
        .remove::<shared::components::Player>()
        .insert(crate::hunter_pet::HunterPet {
            owner: h.caster,
            slot: 0,
        });
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::hunter_pet::PetStable {
            character_id: Some(1),
            stored: default(),
            summoned: Some(pet),
        });
    let actor = spawn_darkglare(&mut h);
    let other = spawn_npc(&mut h.app, 25.0);
    h.app
        .world_mut()
        .get_mut::<crate::networking::CombatTarget>(h.caster)
        .unwrap()
        .0 = Some(other);
    h.advance(45);
    assert_eq!(h.health(h.npc), 5000.0);
    assert_eq!(h.health(other), 4269.0);
    h.app
        .world_mut()
        .get_mut::<crate::networking::CombatTarget>(h.caster)
        .unwrap()
        .0 = None;
    h.advance(45); // A previously started beam may finish.
    let before = h.health(other);
    h.advance(50);
    assert_eq!(h.health(other), before);
    assert_eq!(
        crate::hunter_pet::summoned_pet(h.app.world(), h.caster),
        Some(pet)
    );
    h.advance(300);
    assert!(
        h.app.world().get_entity(actor).is_err(),
        "native20sec expiry"
    );
    assert_eq!(
        crate::hunter_pet::summoned_pet(h.app.world(), h.caster),
        Some(pet)
    );
}

#[test]
fn guardians2_unbound_demon_is_not_an_imp_substitution() {
    let mut h = harness(Vec::new(), &[111859, 3110], 9).unwrap();
    let actor = imp(&mut h);
    h.app
        .world_mut()
        .entity_mut(actor)
        .remove::<super::guardian::GuardianActor>();
    h.app
        .world_mut()
        .get_mut::<shared::components::Npc>(actor)
        .unwrap()
        .template_id = 98035;
    h.app
        .world_mut()
        .get_mut::<super::summon::SummonedCreature>(actor)
        .unwrap()
        .spell_id = 193332;
    super::guardian::initialize(h.app.world_mut(), actor);
    h.advance(45);
    assert_eq!(
        h.health(h.npc),
        5000.0,
        "unbound Dreadstalker must not substitute Imp Firebolt"
    );
}
