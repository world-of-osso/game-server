use super::*;
use crate::simc_apl_native::{Encounter, FightStyle, NativeFacts};
use server::simc_apl::Facts;

fn facts<'a>(run: &'a super::super::Run, bindings: &'a Bindings) -> NativeFacts<'a> {
    NativeFacts::new(
        run.app.world(),
        run.actor,
        bindings,
        Encounter::Fixture {
            started_at: 0.0,
            duration: 300.0,
            style: FightStyle::Patchwerk,
        },
        run.targets.len(),
    )
    .with_targets(run.targets.first().copied(), &run.targets)
}

#[test]
#[ignore = "native Ret literal bindings: actual earned Judgment HolyPower, no fixture grant"]
fn dps_parity_ret_literal_native_resource_tracks_earned_judgment() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = super::super::ret_tier_tests::native_ret(4);
    let bindings = bindings();
    let before = facts(&run, &bindings).value("holy_power").unwrap();
    assert_eq!(before, 0.0);
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        20271,
        Some(run.targets[0]),
        false,
    )
    .unwrap();
    for _ in 0..200 {
        run.app.update();
        if facts(&run, &bindings).value("holy_power").unwrap() > before {
            break;
        }
    }
    assert_eq!(
        facts(&run, &bindings).value("holy_power").unwrap(),
        before + 1.0
    );
}

#[test]
#[ignore = "native Ret literal bindings: selected traits and actual active aura state"]
fn dps_parity_ret_literal_native_talents_and_buffs_read_actual_state() {
    let run = super::super::ret_tier_tests::native_ret(4);
    let bindings = bindings();
    let traits = run
        .app
        .world()
        .get::<crate::class_progression::TraitLoadout>(run.actor)
        .unwrap();
    let native = facts(&run, &bindings);
    for (name, id) in [
        ("radiant_glory", 458359),
        ("execution_sentence", 343527),
        ("holy_flames", 406545),
        ("crusade", 1253598),
        ("walk_into_light", 1263782),
    ] {
        let selected = traits
            .active_talent_spells()
            .any(|spell| spell.spell_id == id);
        assert_eq!(
            native.value(&format!("talent.{name}")).unwrap(),
            if selected { 1.0 } else { 0.0 },
            "selected native talent {name}"
        );
    }
    for (name, id) in [
        ("avenging_wrath", 31884),
        ("divine_arbiter_verdict", 1306161),
        ("empyrean_legacy", 387178),
        ("divine_arbiter_divine_storm", 1306162),
        ("hammer_of_light_free", 433732),
        ("undisputed_ruling", 432629),
        ("art_of_war", 406086),
        ("righteous_cause", 402916),
    ] {
        let active = crate::auras::has_aura(run.app.world(), run.actor, id);
        assert_eq!(
            native.value(&format!("buff.{name}.up")).unwrap(),
            if active { 1.0 } else { 0.0 },
            "actual native aura {name}"
        );
    }
    assert_eq!(native.value("dot.expurgation.ticking").unwrap(), 0.0);
    assert_eq!(
        native.value("debuff.execution_sentence_debuff.up").unwrap(),
        0.0
    );
}

#[test]
#[ignore = "native Ret literal bindings: unsupported identities remain explicit errors"]
fn dps_parity_ret_literal_native_unknown_aliases_are_errors() {
    let run = super::super::ret_tier_tests::native_ret(4);
    let bindings = bindings();
    let native = facts(&run, &bindings);
    for name in [
        "holy_power_typo",
        "talent.radiant_glory_typo",
        "buff.empyrean_legacy_typo.up",
        "cooldown.avenging_wrath_typo.remains",
        "dot.expurgation_typo.ticking",
    ] {
        assert!(native.value(name).is_err(), "unsupported alias {name}");
    }
}
