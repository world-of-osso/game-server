//! Actual Infernal earns Crashing Chaos uses for actual Chaos Bolts.
use super::*;
use crate::auras::ActiveAuras;
use crate::class_progression::TraitLoadout;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

fn stacks(run: &Run) -> Option<u8> {
    run.app
        .world()
        .get::<ActiveAuras>(run.actor)?
        .iter()
        .find(|aura| aura.spell_id == 417282 && aura.caster == run.actor)
        .map(|aura| aura.stacks)
}

fn paid_cast(run: &mut Run, spell: u32) {
    let actor = run.actor;
    let target = run.targets[0];
    let actual = literal_rotation::execution::resolve_action(run.app.world(), actor, spell)
        .unwrap()
        .expect("selected native action");
    literal_rotation::execution::execute(
        run.app.world_mut(),
        actor,
        literal_rotation::execution::Execution::Spell(actual, target, Some(target)),
    )
    .unwrap();
    preparation::finish_pending_setup(run).unwrap();
    for _ in 0..60 {
        run.app.update();
    }
}

fn first_bolt_damage(remove_earned_buff: bool) -> (i32, f64) {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(267, 1).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    engage(&mut run);
    paid_cast(&mut run, 1122);
    assert_eq!(stacks(&run), Some(8), "actual Infernal must earn the buff");
    let native_percent = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(417282)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .base_points;
    if remove_earned_buff {
        crate::auras::remove_auras(
            run.app.world_mut(),
            actor,
            crate::auras::RemoveReason::Consumed,
            |aura| aura.spell_id == 417282 && aura.caster == actor,
        );
    }
    paid_cast(&mut run, 116858);
    let hit = run
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .find(|event| {
            event.source == Some(actor.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(116858)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .expect("actual Chaos Bolt must land");
    (hit.amount, f64::from(native_percent))
}

#[test]
#[ignore = "requires private267 native player inputs"]
fn destruction_crashing_chaos_eight_uses_apply_one_native_damage_bonus() {
    let (plain, percent) = first_bolt_damage(true);
    let (buffed, _) = first_bolt_damage(false);
    let ratio = f64::from(buffed) / f64::from(plain);
    let expected = 1.0 + percent / 100.0;
    assert!(
        (ratio - expected).abs() < 0.02,
        "actual hit must receive one native Crashing Chaos bonus, not eight: plain={plain}, buffed={buffed}, expected ratio={expected}"
    );
}

#[test]
#[ignore = "requires private267 native player inputs"]
fn destruction_crashing_chaos_actual_infernal_earns_eight_chaos_bolt_uses() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(267, 1).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    assert!(
        run.app
            .world()
            .get::<TraitLoadout>(actor)
            .unwrap()
            .active_talent_spells()
            .any(|talent| talent.spell_id == 417234)
    );
    engage(&mut run);
    paid_cast(&mut run, 1122);
    // SimC76f8118 actions5080–5082 and init839–841: an actual Infernal
    // grants native eight reverse uses, not a manually granted proc buff.
    assert_eq!(
        stacks(&run),
        Some(8),
        "actual Infernal must earn Crashing Chaos"
    );
    for remaining in [7, 6] {
        let before = run
            .app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| {
                event.source == Some(actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(116858)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            })
            .count();
        paid_cast(&mut run, 116858);
        let after = run
            .app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| {
                event.source == Some(actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(116858)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            })
            .count();
        assert_eq!(after, before + 1, "paid Chaos Bolt must actually hit");
        assert_eq!(
            stacks(&run),
            Some(remaining),
            "each actual Chaos Bolt consumes one use"
        );
    }
}
