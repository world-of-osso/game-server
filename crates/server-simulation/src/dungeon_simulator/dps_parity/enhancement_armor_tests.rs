//! Source-qualified Doom and Windstrike damage must bypass physical armor.
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

fn actual_damage(root: u32, armorless: bool) -> Vec<(u32, i32)> {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(263, 1).unwrap();
    engage(&mut run);
    let target = run.targets[0];
    if root == 115356 {
        let actor = run.actor;
        enhancement_generation_tests::cast(&mut run, 114051, actor);
        assert!(crate::auras::has_aura(run.app.world(), actor, 114051));
    }
    run.app
        .world_mut()
        .entity_mut(run.actor)
        .remove::<crate::combat::AutoAttack>();
    let armor = run
        .app
        .world_mut()
        .get_mut::<crate::level_scaling::CreatureArmor>(target)
        .unwrap();
    assert!(
        armor.0 > 0.0,
        "the native dummy supplies real nonzero armor"
    );
    if armorless {
        run.app
            .world_mut()
            .get_mut::<crate::level_scaling::CreatureArmor>(target)
            .unwrap()
            .0 = 0.0;
    }
    let before = run.app.world().resource::<ObservedCombatEvents>().0.len();
    let selected = if root == 384352 { run.actor } else { target };
    enhancement_generation_tests::cast(&mut run, root, selected);
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    let damage: Vec<_> = events[before..]
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(target.to_bits())
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
                && match root {
                    384352 => event.spell_id == Some(469270),
                    115356 => matches!(event.spell_id, Some(115357 | 115360)),
                    _ => unreachable!(),
                }
        })
        .map(|event| (event.spell_id.unwrap(), event.amount))
        .collect();
    assert!(
        !damage.is_empty(),
        "actual native root{root} must deal its weapon/area damage: {:?}",
        &events[before..]
    );
    damage
}

#[test]
#[ignore = "native263 reference inputs and private world database"]
fn enhancement_actual_doom_hybrid_damage_ignores_target_physical_armor() {
    // SimC76f8118 action.cpp1686–1689: only purePhysical reads armor; Doom469270 isStormstrike9.
    assert_eq!(actual_damage(384352, false), actual_damage(384352, true));
}

#[test]
#[ignore = "native263 reference inputs and private world database"]
fn enhancement_actual_ascendance_windstrike_weapon_hits_ignore_target_armor() {
    // SimC76f8118 sc_shaman.cpp4724–4731: Windstrike's real MH/OH actions explicitlyignorearmor.
    assert_eq!(actual_damage(115356, false), actual_damage(115356, true));
}
