//! Stormbound Emblem of Dazar grants its native haste after channel ticks.
use super::combat;
use bevy::prelude::*;
use combat::auras::scripts::AuraScripts;
use combat::auras::{self, ActiveAuras, AuraApplication, AuraInstance, AuraSource, RemoveReason};
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_info::SpellStore;

const CHANNEL: u32 = 1295275;
const EQUIPPED: u32 = 1294744;
const HASTE: u32 = 1294745;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::LateAuraRegistration(register)];

#[derive(Component, Default)]
struct StormboundTicks(u32);

fn register(scripts: &mut AuraScripts) {
    let script = scripts.0.entry(CHANNEL).or_default();
    script.on_apply = Some(start);
    script.on_periodic = Some(tick);
    script.on_remove = Some(finish);
}

fn start(world: &mut World, owner: Entity, aura: &AuraInstance) {
    if aura.caster == owner { world.entity_mut(owner).insert(StormboundTicks::default()); }
}

fn tick(world: &mut World, owner: Entity, aura: &AuraInstance, index: u8) {
    if aura.caster == owner && index == 0 {
        if let Some(mut ticks) = world.get_mut::<StormboundTicks>(owner) { ticks.0 += 1; }
    }
}

// SimC76f8118 unique_gear_midnight.cpp3863–3954: each completed 0.5s
// channel tick supplies one native haste-buff duration, including interruptions.
fn finish(world: &mut World, owner: Entity, aura: &AuraInstance, _: RemoveReason) {
    if aura.caster != owner { return; }
    let ticks = world.get::<StormboundTicks>(owner).map_or(0, |ticks| ticks.0);
    world.entity_mut(owner).remove::<StormboundTicks>();
    if ticks == 0 { return; }
    let root = world.get::<ActiveAuras>(owner).and_then(|auras| auras.iter()
        .find(|aura| aura.spell_id == EQUIPPED && aura.caster == owner && matches!(aura.source, AuraSource::Equipment(_))))
        .expect("Stormbound channel requires its equipped native item").clone();
    let amount = root.effects.iter().find(|effect| effect.index == 0)
        .expect("Stormbound equipped item retains its scaled amount").amount;
    let mut buff = world.resource::<SpellStore>().get(HASTE).expect("native Stormbound haste").clone();
    buff.cast_item_level = root.cast_item_level;
    buff.effects.iter_mut().find(|effect| effect.index == 0).unwrap().base_points = amount;
    let duration = buff.misc.as_ref().and_then(|misc| misc.duration.as_ref())
        .expect("native Stormbound haste duration").duration_ms;
    auras::apply_aura_with_spell(world, &AuraApplication {
        caster: owner, target: owner, spell_id: HASTE,
        effect_mask: auras::aura_effect_mask(&buff), source: AuraSource::Cast,
    }, &buff);
    assert!(auras::set_owned_duration(world, owner, owner, HASTE, duration as u32 * ticks));
}
