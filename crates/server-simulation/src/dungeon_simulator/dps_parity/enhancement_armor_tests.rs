//! Source-qualified Doom and Windstrike damage must bypass physical armor.
use super::*;

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

pub(super) fn assert_armor_response(run: &Run, spells: &[u32], reduces_damage: bool) {
    let capture = run
        .app
        .world()
        .resource::<crate::spell_cast::effects::factor_capture::FactorCapture>()
        .export();
    let events = capture["events"].as_array().expect("native factor events");
    for spell in spells {
        let hits: Vec<_> = events
            .iter()
            .filter(|event| {
                event["event"] == "result" && event["spell_id"].as_u64() == Some(u64::from(*spell))
            })
            .collect();
        assert!(
            !hits.is_empty(),
            "actual native spell{spell} must produce damage"
        );
        for hit in hits {
            assert_hit_armor_response(hit, reduces_damage);
        }
    }
}

fn assert_hit_armor_response(hit: &Value, reduces_damage: bool) {
    let factors = &hit["factors"];
    let input = factors["native_rounded_amount"]
        .as_f64()
        .expect("native pre-mitigation amount");
    let actual = factors["actual_amount"]
        .as_f64()
        .expect("actual native damage");
    assert!(
        input > 0.0 && actual > 0.0,
        "the actual native spell must deal damage"
    );
    assert_eq!(
        factors["blocked_amount"], 0,
        "armor control has no shield block"
    );
    assert_eq!(
        factors["absorbed_amount"], 0,
        "armor control has no absorbs"
    );
    println!(
        "NATIVE_ARMOR spell={} input={input} actual={actual} reduces={reduces_damage}",
        hit["spell_id"]
    );
    if reduces_damage {
        assert!(
            actual < input,
            "native Physical damage must lose damage to armor"
        );
    } else {
        assert_eq!(
            actual, input,
            "native damage must retain its own rolled amount without armor loss"
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

fn assert_actual_damage(root: u32, armorless: bool) {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_root(root);
    apply_armor_control(&mut run, armorless);
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
    // The control cast can proc real stat gear. Compare each actual hit with
    // its own native pre-mitigation roll, not a different cast's Mastery/Vers.
    let reduces_damage = root == 17364 && !armorless;
    assert_armor_response(&run, damage_spells(root), reduces_damage);
}

#[test]
#[ignore = "native263 reference inputs and private world database"]
fn enhancement_actual_physical_stormstrike_is_reduced_by_target_armor() {
    assert_actual_damage(17364, false);
    assert_actual_damage(17364, true);
}

#[test]
#[ignore = "native263 reference inputs and private world database"]
fn enhancement_actual_doom_hybrid_damage_ignores_target_physical_armor() {
    // SimC76f8118 action.cpp1686–1689: only purePhysical reads armor; Doom469270 isStormstrike9.
    assert_actual_damage(384352, false);
    assert_actual_damage(384352, true);
}

#[test]
#[ignore = "native263 reference inputs and private world database"]
fn enhancement_actual_ascendance_windstrike_weapon_hits_ignore_target_armor() {
    // SimC76f8118 sc_shaman.cpp4724–4731: Windstrike's real MH/OH actions explicitlyignorearmor.
    assert_actual_damage(115356, false);
    assert_actual_damage(115356, true);
}
