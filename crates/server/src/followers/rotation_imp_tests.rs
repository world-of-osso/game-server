//! Native Wild Imp binding and WARLOCK_WILD_IMP_OWNER_SP_SIMC; no substituted actor/stats.
use super::kit_tests::at_level;
use crate::spell_cast::summon::SummonedCreature;
use crate::spell_cast::test_harness::{Harness, world_db};
use bevy::prelude::*;
use shared::components::Health;
use shared::protocol::CombatLogKind;

fn prepared_world() -> crate::world_data::WorldData {
    let root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/wild-imp-fixtures");
    std::fs::create_dir_all(&root).unwrap();
    let directory = tempfile::tempdir_in(root).unwrap();
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
    crate::world_data::load_world_data(&path)
}

fn warlock(level: u8) -> (Harness, Entity) {
    let (mut h, owner) = at_level(230861, level, |app| {
        app.add_plugins(crate::stats::StatsPlugin);
    });
    h.app.insert_resource(prepared_world());
    h.app.world_mut().entity_mut(h.npc).insert(Health {
        current: 100_000_000.0,
        max: 100_000_000.0,
    });
    let target = h.npc;
    h.app
        .world_mut()
        .entity_mut(owner)
        .insert(crate::networking::CombatTarget(Some(target)));
    (h, owner)
}

fn summon_imp(h: &mut Harness, owner: Entity) -> Entity {
    assert!(
        crate::spell_cast::try_cast_triggered(h.app.world_mut(), owner, 104317, Some(owner), 0),
        "actual native104317 must summon its authenticated55659"
    );
    let world = h.app.world_mut();
    world
        .query::<(Entity, &SummonedCreature)>()
        .iter(world)
        .find(|(_, summon)| summon.owner == owner && summon.spell_id == 104317)
        .map(|(entity, _)| entity)
        .expect("real native Wild Imp actor")
}

fn first_firebolt(h: &mut Harness, imp: Entity) -> Option<i32> {
    for _ in 0..400 {
        h.advance(1);
        if let Some(event) = h.inbox.log.iter().find(|event| {
            event.source == Some(imp.to_bits())
                && event.spell_id == Some(104318)
                && event.kind == CombatLogKind::Damage
                && !event.crit
        }) {
            return Some(event.amount + event.absorbed + event.resisted + event.blocked);
        }
    }
    None
}

#[test]
fn rotation_fidelity_wild_imp_uses_actual_native_summon_and_sourced_firebolt_binding() {
    let (mut h, owner) = warlock(32);
    let imp = summon_imp(&mut h, owner);
    let damage = first_firebolt(&mut h, imp)
        .expect("authentic Wild Imp must cast its sourced104318, not3110");
    assert!(damage > 0, "actual Fel Firebolt damage");
    assert!(
        !h.inbox
            .go
            .iter()
            .any(|event| event.caster == imp.to_bits() && event.spell_id == 3110),
        "Wild Imp must never substitute Grimoire Imp's archive Firebolt"
    );
}

fn owner_scaled_firebolt(level: u8) -> (f32, f32, i32) {
    let (mut h, owner) = warlock(level);
    // Explicit pinned source fixture isolates inheritance from the separate
    // production-binding test: SimC76f8118 sc_warlock_pets.cpp721/867, not a
    // fabricated TDB spell row or a replacement Imp416 actor.
    let mut bindings = crate::spell_cast::guardian::GuardianSpells::default();
    bindings.spells.insert(55659, vec![104318]);
    h.app.insert_resource(bindings);
    let imp = summon_imp(&mut h, owner);
    let world = h.app.world();
    let owner_power = world
        .get::<crate::stats::CharacterStats>(owner)
        .unwrap()
        .spell_power;
    assert!(owner_power > 0.0, "actual native owner derived spell power");
    let spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(104318)
        .unwrap();
    let effect = spell
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap();
    let summon_modifier = crate::auras::summon_damage::multiplier(world, imp);
    // SimC76f8118 Wild Imp constructor705 overrides base19: SP share1.2.
    let expected = owner_power * 1.2 * effect.bonus_coefficient * summon_modifier;
    let tolerance = expected * effect.variance + 1.0;
    let observed = first_firebolt(&mut h, imp)
        .expect("source-bound native Firebolt must land a noncritical hit");
    assert!(
        (observed as f32 - expected).abs() <= tolerance,
        "WARLOCK_WILD_IMP_OWNER_SP_SIMC1.2/native coefficient, no legacy0.15: level={level}, owner_sp={owner_power}, expected={expected}, observed={observed}, native_variance_bound={tolerance}"
    );
    (owner_power, expected, observed)
}

