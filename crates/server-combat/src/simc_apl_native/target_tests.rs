//! Actual native target/aura state, not actor-wide scalar substitutes.
use super::*;
use shared::components::Health;

const COLOSSUS_SMASH: u32 = 167105;
const COLOSSUS_WOUND: u32 = 208086;
const REND: u32 = 772;
const REND_DOT: u32 = 388539;

fn warrior() -> crate::spell_cast::test_harness::Harness {
    let mut h = crate::spell_cast::test_harness::harness(
        Vec::new(),
        &[COLOSSUS_SMASH, COLOSSUS_WOUND, REND, REND_DOT],
        1,
    )
    .unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::networking::CombatTarget(Some(h.npc)));
    h
}

fn encounter(world: &World) -> Encounter {
    Encounter::Fixture {
        started_at: crate::spell_cast::history_now(world),
        duration: 300.0,
        style: FightStyle::Patchwerk,
    }
}

#[test]
fn arms_native_apl_target_health_reads_the_actual_selected_unit() {
    let mut h = warrior();
    let binding = Bindings::default();
    for (current, expected) in [(1250.0, 25.0), (750.0, 15.0)] {
        h.app.world_mut().entity_mut(h.npc).insert(Health {
            current,
            max: 5000.0,
        });
        let facts = NativeFacts::new(
            h.app.world(),
            h.caster,
            &binding,
            encounter(h.app.world()),
            1,
        );
        assert_eq!(facts.value("target.health.pct").unwrap(), expected);
    }
}

#[test]
fn arms_native_apl_target_aura_remains_requires_this_casters_real_aura() {
    let mut h = warrior();
    let foreign = crate::spell_cast::test_harness::spawn_player(&mut h.app, 1, 0.0);
    let binding = Bindings {
        buffs: [
            ("colossus_smash".into(), COLOSSUS_WOUND),
            ("rend_dot".into(), REND_DOT),
        ]
        .into(),
        ..Default::default()
    };
    for (spell, prefix) in [
        (COLOSSUS_WOUND, "debuff.colossus_smash"),
        (REND_DOT, "dot.rend_dot"),
    ] {
        let remaining = format!("{prefix}.remains");
        crate::auras::apply_aura(
            h.app.world_mut(),
            &crate::auras::AuraApplication {
                caster: foreign,
                target: h.npc,
                spell_id: spell,
                effect_mask: 1,
                source: crate::auras::AuraSource::Cast,
            },
        );
        assert!(
            h.app
                .world()
                .get::<crate::auras::ActiveAuras>(h.npc)
                .unwrap()
                .iter()
                .any(|aura| aura.spell_id == spell && aura.caster == foreign)
        );
        let before = NativeFacts::new(
            h.app.world(),
            h.caster,
            &binding,
            encounter(h.app.world()),
            1,
        );
        assert_eq!(before.value(&remaining).unwrap(), 0.0);
        for (property, expected) in [("up", 0.0), ("down", 1.0), ("stack", 0.0)] {
            assert_eq!(
                before.value(&format!("{prefix}.{property}")).unwrap(),
                expected
            );
        }
        if prefix.starts_with("dot.") {
            assert_eq!(before.value(&format!("{prefix}.ticking")).unwrap(), 0.0);
        }
        crate::auras::apply_aura(
            h.app.world_mut(),
            &crate::auras::AuraApplication {
                caster: h.caster,
                target: h.npc,
                spell_id: spell,
                effect_mask: 1,
                source: crate::auras::AuraSource::Cast,
            },
        );
        let after = NativeFacts::new(
            h.app.world(),
            h.caster,
            &binding,
            encounter(h.app.world()),
            1,
        );
        assert!(after.value(&remaining).unwrap() > 0.0);
        for (property, expected) in [("up", 1.0), ("down", 0.0), ("stack", 1.0)] {
            assert_eq!(
                after.value(&format!("{prefix}.{property}")).unwrap(),
                expected
            );
        }
        if prefix.starts_with("dot.") {
            assert_eq!(after.value(&format!("{prefix}.ticking")).unwrap(), 1.0);
        }
    }
}

#[test]
fn arms_native_apl_owned_dot_expires_on_actual_native_clock() {
    let mut h = warrior();
    h.app.world_mut().entity_mut(h.npc).insert(Health {
        current: 100_000_000.0,
        max: 100_000_000.0,
    });
    let bindings = Bindings {
        buffs: [("rend_dot".into(), REND_DOT)].into(),
        ..Default::default()
    };
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target: h.npc,
            spell_id: REND_DOT,
            effect_mask: 1,
            source: crate::auras::AuraSource::Cast,
        },
    );
    let expires_at = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(h.npc)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == REND_DOT && aura.caster == h.caster)
        .unwrap()
        .expires_at
        .unwrap();
    let now = crate::spell_cast::now(h.app.world());
    let step = h.app.world().resource::<Time>().delta_secs_f64();
    assert!(expires_at > now && step > 0.0);
    let before = NativeFacts::new(
        h.app.world(),
        h.caster,
        &bindings,
        encounter(h.app.world()),
        1,
    );
    assert_eq!(before.value("dot.rend_dot.ticking").unwrap(), 1.0);
    h.advance(((expires_at - now) / step).ceil() as usize + 1);
    assert!(crate::spell_cast::now(h.app.world()) >= expires_at);
    for unit in [h.caster, h.npc] {
        assert!(
            crate::spell_cast::targets::is_alive(h.app.world(), unit),
            "native expiry, not death cleanup"
        );
    }
    let after = NativeFacts::new(
        h.app.world(),
        h.caster,
        &bindings,
        encounter(h.app.world()),
        1,
    );
    for (property, expected) in [
        ("remains", 0.0),
        ("ticking", 0.0),
        ("up", 0.0),
        ("down", 1.0),
        ("stack", 0.0),
    ] {
        assert_eq!(
            after.value(&format!("dot.rend_dot.{property}")).unwrap(),
            expected
        );
    }
}

#[test]
fn arms_native_apl_patchwerk_add_facts_use_simc_absent_event_values() {
    let h = warrior();
    let binding = Bindings::default();
    let facts = NativeFacts::new(
        h.app.world(),
        h.caster,
        &binding,
        encounter(h.app.world()),
        1,
    );
    // raid_event.cpp3067–3088: absent event in/cooldown uses timespan::max.
    assert_eq!(facts.value("raid_event.adds.exists").unwrap(), 0.0);
    assert_eq!(facts.value("raid_event.adds.remains").unwrap(), 0.0);
    assert_eq!(
        facts.value("raid_event.adds.in").unwrap(),
        i64::MAX as f64 / 1000.0
    );
    assert!(facts.value("raid_event.adds.missing").is_err());
}

#[test]
fn arms_native_apl_dungeon_add_timing_is_a_named_missing_fact() {
    let h = warrior();
    let binding = Bindings::default();
    let facts = NativeFacts::new(
        h.app.world(),
        h.caster,
        &binding,
        Encounter::Dungeon { started_at: 0.0 },
        1,
    );
    let error = facts.value("raid_event.adds.in").unwrap_err();
    assert!(error.starts_with("DUNGEON_TTD_UNAVAILABLE:"), "{error}");
}
