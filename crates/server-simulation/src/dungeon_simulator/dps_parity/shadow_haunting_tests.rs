//! SimC76f8118 sc_priest_shadow.cpp427–435: actual apparition impacts extend owned DoTs.
use super::literal_rotation::execution::{self, Execution};
use super::*;
use crate::auras::ActiveAuras;
use crate::spell_cast::{read_native_cast_start, test_harness::ObservedCombatEvents, CastFailure};
use crate::spell_info::SpellStore;
use shared::protocol::CombatLogKind;

fn cast_when_ready(run: &mut Run, id: u32) {
    let native = run
        .app
        .world()
        .resource::<SpellStore>()
        .get(id)
        .unwrap()
        .clone();
    for _ in 0..400 {
        let ready = read_native_cast_start(run.app.world_mut(), run.actor, &native, run.targets[0]);
        match ready {
            Ok(_) => {
                let command = Execution::Spell(id, run.targets[0], None);
                execution::execute(run.app.world_mut(), run.actor, command).unwrap();
                return;
            }
            Err(
                CastFailure::OnGlobalCooldown
                | CastFailure::OnCooldown
                | CastFailure::NoCharges
                | CastFailure::InProgress,
            ) => run.app.update(),
            Err(reason) => panic!("ordinary spell{id}: {reason:?}"),
        }
    }
    panic!("ordinary spell{id} never became ready");
}

fn deadline(run: &Run, spell: u32) -> Option<f64> {
    run.app
        .world()
        .get::<ActiveAuras>(run.targets[0])?
        .iter()
        .find(|aura| aura.caster == run.actor && aura.spell_id == spell)
        .and_then(|aura| aura.expires_at)
}

fn apparition_hits(run: &Run) -> usize {
    run.app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            let own = event.source == Some(run.actor.to_bits());
            let recipient = event.target == Some(run.targets[0].to_bits());
            let apparition = matches!(event.spell_id, Some(413231 | 1264176));
            let damage = event.kind == CombatLogKind::Damage && event.amount > 0;
            let source = own && recipient;
            source && apparition && damage
        })
        .count()
}

#[test]
#[ignore = "requires native258 Player/profile/traits"]
fn shadow_haunting_actual_apparition_impact_extends_existing_pain_and_vt() {
    let mut run = prepare_literal(258, 1).unwrap();
    engage(&mut run);
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 1279355));
    let milliseconds =
        crate::spell_cast::scripts::script_effect_value(run.app.world(), run.actor, 1279355, 1);
    assert_eq!(milliseconds, 1000.0);
    cast_when_ready(&mut run, 589);
    cast_when_ready(&mut run, 1227280);
    for _ in 0..100 {
        if deadline(&run, 34914).is_some() {
            break;
        }
        run.app.update();
    }
    let pain_before = deadline(&run, 589).expect("ordinary Pain is live");
    let vt_before =
        deadline(&run, 34914).expect("real Slam applies native VT before its apparition");
    assert_eq!(
        apparition_hits(&run),
        0,
        "capture DoTs before the real projectile arrives"
    );
    for _ in 0..100 {
        run.app.update();
        if apparition_hits(&run) > 0 {
            break;
        }
    }
    let impacts = apparition_hits(&run);
    assert!(
        impacts > 0,
        "real Slam's travelling apparition must land damage"
    );
    let seconds = f64::from(milliseconds) / 1000.0;
    for (spell, before) in [(589, pain_before), (34914, vt_before)] {
        let expected = before + seconds * impacts as f64;
        let observed = deadline(&run, spell).unwrap();
        assert!((observed - expected).abs() < 0.001,
            "native Haunting extends spell{spell} once per actual impact: observed={observed}, expected={expected}, impacts={impacts}");
    }
}