#[test]
fn rotation_fidelity_wild_imp_exhausts_native_energy_without_regeneration_and_despawns() {
    let (mut h, owner) = warlock(32);
    let imp = summon_imp(&mut h, owner);
    let initial = crate::spell_cast::powers::power_value(
        h.app.world(),
        imp,
        shared::components::PowerType::Energy,
    )
    .unwrap()
    .0;
    let cost = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(104318)
        .unwrap()
        .power_costs
        .iter()
        .find(|cost| cost.power_type == shared::components::PowerType::Energy as i32)
        .unwrap()
        .mana_cost as f32;
    assert_eq!(initial, 100.0, "native Energy pool and pinned Imp base100");
    assert_eq!(cost, 20.0, "native104318 Energy cost");
    h.advance(500); //25s is before native104317's40s lifetime.
    let casts = h
        .inbox
        .go
        .iter()
        .filter(|event| event.caster == imp.to_bits() && event.spell_id == 104318)
        .count();
    assert_eq!(
        casts,
        (initial / cost) as usize,
        "SimC disables Imp regen; exhaustion budget comes from native100/20, not guessed bolts"
    );
    assert!(
        h.app.world().get_entity(imp).is_err(),
        "exhausted Imp must demise before its independent native40s expiry"
    );
    let outcomes = h
        .inbox
        .log
        .iter()
        .filter(|event| {
            event.source == Some(imp.to_bits())
                && event.spell_id == Some(104318)
                && matches!(event.kind, CombatLogKind::Damage | CombatLogKind::Miss(_))
        })
        .count();
    assert_eq!(
        outcomes, casts,
        "last in-flight bolt must resolve before source retirement"
    );
}

#[test]
fn rotation_fidelity_wild_imp_native_idle_expiry_also_rolls_natural_core() {
    let (mut h, owner) = warlock(32);
    let imps: Vec<_> = (0..21).map(|_| summon_imp(&mut h, owner)).collect();
    h.app
        .world_mut()
        .get_mut::<crate::networking::CombatTarget>(owner)
        .unwrap()
        .0 = None;
    h.advance(780); //39s: retain native40s lifetime, no target or energy spending.
    assert!(
        imps.iter()
            .all(|imp| h.app.world().get_entity(*imp).is_ok())
    );
    assert!(
        !h.inbox
            .go
            .iter()
            .any(|event| imps.iter().any(|imp| event.caster == imp.to_bits())
                && event.spell_id == 104318)
    );
    h.advance(40);
    assert!(
        imps.iter()
            .all(|imp| h.app.world().get_entity(*imp).is_err()),
        "all actual native40s expiries must complete before checking Core"
    );
    assert!(
        crate::auras::has_aura(h.app.world(), owner, 264173),
        "pinned natural-demise PRD includes expiry, not only Energy exhaustion"
    );
}

#[test]
fn rotation_fidelity_wild_imp_unlearned_demoniac_does_not_grant_core() {
    let (mut h, owner) = warlock(14);
    assert!(
        !h.app
            .world()
            .get::<crate::class_progression::KnownSpells>(owner)
            .unwrap()
            .contains(426115)
    );
    let imps: Vec<_> = (0..21).map(|_| summon_imp(&mut h, owner)).collect();
    h.advance(500);
    assert!(
        imps.iter()
            .all(|imp| h.app.world().get_entity(*imp).is_err())
    );
    assert!(
        !crate::auras::has_aura(h.app.world(), owner, 264173),
        "natural fades must not grant the unlearned15 Demoniac effect at14"
    );
}

