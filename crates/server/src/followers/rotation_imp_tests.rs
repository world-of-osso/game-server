//! Native Wild Imp binding and WARLOCK_WILD_IMP_OWNER_SP_SIMC; no substituted actor/stats.
use super::kit_tests::at_level;
use crate::spell_cast::summon::SummonedCreature;
use crate::spell_cast::test_harness::{Harness, world_db};
use bevy::prelude::*;
use shared::components::Health;
use shared::protocol::CombatLogKind;

fn prepared_world() -> crate::world_data::WorldData {
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
    h.app
        .world_mut()
        .get_mut::<crate::networking::CombatTarget>(owner)
        .unwrap()
        .0 = Some(h.npc);
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
fn rotation_fidelity_wild_imp_firebolt_scales_with_actual_native_owner_spell_power() {
    let low = owner_scaled_firebolt(32);
    let high = owner_scaled_firebolt(40);
    assert!(high.0 > low.0, "native level40 owner has more SP than32");
    assert!(
        high.2 > low.2,
        "actual Firebolt damage grows with native owner SP"
    );
}
