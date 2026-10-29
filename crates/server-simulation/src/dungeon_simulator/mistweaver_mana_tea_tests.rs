use shared::components::Mana;
use shared::protocol::CombatLogKind;

fn stacks(world: &bevy::prelude::World, unit: bevy::prelude::Entity) -> u8 {
    world
        .get::<crate::auras::ActiveAuras>(unit)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == 115867 && aura.caster == unit)
                .map(|aura| aura.stacks)
        })
        .unwrap_or(0)
}

#[test]
#[ignore = "requires the native MID2 Mistweaver profile and canonical DB"]
fn mana_tea_spending_earns_stacks_and_channel_consumes_them() {
    let mut run = super::super::dps_parity::prepare_registered_player(66, 1).unwrap();
    let actor = super::super::dps_parity::spawn_second_reference_player(&mut run, 270).unwrap();
    for _ in 0..5 {
        run.app.update();
    }
    assert!(crate::auras::has_aura(run.app.world(), actor, 115869));
    assert_eq!(stacks(run.app.world(), actor), 0);
    for _ in 0..8 {
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 399491, Some(actor), false)
            .unwrap();
        for _ in 0..80 {
            run.app.update();
        }
    }
    let earned = stacks(run.app.world(), actor);
    assert!(
        earned > 0,
        "actual paid healing casts must earn Mana Tea stacks"
    );
    assert_eq!(
        super::super::dps_parity::literal_rotation::execution::resolve_action(
            run.app.world(),
            actor,
            115294
        )
        .unwrap(),
        Some(115294)
    );
    // The earned stacks come from paid casts; leave room to observe restoration.
    {
        let mut mana = run.app.world_mut().get_mut::<Mana>(actor).unwrap();
        mana.current = mana.max * 0.5;
    }
    let before = run.app.world().get::<Mana>(actor).unwrap().current;
    let event_cursor = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .len();
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 115294, Some(actor), false)
        .unwrap();
    for _ in 0..80 {
        run.app.update();
    }
    let after = run.app.world().get::<Mana>(actor).unwrap().current;
    let remaining = stacks(run.app.world(), actor);
    assert!(remaining < earned);
    assert!(after > before);
    let spell = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(115294)
        .unwrap();
    let effect = spell
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap();
    let native_amount =
        crate::spell_cast::effects::modded_base_points(run.app.world(), actor, spell, effect)
            .trunc() as i32;
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0[event_cursor..];
    let ticks: Vec<_> = events
        .iter()
        .filter(|event| {
            event.source == Some(actor.to_bits())
                && event.spell_id == Some(115294)
                && event.kind == CombatLogKind::Energize
        })
        .collect();
    assert_eq!(ticks.len(), usize::from(earned - remaining));
    assert!(native_amount > 0);
    assert!(ticks.iter().all(|event| event.amount == native_amount));
}
