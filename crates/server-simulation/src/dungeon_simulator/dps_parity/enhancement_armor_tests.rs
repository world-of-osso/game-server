//! Source-qualified Doom and Windstrike damage must bypass physical armor.
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

fn prepare_root(root: u32) -> Run {
    let mut run = prepare_literal(263, 1).unwrap();
    engage(&mut run);
    if root == 115356 {
        let actor = run.actor;
        enhancement_generation_tests::cast(&mut run, 114051, actor);
        assert!(crate::auras::has_aura(run.app.world(), actor, 114051));
    }
    run.app
        .world_mut()
        .entity_mut(run.actor)
        .remove::<crate::combat::AutoAttack>();
    run
}

pub(super) fn apply_armor_control(run: &mut Run, armorless: bool) {
    const DEMATERIALIZE_ARMOR: u32 = 40935;
    let target = run.targets[0];
    assert!(
        run.app
            .world()
            .get::<crate::level_scaling::CreatureArmor>(target)
            .unwrap()
            .0
            > 0.0
    );
    if armorless {
        // Tuned creatures ignore raw CreatureArmor edits. Native40935 has only101,-100%,Physical;
        // its total multiplier runs after target-level armor scaling and adds no damage amplifier.
        crate::spell_cast::cast_triggered(
            run.app.world_mut(),
            run.actor,
            DEMATERIALIZE_ARMOR,
            Some(target),
            1,
        );
    }
    preparation::finish_pending_setup(run).unwrap();
    if armorless {
        assert!(crate::auras::has_aura(
            run.app.world(),
            target,
            DEMATERIALIZE_ARMOR
        ));
        assert_eq!(
            run.app
                .world()
                .get::<crate::stats::StatModifiers>(target)
                .unwrap()
                .total_armor_pct,
            -100.0,
            "the actual native control must remove scaled armor, not just change its raw input"
        );
    }
}

fn damage_spells(root: u32) -> &'static [u32] {
    match root {
        384352 => &[469270],
        115356 => &[115357, 115360],
        17364 => &[32175, 32176],
        _ => unreachable!(),
    }
}

fn collect_damage(run: &Run, root: u32, before: usize) -> Vec<(u32, i32)> {
    run.app.world().resource::<ObservedCombatEvents>().0[before..]
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
                && event
                    .spell_id
                    .is_some_and(|spell| damage_spells(root).contains(&spell))
        })
        .map(|event| (event.spell_id.unwrap(), event.amount))
        .collect()
}

fn actual_damage(root: u32, armorless: bool) -> Vec<(u32, i32)> {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_root(root);
    apply_armor_control(&mut run, armorless);
    let before = run.app.world().resource::<ObservedCombatEvents>().0.len();
    let selected = if root == 384352 {
        run.actor
    } else {
        run.targets[0]
    };
    run.app.world_mut().insert_resource(
        crate::spell_cast::effects::factor_capture::FactorCapture::new(
            run.actor,
            run.targets[0],
            damage_spells(root)[0],
            "armor_control",
            4,
        ),
    );
    let _damage_seed = crate::random::seed_scope(818181);
    enhancement_generation_tests::cast(&mut run, root, selected);
    let damage = collect_damage(&run, root, before);
    println!(
        "ENHANCEMENT_ARMOR_FACTORS armorless={armorless} {}",
        run.app
            .world()
            .resource::<crate::spell_cast::effects::factor_capture::FactorCapture>()
            .export()
    );
    assert!(
        !damage.is_empty(),
        "actual native root{root} must deal damage"
    );
    let armor_pct = run
        .app
        .world()
        .get::<crate::stats::StatModifiers>(run.targets[0])
        .map_or(0.0, |modifiers| modifiers.total_armor_pct);
    println!(
        "ENHANCEMENT_ARMOR root={root} armorless={armorless} armor_pct={armor_pct} damage={damage:?}"
    );
    damage
}

#[test]
#[ignore = "native263 reference inputs and private world database"]
fn enhancement_actual_physical_stormstrike_is_reduced_by_target_armor() {
    let armored: i32 = actual_damage(17364, false)
        .iter()
        .map(|(_, amount)| *amount)
        .sum();
    let unarmored: i32 = actual_damage(17364, true)
        .iter()
        .map(|(_, amount)| *amount)
        .sum();
    assert!(
        armored < unarmored,
        "the real physical control must distinguish armor: armored={armored}, unarmored={unarmored}"
    );
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
