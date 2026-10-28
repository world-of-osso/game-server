//! Soul Reaper spends earned lesser-ghoul stacks.
use super::combat;
use bevy::prelude::*;
use combat::auras::ActiveAuras;
use combat::spell_cast::{self, script_registry::SpellScriptHook, scripts::AfterCastHook, summon::SummonedCreature, targets};
use combat::spell_info::SpellInfo;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::AuraRollAfterCast(after_cast)];

fn after_cast(spell: u32) -> Option<AfterCastHook> {
    (spell == 343294).then_some(spend_ready_ghouls)
}

fn spend_ready_ghouls(world: &mut World, caster: Entity, _: &SpellInfo) {
    // SimC76f8118 sc_death_knight.cpp12712–12724: consume Reaping and up to
    // native effect3 ready ghouls; each becomes a ghoul or actual Army orders.
    let reaping = world.get::<ActiveAuras>(caster).and_then(|auras| auras.iter()
        .find(|aura| aura.caster == caster && aura.spell_id == 1242654).map(|aura| aura.instance_id));
    if let Some(instance) = reaping { combat::auras::spend_stack(world, caster, instance); }
    let ready = world.get::<ActiveAuras>(caster).and_then(|auras| auras.iter()
        .find(|aura| aura.caster == caster && aura.spell_id == 1254252)
        .map(|aura| (aura.instance_id, aura.stacks)));
    let Some((instance, stacks)) = ready else { return; };
    let maximum = spell_cast::scripts::script_effect_value(world, caster, 343294, 2) as u8;
    for _ in 0..stacks.min(maximum) {
        combat::auras::spend_stack(world, caster, instance);
        if combat::auras::has_aura(world, caster, 42650) { order_army(world, caster); }
        else if !spell_cast::try_cast_triggered(world, caster, 275430, Some(caster), 1) {
            warn!(?caster, "Soul Reaper lesser ghoul summon refused");
        }
    }
}

fn order_army(world: &mut World, caster: Entity) {
    let Some(target) = world.get::<combat::networking::CombatTarget>(caster).and_then(|target| target.0) else {
        warn!(?caster, "Soul Reaper Army order has no target"); return;
    };
    let extra = combat::combat_refs::active_opponents(world, caster).filter(|enemy|
        *enemy != target && targets::is_alive(world, *enemy)).count();
    let spell = if extra + 1 >= 3 { 1300330 } else { 1294264 };
    let now = combat::auras::aura_now(world);
    let actors: Vec<_> = world.query::<(Entity, &SummonedCreature)>().iter(world)
        .filter_map(|(actor, summon)| (summon.owner == caster && summon.spell_id == 1294026
            && summon.expires_at.is_some_and(|expiry| expiry > now)).then_some(actor)).collect();
    for actor in actors {
        if !spell_cast::try_cast_triggered(world, actor, spell, Some(target), 1) {
            warn!(?actor, spell, "Soul Reaper Army order refused");
        }
    }
}