#[test]
fn rotation_fidelity_wild_imp_demise_naturally_earns_and_spends_demonic_core() {
    let (mut h, owner) = warlock(32);
    assert!(
        h.app
            .world()
            .get::<crate::class_progression::KnownSpells>(owner)
            .unwrap()
            .contains(426115),
        "actual learned Demoniac driver"
    );
    assert!(!crate::auras::has_aura(h.app.world(), owner, 264173));
    // Native264173 cumulative4; pinned SimC fade PRD hardcap21 per success.
    //84real fades guarantee four earned uses without injecting a proc or RNG.
    let imps: Vec<_> = (0..84).map(|_| summon_imp(&mut h, owner)).collect();
    h.advance(500);
    assert!(
        imps.iter()
            .all(|imp| h.app.world().get_entity(*imp).is_err()),
        "all84real native actors must exhaust naturally before testing the proc"
    );
    let core_stacks = |world: &World| {
        world
            .get::<crate::auras::ActiveAuras>(owner)
            .and_then(|auras| auras.iter().find(|aura| aura.spell_id == 264173))
            .map_or(0, |aura| aura.stacks)
    };
    let before = core_stacks(h.app.world());
    assert_eq!(
        before, 4,
        "84natural fades must earn native four-use Core cap through sourced PRD, no injected aura/RNG"
    );
    let shards = crate::spell_cast::powers::power_value(
        h.app.world(),
        owner,
        shared::components::PowerType::SoulShards,
    )
    .unwrap();
    let energize = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(280127)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .base_points;
    let target = h.npc;
    crate::spell_cast::creature_cast(h.app.world_mut(), owner, 264178, Some(target), false)
        .expect("ordinary Demonbolt admitted with actual earned Core");
    h.advance(1);
    assert!(
        h.inbox
            .go
            .iter()
            .any(|event| event.caster == owner.to_bits() && event.spell_id == 264178),
        "Core must make the actual native Demonbolt instant"
    );
    assert_eq!(
        core_stacks(h.app.world()),
        before - 1,
        "one actual Demonbolt consumes one earned Core use"
    );
    let after_shards = crate::spell_cast::powers::power_value(
        h.app.world(),
        owner,
        shared::components::PowerType::SoulShards,
    )
    .unwrap()
    .0;
    assert_eq!(
        after_shards,
        (shards.0 + energize).min(shards.1),
        "Core consumption must preserve actual native280127 shard generation"
    );
}

#[test]
fn rotation_fidelity_wild_imp_hand_of_guldan_spawns_three_native_actors() {
    let (mut h, owner) = warlock(32);
    let target = h.npc;
    let shards = crate::spell_cast::powers::power_value(
        h.app.world(),
        owner,
        shared::components::PowerType::SoulShards,
    )
    .unwrap()
    .0;
    assert!(
        shards >= 30.0,
        "native default three shards, never injected"
    );
    crate::spell_cast::creature_cast(h.app.world_mut(), owner, 105174, Some(target), false)
        .expect("ordinary native three-shard Hand of Guldan");
    h.advance(100);
    assert!(
        h.inbox.log.iter().any(|event| {
            event.source == Some(owner.to_bits())
                && event.spell_id == Some(86040)
                && event.kind == CombatLogKind::Damage
        }),
        "actual meteor must land before qualifying its missing summons"
    );
    let world = h.app.world_mut();
    let imps: Vec<_> = world
        .query::<(Entity, &SummonedCreature)>()
        .iter(world)
        .filter(|(_, summon)| summon.owner == owner && summon.spell_id == 104317)
        .map(|(actor, _)| actor)
        .collect();
    assert_eq!(
        imps.len(),
        3,
        "native fixed30raw shard cost and sourced one Imp per shard"
    );
    for imp in imps {
        assert_eq!(
            h.app
                .world()
                .get::<shared::components::Npc>(imp)
                .unwrap()
                .template_id,
            55659
        );
        assert!(
            first_firebolt(&mut h, imp).is_some(),
            "each real Imp must attack its owner's target"
        );
    }
}

#[test]
fn rotation_fidelity_wild_imp_firebolt_scales_with_actual_native_owner_spell_power() {
    let low = owner_scaled_firebolt(32);
    let high = owner_scaled_firebolt(40);
    assert!(high.0 > low.0, "native level40 owner has more SP than32");
    assert!(
        high.2 > low.2,
        "actual Firebolt damage grows with native owner SP"
    );
}
