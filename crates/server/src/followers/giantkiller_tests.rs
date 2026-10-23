//! Native362980 amounts; target scaling policy: SimC76f8118 sc_evoker.cpp4613–4629.
use crate::auras::{ActiveAuras, RemoveReason, has_aura, remove_auras};
use crate::spell_cast::test_harness::Harness;
use bevy::prelude::*;
use shared::components::Health;
use shared::protocol::CombatLogKind;

const GIANTKILLER: u32 = 362980;
const TYRANNY: u32 = 376888;
const DRAGONRAGE: u32 = 375087;
const LIVING_FLAME_DAMAGE: u32 = 361500;
const DISINTEGRATE: u32 = 356995;
const FROSTBOLT_DAMAGE: u32 = 228597;

struct Case {
    spell: u32,
    health_fraction: f32,
    removed: &'static [u32],
    dragonrage: bool,
}

impl Case {
    fn flame(health_fraction: f32) -> Self {
        Self {
            spell: LIVING_FLAME_DAMAGE,
            health_fraction,
            removed: &[],
            dragonrage: false,
        }
    }
}

fn native_hit(case: Case) -> (i32, f32) {
    let (mut h, caster) = super::kit_tests::at_level(215240, 90, |_| {});
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .remove::<crate::combat::AutoAttack>();
    assert!(has_aura(h.app.world(), caster, TYRANNY));
    let mastery = h
        .app
        .world()
        .get::<ActiveAuras>(caster)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == GIANTKILLER)
        .expect("real Devastation follower retains its native mastery aura")
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .amount;
    assert!(mastery > 0.0);
    for &spell in case.removed {
        remove_auras(h.app.world_mut(), caster, RemoveReason::Unlearned, |aura| {
            aura.spell_id == spell
        });
        assert!(!has_aura(h.app.world(), caster, spell));
    }
    if case.dragonrage {
        crate::spell_cast::cast_triggered(h.app.world_mut(), caster, DRAGONRAGE, Some(caster), 0);
        assert!(has_aura(h.app.world(), caster, DRAGONRAGE));
    }
    let mut health = h.app.world_mut().get_mut::<Health>(h.npc).unwrap();
    health.current = health.max * case.health_fraction;
    // Reset only the RNG stream, never native crit/coefficients/stats. Each paired
    // cast sees the same actual crit roll and Living Flame variance.
    let _seed = crate::random::seed_scope(70448);
    crate::spell_cast::cast_triggered(h.app.world_mut(), caster, case.spell, Some(h.npc), 0);
    h.advance(40);
    let hit = first_hit(&h, caster, case.spell);
    (hit, mastery)
}

fn first_hit(h: &Harness, caster: Entity, spell: u32) -> i32 {
    h.inbox
        .log
        .iter()
        .find(|event| {
            event.kind == CombatLogKind::Damage
                && event.spell_id == Some(spell)
                && event.source == Some(caster.to_bits())
                && event.target == Some(h.npc.to_bits())
        })
        .unwrap_or_else(|| {
            panic!(
                "native spell{spell} must actually damage its target: {:?}",
                h.inbox.log
            )
        })
        .amount
}

fn assert_multiplier(actual: i32, base: i32, mastery: f32, fraction: f32) {
    let expected = base as f32 * (1.0 + mastery * fraction / 100.0);
    assert!(
        (actual as f32 - expected).abs() <= 3.0,
        "native damage{actual}, baseline{base}, mastery{mastery}%, fraction{fraction}: expected{expected}"
    );
}

#[test]
#[ignore = "requires frozen native follower/spell inputs; future Giantkiller damage policy"]
fn giantkiller_native_direct_damage_tracks_target_health_with_thirty_percent_floor() {
    let (base, _) = native_hit(Case {
        removed: &[GIANTKILLER],
        ..Case::flame(1.0)
    });
    for (health, fraction) in [(1.0, 1.0), (0.5, 0.5), (0.1, 0.3)] {
        let (actual, mastery) = native_hit(Case::flame(health));
        assert_multiplier(actual, base, mastery, fraction);
    }
}

#[test]
#[ignore = "requires frozen native follower/spell inputs; future Giantkiller damage policy"]
fn giantkiller_native_tyranny_requires_dragonrage_for_ordinary_flame() {
    let (base, _) = native_hit(Case {
        removed: &[GIANTKILLER],
        dragonrage: true,
        ..Case::flame(0.1)
    });
    let (full, mastery) = native_hit(Case {
        dragonrage: true,
        ..Case::flame(0.1)
    });
    assert_multiplier(full, base, mastery, 1.0);
    let (ordinary, mastery) = native_hit(Case {
        removed: &[TYRANNY],
        dragonrage: true,
        ..Case::flame(0.1)
    });
    assert_multiplier(ordinary, base, mastery, 0.3);
}

#[test]
#[ignore = "requires frozen native follower/spell inputs; future Giantkiller damage policy"]
fn giantkiller_native_periodic_damage_scales_but_other_class_spells_do_not() {
    let (base, _) = native_hit(Case {
        spell: DISINTEGRATE,
        removed: &[GIANTKILLER],
        ..Case::flame(1.0)
    });
    let (actual, mastery) = native_hit(Case {
        spell: DISINTEGRATE,
        ..Case::flame(1.0)
    });
    assert_multiplier(actual, base, mastery, 1.0);
    let (other_base, _) = native_hit(Case {
        spell: FROSTBOLT_DAMAGE,
        removed: &[GIANTKILLER],
        ..Case::flame(1.0)
    });
    let (other, _) = native_hit(Case {
        spell: FROSTBOLT_DAMAGE,
        ..Case::flame(1.0)
    });
    assert_eq!(
        other, other_base,
        "Giantkiller must not amplify another class's spell"
    );
}

#[test]
#[ignore = "requires frozen native follower/spell inputs; future Giantkiller Tyranny exceptions"]
fn giantkiller_native_tyranny_preserves_full_mastery_on_named_secondary_damage() {
    // Pinned sc_evoker.cpp5721/6165/7449 override use_full_mastery for these
    // native children even after their initiating Dragonrage/Deep Breath ends.
    for spell in [353759, 441172, 1292322] {
        let (base, _) = native_hit(Case {
            spell,
            removed: &[GIANTKILLER],
            ..Case::flame(0.1)
        });
        let (actual, mastery) = native_hit(Case {
            spell,
            ..Case::flame(0.1)
        });
        assert_multiplier(actual, base, mastery, 1.0);
        let (ordinary, mastery) = native_hit(Case {
            spell,
            removed: &[TYRANNY],
            ..Case::flame(0.1)
        });
        assert_multiplier(ordinary, base, mastery, 0.3);
    }
}
