//! SimC76f8118 sc_priest.cpp387–394/4358–4391: earned Rift extension cap.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, ActiveAuras};
use combat::spell_cast::{self, script_registry::SpellScriptHook};
use combat::spell_info::SpellInfo;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::NativeAfterCast(after_cast)];
const VOID_BLAST: u32 = 450983;
const TALENT: u32 = 449912;
const RIFT: u32 = 459314;
const VOIDHEART: u32 = 449887;

#[derive(Component)]
struct Extensions {
    rift: u32,
    paid: u8,
}

fn after_cast(spell: u32) -> Option<spell_cast::scripts::AfterCastHook> {
    (spell == VOID_BLAST).then_some(extend_rift)
}

fn extend_rift(world: &mut World, caster: Entity, _: &SpellInfo) {
    if !auras::has_aura(world, caster, TALENT) {
        return;
    }
    let Some(instance) = read_rift_instance(world, caster) else {
        return;
    };
    let cap = spell_cast::scripts::script_effect_value(world, caster, TALENT, 1) as u8;
    let seconds = spell_cast::scripts::script_effect_value(world, caster, TALENT, 2);
    if !spend_extension(world, caster, instance, cap) {
        return;
    }
    // The existing owned-duration operation preserves pending periodic deadlines.
    auras::extend_owned_duration(
        world,
        caster,
        caster,
        &[RIFT, VOIDHEART],
        (seconds * 1000.0) as u32,
    );
}

fn read_rift_instance(world: &World, caster: Entity) -> Option<u32> {
    let now = spell_cast::now(world);
    world
        .get::<ActiveAuras>(caster)?
        .iter()
        .find(|aura| {
            let owned = aura.caster == caster && aura.spell_id == RIFT;
            owned && aura.expires_at.is_some_and(|expiry| expiry > now)
        })
        .map(|aura| aura.instance_id)
}

fn spend_extension(world: &mut World, caster: Entity, rift: u32, cap: u8) -> bool {
    let reset = world
        .get::<Extensions>(caster)
        .is_none_or(|state| state.rift != rift);
    if reset {
        world
            .entity_mut(caster)
            .insert(Extensions { rift, paid: 0 });
    }
    let mut state = world.get_mut::<Extensions>(caster).unwrap();
    if state.paid >= cap {
        return false;
    }
    state.paid += 1;
    true
}
