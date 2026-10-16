//! Source70391 primary-target admission, before costs and cooldowns.
use super::test_harness::*;
use shared::components::Health;
use shared::protocol::CombatLogKind;
use shared::spell_data::CastFailReason;

const TOUCH_OF_DEATH: u32 = 322109;
const IMPROVED_TOUCH_OF_DEATH: u32 = 322113;

fn cast_at_health(current: f32, maximum: f32, improved: bool) -> Harness {
    let ids = if improved {
        vec![TOUCH_OF_DEATH, IMPROVED_TOUCH_OF_DEATH]
    } else {
        vec![TOUCH_OF_DEATH]
    };
    let mut h = harness(Vec::new(), &ids, 10).unwrap();
    h.advance(3);
    assert_eq!(
        crate::auras::has_aura(h.app.world(), h.caster, IMPROVED_TOUCH_OF_DEATH),
        improved
    );
    h.app.world_mut().entity_mut(h.npc).insert(Health {
        current,
        max: maximum,
    });
    h.cast_and_deliver(TOUCH_OF_DEATH, Some(h.npc));
    h
}

fn apply_native_damage_modifiers(h: &mut Harness) {
    let data = load_native_creature_world(&[6]);
    let armor = data.creature_armor_at(6, 0, 10).unwrap();
    assert!(armor > 0.0);
    h.app.world_mut().entity_mut(h.npc).insert((
        crate::level_scaling::CreatureArmor(armor),
        shared::components::UnitLevel(10),
    ));
    let conn = rusqlite::Connection::open_with_flags(
        &world_db().unwrap().powers_db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(crate::world_data::load_expected_stats(&conn).armor_constants());
    for (target, spell) in [(h.caster, 8599), (h.npc, 871)] {
        crate::auras::apply_aura(
            h.app.world_mut(),
            &crate::auras::AuraApplication {
                caster: target,
                target,
                spell_id: spell,
                effect_mask: 1,
                source: crate::auras::AuraSource::Cast,
            },
        );
    }
    assert!(
        h.app
            .world()
            .get::<crate::auras::AuraStatMods>(h.caster)
            .unwrap()
            .damage_done(1)
            > 1.0
    );
    assert!(
        h.app
            .world()
            .get::<crate::auras::AuraStatMods>(h.npc)
            .unwrap()
            .damage_taken(1)
            < 1.0
    );
}

#[test]
fn touch_of_death_native_eligible_damage_uses_max_health_despite_armor_and_modifiers() {
    for (current, expected) in [(999.0, 0.0), (1400.0, 1050.0)] {
        let mut h = harness(
            Vec::new(),
            &[TOUCH_OF_DEATH, IMPROVED_TOUCH_OF_DEATH, 8599, 871],
            10,
        )
        .unwrap();
        h.advance(3);
        h.app.world_mut().entity_mut(h.npc).insert(Health {
            current,
            max: 10000.0,
        });
        h.app
            .world_mut()
            .entity_mut(h.caster)
            .insert(shared::components::UnitLevel(10));
        // Deliberately exercise Can'tCrit, not an RNG override or legal stat claim.
        h.app
            .world_mut()
            .get_mut::<crate::stats::CharacterStats>(h.caster)
            .unwrap()
            .crit_pct = 100.0;
        apply_native_damage_modifiers(&mut h);
        h.cast_and_deliver(TOUCH_OF_DEATH, Some(h.npc));
        assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
        assert_eq!(
            h.health(h.npc),
            expected,
            "native maxhealth damage; outcomes {:?}",
            h.inbox.log
        );
        assert!(
            h.inbox
                .log
                .iter()
                .any(|event| event.spell_id == Some(TOUCH_OF_DEATH)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
                    && !event.crit)
        );
    }
}

#[test]
fn touch_of_death_native_ineligible_target_rejects_without_success_or_cooldown() {
    for improved in [false, true] {
        let h = cast_at_health(5000.0, 5000.0, improved);
        assert!(
            h.inbox.failed.iter().any(|failure| {
                failure.spell_id == TOUCH_OF_DEATH
                    && failure.reason == CastFailReason::InvalidTarget
            }),
            "healthy stronger target must fail: {:?}",
            h.inbox.failed
        );
        assert!(h.inbox.log.iter().all(|event| {
            event.spell_id != Some(TOUCH_OF_DEATH) || event.kind != CombatLogKind::CastSuccess
        }));
        assert!(
            h.inbox
                .cooldowns
                .iter()
                .all(|event| event.spell_id != TOUCH_OF_DEATH)
        );
        assert_eq!(h.health(h.npc), 5000.0);
    }
}

#[test]
fn touch_of_death_native_stronger_creature_needs_improved_rank_and_below_threshold() {
    for (current, improved, allowed) in [
        (1400.0, false, false),
        (1400.0, true, true),
        (1500.0, true, false),
        (1600.0, true, false),
        (999.0, false, true),
    ] {
        let h = cast_at_health(current, 10000.0, improved);
        let succeeded = h.inbox.log.iter().any(|event| {
            event.spell_id == Some(TOUCH_OF_DEATH) && event.kind == CombatLogKind::CastSuccess
        });
        assert_eq!(
            succeeded, allowed,
            "health={current}, improved={improved}, failures={:?}",
            h.inbox.failed
        );
    }
}
