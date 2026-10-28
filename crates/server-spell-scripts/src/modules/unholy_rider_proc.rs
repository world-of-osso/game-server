//! Rune spend can summon an actual Rider of the Apocalypse.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{self, script_registry::SpellScriptHook, summon::SummonedCreature};
use combat::spell_info::{SpellInfo, SpellStore};
use shared::components::ActiveSpec;

const RIDERS: &[u32] = &[444248, 444252, 444254, 444251];

#[derive(Component)]
struct LastRider(u32);

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::BeforeNativeOnCast(on_cast)];

fn on_cast(world: &mut World, owner: Entity, spell: &SpellInfo, depth: u8) {
    if depth != 0 || world.get::<ActiveSpec>(owner).is_none_or(|spec| spec.0 != 252)
        || !combat::auras::has_aura(world, owner, 444005)
        || !spell.power_costs.iter().any(|cost| cost.power_type == 5 && cost.mana_cost > 0) { return; }
    // SimC76f8118 sc_death_knight.cpp13080–13110: the source uses its measured
    // 20% chance per rune-cost action, including a free cast of that action.
    if combat::random::random::<f32>() >= 0.2 { return; }
    let now = combat::auras::aura_now(world);
    let active: Vec<u32> = world.query::<&SummonedCreature>().iter(world)
        .filter_map(|summon| (summon.owner == owner && RIDERS.contains(&summon.spell_id)
            && summon.expires_at.is_some_and(|expiry| expiry > now)).then_some(summon.spell_id)).collect();
    if RIDERS.iter().all(|rider| active.contains(rider)) { return; }
    let previous = world.get::<LastRider>(owner).map(|last| last.0);
    let choices: Vec<_> = RIDERS.iter().copied().filter(|rider| Some(*rider) != previous).collect();
    // SimC76f8118 sc_death_knight.cpp13630–13648: exclude the previous pick,
    // even when another currently active rider is selected and produces nothing.
    let chosen = choices[(combat::random::random::<f64>() * choices.len() as f64) as usize];
    world.entity_mut(owner).insert(LastRider(chosen));
    if active.contains(&chosen) { return; }
    if !spell_cast::try_cast_triggered(world, owner, chosen, Some(owner), 1) {
        warn!(?owner, chosen, "Rider's Champion summon refused"); return;
    }
    let duration = world.resource::<SpellStore>().get(454389).and_then(|spell| spell.misc.as_ref())
        .and_then(|misc| misc.duration).expect("native random Rider duration").duration_ms;
    assert!(duration > 0, "native random Rider duration must be positive");
    let actor = world.query::<(Entity, &SummonedCreature)>().iter(world).find_map(|(actor, summon)|
        (summon.owner == owner && summon.spell_id == chosen && summon.expires_at.is_none()).then_some(actor))
        .expect("successful Rider summon must create its real actor");
    world.get_mut::<SummonedCreature>(actor).unwrap().expires_at = Some(now + f64::from(duration) / 1000.0);
}
