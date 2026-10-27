//! Actual Chaos Bolts earn Chaotic Inferno; its Incinerate is instant.
use super::*;
use crate::class_progression::TraitLoadout;
use shared::components::PowerType;

const BUFF: u32 = 1244860;

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

fn shards(run: &Run) -> f32 {
    crate::spell_cast::powers::power_value(run.app.world(), run.actor, PowerType::SoulShards)
        .unwrap()
        .0
}

fn fund_one_bolt(run: &mut Run) {
    for _ in 0..20 {
        if shards(run) >= 20.0 {
            return;
        }
        paid_cast(run, 29722);
    }
    panic!("actual paid Incinerates must fund a Chaos Bolt");
}

fn incinerate_cast_ms(run: &Run) -> i32 {
    let world = run.app.world();
    let spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(29722)
        .unwrap();
    crate::spell_cast::cast_time_ms(world, run.actor, spell)
}

#[test]
#[ignore = "requires private267 native player inputs"]
fn destruction_chaotic_inferno_actual_chaos_bolt_earns_instant_incinerate() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(267, 1).unwrap();
    let actor = run.actor;
    assert!(
        run.app
            .world()
            .get::<TraitLoadout>(actor)
            .unwrap()
            .active_talent_spells()
            .any(|talent| talent.spell_id == 1244788)
    );
    engage(&mut run);
    assert!(incinerate_cast_ms(&run) > 0);
    // SimC76f8118 actions4391–4398, init1331–1334: native 25% accumulated
    // PRD guarantees a proc within twelve actual Chaos Bolt executions.
    let mut earned = false;
    for _ in 0..12 {
        fund_one_bolt(&mut run);
        paid_cast(&mut run, 116858);
        if crate::auras::has_aura(run.app.world(), actor, BUFF) {
            earned = true;
            break;
        }
    }
    assert!(earned, "actual Chaos Bolts must earn Chaotic Inferno");
    assert_eq!(
        incinerate_cast_ms(&run),
        0,
        "Chaotic Inferno makes the next Incinerate instant"
    );
    paid_cast(&mut run, 29722);
    assert!(
        !crate::auras::has_aura(run.app.world(), actor, BUFF),
        "the instant Incinerate consumes Chaotic Inferno"
    );
    assert!(incinerate_cast_ms(&run) > 0);
}
