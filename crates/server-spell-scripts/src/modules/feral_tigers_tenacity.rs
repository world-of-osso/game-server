//! Tiger's Fury grants three finisher-triggered Tiger's Tenacity charges.
use super::combat;
use bevy::prelude::*;
use combat::auras::{AuraInstance, procs::AuraEvent, scripts::AuraScripts};
use combat::spell_cast::{SpellHistory, script_registry::SpellScriptHook};
use combat::spell_info::SpellStore;
use shared::components::PowerType;

const TALENT: u32 = 391872;
const BUFF: u32 = 391873;
const TIGERS_FURY: u32 = 5217;

#[derive(Component)]
struct ConsumedFinisher {
    spell: u32,
    at: f64,
}

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::LateAuraRegistration(register)];

fn register(scripts: &mut AuraScripts) {
    scripts.0.entry(TALENT).or_default().check_proc = Some(tigers_fury_only);
    let buff = scripts.0.entry(BUFF).or_default();
    buff.check_proc = Some(finisher_only);
    buff.on_damage_proc = Some(record_finisher);
}

fn tigers_fury_only(_: &World, aura: &AuraInstance, event: &AuraEvent) -> bool {
    event.actor == aura.caster && event.spell_id == Some(TIGERS_FURY)
}

// SimC76f8118 sc_druid.cpp3815–3857 consumes one charge for a landed,
// foreground Cat finisher, regardless of how many targets that cast hits.
fn finisher_only(world: &World, aura: &AuraInstance, event: &AuraEvent) -> bool {
    if event.actor != aura.caster || event.periodic || !event.can_proc {
        return false;
    }
    let Some(spell_id) = event.spell_id else {
        return false;
    };
    let Some(spell) = world.resource::<SpellStore>().get(spell_id) else {
        return false;
    };
    if !spell
        .power_costs
        .iter()
        .any(|cost| cost.power_type == PowerType::ComboPoints as i32)
    {
        return false;
    }
    let Some(at) = world
        .get::<SpellHistory>(aura.caster)
        .and_then(|history| history.last_successful_cast_at(spell_id))
    else {
        return false;
    };
    !world
        .get::<ConsumedFinisher>(aura.caster)
        .is_some_and(|last| last.spell == spell_id && last.at == at)
}

fn record_finisher(world: &mut World, unit: Entity, _: &AuraInstance, event: &AuraEvent) {
    let spell = event.spell_id.expect("admitted finisher has a spell");
    let at = world
        .get::<SpellHistory>(unit)
        .and_then(|history| history.last_successful_cast_at(spell))
        .expect("admitted finisher has a successful cast");
    world
        .entity_mut(unit)
        .insert(ConsumedFinisher { spell, at });
}
