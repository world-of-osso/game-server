//! Literal target placement must reach the ordinary native destination request.
use super::super::*;
use crate::spell_cast::test_harness::{harness, spawn_player};
use server::simc_apl::Action;
use shared::protocol::CombatLogKind;

fn empty_equipment() -> Equipment {
    Equipment {
        slots: SLOTS
            .iter()
            .map(|&(name, slot)| (name.into(), Slot::empty(slot)))
            .collect(),
        equipped: Default::default(),
        names: Default::default(),
        set_facts: Default::default(),
    }
}

#[test]
#[ignore = "requires native private world DB: actual Healing Rain destination and pulses"]
fn literal_destination_heals_at_selected_unit_not_at_the_caster() {
    let mut harness = harness(Vec::new(), &[73920, 73921], 7).unwrap();
    let caster = harness.caster;
    let inside = spawn_player(&mut harness.app, 1, 21.0);
    let outside = spawn_player(&mut harness.app, 1, 41.0);
    harness.advance(4);
    for target in [caster, inside, outside] {
        let mut health = harness
            .app
            .world_mut()
            .get_mut::<shared::components::Health>(target)
            .unwrap();
        health.current = health.max / 2.0;
    }
    let equipment = empty_equipment();
    let prepared = super::super::super::execution::Prepared::read(
        harness.app.world_mut(),
        caster,
        &[inside],
        &equipment,
        [super::super::super::execution::NativeAction {
            name: "healing_rain",
            spell: 73920,
            self_target: false,
            talent: None,
        }],
        false,
    )
    .unwrap();
    let action = Action {
        source_row: None,
        name: "healing_rain".into(),
        target: None,
        options: Default::default(),
    };
    let execution = prepared
        .execution(&action, &equipment)
        .unwrap()
        .expect("native placed Healing Rain ready");
    super::super::super::execution::execute(harness.app.world_mut(), caster, execution).unwrap();
    harness.advance(130);
    let heals: Vec<_> = harness
        .inbox
        .log
        .iter()
        .filter(|event| {
            event.spell_id == Some(73921) && event.kind == CombatLogKind::Heal && event.amount > 0
        })
        .collect();
    assert!(
        heals
            .iter()
            .any(|event| event.target == Some(inside.to_bits())),
        "native pulses must heal the selected destination ally"
    );
    assert!(
        !heals
            .iter()
            .any(|event| event.target == Some(outside.to_bits())),
        "native radius must exclude the distant ally"
    );
    assert!(
        !heals
            .iter()
            .any(|event| event.target == Some(caster.to_bits())),
        "the selected destination is not the caster position"
    );
}

#[test]
#[ignore = "requires native private world DB: actual Havoc Metamorphosis destination"]
fn literal_metamorphosis_admits_landing_damage_at_selected_destination() {
    use super::super::super::execution::{NativeAction, Prepared, execute};
    let mut harness = harness(Vec::new(), &[191427, 200166, 247121, 162264, 201453], 12).unwrap();
    harness
        .app
        .insert_resource(crate::test_data::world_data().clone());
    let caster = harness.caster;
    let target = harness.npc;
    harness
        .app
        .world_mut()
        .get_mut::<shared::components::Position>(target)
        .unwrap()
        .x = 21.0;
    harness.advance(4);
    let equipment = empty_equipment();
    let prepared = Prepared::read(
        harness.app.world_mut(),
        caster,
        &[target],
        &equipment,
        [NativeAction {
            name: "metamorphosis",
            spell: 191427,
            self_target: false,
            talent: None,
        }],
        false,
    )
    .unwrap();
    let action = Action {
        source_row: None,
        name: "metamorphosis".into(),
        target: None,
        options: Default::default(),
    };
    let execution = prepared
        .execution(&action, &equipment)
        .unwrap()
        .expect("native cursor Metamorphosis ready");
    execute(harness.app.world_mut(), caster, execution).unwrap();
    harness.advance(100);
    assert!(
        harness.inbox.log.iter().any(|event| {
            event.source == Some(caster.to_bits())
                && event.spell_id == Some(191427)
                && event.kind == CombatLogKind::CastSuccess
        }),
        "ordinary native Metamorphosis succeeds"
    );
    // Native jump254 progress-curve displacement is separately unsupported.
    // The existing landing payload must still reach the selected cursor point.
    assert!(
        harness.inbox.log.iter().any(|event| {
            event.target == Some(target.to_bits())
                && event.spell_id == Some(200166)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        }),
        "native landing damage reaches the selected destination, outside caster radius"
    );
}
