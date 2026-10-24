//! Opt-in passive diagnostic at the existing interrupt/removal boundaries.
use bevy::prelude::*;
use serde_json::{Value, json};

use crate::auras::{ActiveAuras, AuraInstance, RemoveReason};

#[derive(Resource)]
struct RemovalProbe {
    unit: Entity,
    spell: u32,
    interrupt: Option<Value>,
    removed: Vec<Value>,
}

pub fn start_removal_probe(world: &mut World, unit: Entity, spell: u32) {
    world.insert_resource(RemovalProbe {
        unit,
        spell,
        interrupt: None,
        removed: Vec::new(),
    });
}

pub(in crate::auras) fn observe_interrupt(
    world: &mut World,
    unit: Entity,
    flags: [u32; 2],
    except_spell: Option<u32>,
) {
    let Some(probe) = world.get_resource::<RemovalProbe>() else {
        return;
    };
    if probe.unit != unit || Some(probe.spell) == except_spell {
        return;
    }
    let Some(aura) = world
        .get::<ActiveAuras>(unit)
        .and_then(|auras| auras.iter().find(|aura| aura.spell_id == probe.spell))
    else {
        return;
    };
    let matched = [
        aura.interrupt_flags[0] & flags[0],
        aura.interrupt_flags[1] & flags[1],
    ];
    if matched == [0; 2] {
        return;
    }
    let receipt = json!({"requested_flags": flags, "aura_flags": aura.interrupt_flags,
        "matched_flags": matched, "causing_spell": except_spell});
    world.resource_mut::<RemovalProbe>().interrupt = Some(receipt);
}

pub(super) fn record_removed(
    world: &mut World,
    unit: Entity,
    aura: &AuraInstance,
    reason: RemoveReason,
) {
    let Some(probe) = world.get_resource::<RemovalProbe>() else {
        return;
    };
    if probe.unit != unit || probe.spell != aura.spell_id {
        return;
    }
    let receipt = json!({
        "time": crate::spell_cast::now(world), "unit": unit.to_bits(),
        "spell": aura.spell_id, "reason": format!("{reason:?}"),
        "interrupt": probe.interrupt,
        "position": world.get::<shared::components::Position>(unit),
        "moving": world.get::<crate::networking::Moving>(unit).map(|moving| moving.0),
        "in_combat": world.get::<shared::components::CombatStatus>(unit).map(|status| status.0),
        "auto_attack": world.get::<crate::combat::AutoAttack>(unit).map(|attack| json!({
            "victim": attack.victim.to_bits(), "timer_ms": attack.timer.as_millis(),
            "off_hand_timer_ms": attack.off_hand_timer.as_millis()})),
        "last_successful_cast": world.get::<crate::spell_cast::SpellHistory>(unit)
            .and_then(|history| history.last_successful_cast()),
    });
    let mut probe = world.resource_mut::<RemovalProbe>();
    probe.removed.push(receipt);
    probe.interrupt = None;
}

pub fn removal_probe_snapshot(world: &World) -> Value {
    let probe = world.resource::<RemovalProbe>();
    json!({"unit": probe.unit.to_bits(), "spell": probe.spell, "removed": probe.removed})
}
