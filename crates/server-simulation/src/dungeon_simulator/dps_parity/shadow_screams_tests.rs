//! SimC76f8118 sc_priest_shadow.cpp1324–1327/2203–2204; sc_priest.hpp1236.
//! Native375767 supplies393919's period amount; observe actual VT ticks, not timer fields.
use super::*;
use crate::auras::{ActiveAuras, RemoveReason};
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

const SCREAMS: u32 = 393919;
const MADNESS: u32 = 335467;
const VT: u32 = 34914;

fn owned_screams(run: &Run) -> Option<&crate::auras::AuraInstance> {
    run.app
        .world()
        .get::<ActiveAuras>(run.actor)?
        .iter()
        .find(|aura| aura.caster == run.actor && aura.spell_id == SCREAMS)
}

fn cast_buff(run: &mut Run) {
    crate::spell_cast::cast_triggered(run.app.world_mut(), run.actor, SCREAMS, Some(run.actor), 1);
}

fn owned_vt_hit(event: &shared::protocol::CombatLogEvent, actor: Entity, target: Entity) -> bool {
    let owned = event.source == Some(actor.to_bits()) && event.target == Some(target.to_bits());
    let vt_damage =
        event.spell_id == Some(VT) && event.kind == CombatLogKind::Damage && event.amount > 0;
    owned && vt_damage
}

fn collect_vt_ticks(run: &mut Run, count: usize, refresh_screams: bool) -> Vec<f64> {
    let mut cursor = run.app.world().resource::<ObservedCombatEvents>().0.len();
    let mut ticks = Vec::new();
    for frame in 0..240 {
        if refresh_screams && frame % 40 == 0 {
            cast_buff(run);
        }
        run.app.update();
        let events = &run.app.world().resource::<ObservedCombatEvents>().0;
        let hits = events[cursor..]
            .iter()
            .filter(|event| owned_vt_hit(event, run.actor, run.targets[0]))
            .count();
        let now = crate::spell_cast::now(run.app.world());
        ticks.extend(std::iter::repeat_n(now, hits));
        cursor = events.len();
        if ticks.len() >= count {
            return ticks;
        }
    }
    panic!(
        "native VT paid only{} of{count} expected ticks",
        ticks.len()
    );
}

fn last_interval(ticks: &[f64]) -> f64 {
    ticks[ticks.len() - 1] - ticks[ticks.len() - 2]
}

#[test]
#[ignore = "requires native258 Player/profile/traits"]
fn shadow_screams_real_slam_background_madness_produces_and_extends_native_buff() {
    let mut run = prepare_literal(258, 1).unwrap();
    engage_fixture_opponents(&mut run);
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 375767));
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(1227280, run.targets[0], None),
    )
    .unwrap();
    for _ in 0..20 {
        run.app.update();
    }
    let aura = owned_screams(&run).expect("Slam's real weak Madness must trigger selected Screams");
    let previous_expiry = aura.expires_at.unwrap();
    let fresh_duration = f64::from(aura.duration_ms) / 1000.0;
    assert!(
        aura.effects
            .iter()
            .any(|effect| effect.index == 0 && effect.amount < 0.0),
        "native flat-label talent supplies the negative period modifier"
    );
    for _ in 0..20 {
        run.app.update();
    }
    crate::spell_cast::cast_triggered(
        run.app.world_mut(),
        run.actor,
        MADNESS,
        Some(run.targets[0]),
        1,
    );
    let expiry = owned_screams(&run).unwrap().expires_at.unwrap();
    assert!(
        (expiry - previous_expiry - fresh_duration).abs() < 0.001,
        "EXTEND adds a fresh native duration; never replace it with now+duration"
    );
}

#[test]
#[ignore = "requires native258 Player/profile/traits"]
fn shadow_screams_native_period_modifier_changes_existing_vt_ticks_and_restores_on_removal() {
    let mut run = prepare_literal(258, 1).unwrap();
    engage_fixture_opponents(&mut run);
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(VT, run.targets[0], None),
    )
    .unwrap();
    let baseline = last_interval(&collect_vt_ticks(&mut run, 2, false));
    let fast = last_interval(&collect_vt_ticks(&mut run, 4, true));
    let amount = owned_screams(&run)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .amount;
    assert!(amount < 0.0, "native375767 modifies native393919 E0");
    let expected_fast = baseline * (1.0 + f64::from(amount) / 100.0);
    assert!(
        (fast - expected_fast).abs() < 0.15,
        "existing native VT must change from{baseline}s to{expected_fast}s, observed{fast}s"
    );
    crate::auras::remove_auras(
        run.app.world_mut(),
        run.actor,
        RemoveReason::Cancelled,
        |aura| aura.caster == run.actor && aura.spell_id == SCREAMS,
    );
    let restored = last_interval(&collect_vt_ticks(&mut run, 3, false));
    assert!(
        (restored - baseline).abs() < 0.15,
        "removal must restore actual native VT interval{baseline}s, observed{restored}s"
    );
}
