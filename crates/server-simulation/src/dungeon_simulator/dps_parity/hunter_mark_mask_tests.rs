//! An earned Sentinel mark enhances only the native spell-family effects it names.
use super::marksmanship_mechanics_tests::{cast, player};
use super::*;
use shared::protocol::CombatLogKind;

const SENTINELS_MARK: u32 = 1253601;

#[derive(Debug)]
struct MarkedHit {
    damage: f64,
    critical: bool,
    aimed_percent: f64,
}

fn shoot_after_earned_mark(spell: u32, keep_mark: bool) -> MarkedHit {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = player(1);
    cast(&mut run, 288613, true);
    // Real selected Feathered Frenzy produces the mark; do not inject a
    // synthetic aura or damage value into either side of this comparison.
    let target = run.targets[0];
    let aimed_percent = read_aimed_mark_percent(&run);
    if !keep_mark {
        crate::auras::remove_auras(
            run.app.world_mut(),
            target,
            crate::auras::RemoveReason::Cancelled,
            |aura| aura.spell_id == SENTINELS_MARK && aura.caster == run.actor,
        );
    }
    cast(&mut run, spell, false);
    advance_until_shot_damage(&mut run, spell, aimed_percent)
}

fn read_aimed_mark_percent(run: &Run) -> f64 {
    let auras = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(run.targets[0])
        .unwrap();
    let mark = auras
        .iter()
        .find(|aura| aura.spell_id == SENTINELS_MARK && aura.caster == run.actor)
        .expect("actual Trueshot must earn this caster's Sentinel mark");
    let amount = mark
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("native mark retains its Aimed damage effect")
        .amount;
    assert!(
        amount > 0.0,
        "native selected mark has a positive Aimed bonus"
    );
    f64::from(amount)
}

fn advance_until_shot_damage(run: &mut Run, spell: u32, aimed_percent: f64) -> MarkedHit {
    let target = run.targets[0];
    for _ in 0..100 {
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        if let Some(event) = events.iter().find(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(spell)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        }) {
            return MarkedHit {
                damage: f64::from(event.amount),
                critical: event.crit,
                aimed_percent,
            };
        }
        run.app.update();
    }
    panic!("actual native spell{spell} never damaged its primary");
}

#[test]
#[ignore = "requires private exact254 profile/traits and native world data"]
fn native_mm_earned_sentinel_mark_leaves_auto_and_steady_damage_unchanged() {
    // SimC76f8118 sc_hunter.cpp1278/1493–1494 applies the mark only when
    // that action's parsed native damage-effect mask matches. Neither shot matches.
    for spell in [75, 56641] {
        let marked = shoot_after_earned_mark(spell, true);
        let unmarked = shoot_after_earned_mark(spell, false);
        assert_eq!(
            marked.critical, unmarked.critical,
            "same seeded native hit outcome"
        );
        let ratio = marked.damage / unmarked.damage;
        assert!(
            (ratio - 1.0).abs() < 0.001,
            "spell{spell}: earned mark cannot multiply unrelated damage: marked={marked:?}, unmarked={unmarked:?}, ratio={ratio}"
        );
    }
}

#[test]
#[ignore = "requires private exact254 profile/traits and native world data"]
fn native_mm_earned_sentinel_mark_keeps_only_aimeds_native_effect_bonus() {
    // SimC76f8118 sc_hunter.cpp1278/1493–1494: Aimed matches effect1;
    // the independent Wildfire effect must not multiply that same Aimed hit.
    let marked = shoot_after_earned_mark(19434, true);
    let unmarked = shoot_after_earned_mark(19434, false);
    assert_eq!(
        marked.critical, unmarked.critical,
        "same seeded native Aimed outcome"
    );
    let expected = 1.0 + marked.aimed_percent / 100.0;
    let ratio = marked.damage / unmarked.damage;
    assert!(
        (ratio - expected).abs() < 0.001,
        "only the actual native Aimed mark effect applies: marked={marked:?}, unmarked={unmarked:?}, ratio={ratio}, expected={expected}"
    );
}
