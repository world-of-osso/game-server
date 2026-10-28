//! Whitemane applies and grows the owner's native Undeath disease.
use super::combat;
use bevy::prelude::*;
use combat::auras::ActiveAuras;
use combat::spell_cast::{self, effects::EffectCtx, script_registry::SpellScriptHook, summon::SummonedCreature};
use combat::spell_info::SpellStore;
use shared::components::{Npc, Position};

#[derive(Component)]
struct UndeathReady(f64);

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::SummonInitialized(initialize),
    SpellScriptHook::AfterCastUpdate(update),
    SpellScriptHook::AfterPeriodicDamagePolicies(grow),
    SpellScriptHook::AfterPolicyDirectDamage(spread),
];

fn initialize(world: &mut World, actor: Entity) {
    if world.get::<SummonedCreature>(actor).is_some_and(|summon| summon.spell_id == 444251)
        && world.get::<Npc>(actor).is_some_and(|npc| npc.template_id == 221633) {
        let now = combat::auras::aura_now(world);
        world.entity_mut(actor).insert(UndeathReady(now));
    }
}

fn update(world: &mut World) {
    let now = combat::auras::aura_now(world);
    let ready: Vec<_> = world.query::<(Entity, &SummonedCreature, &UndeathReady)>().iter(world)
        .filter_map(|(actor, summon, ready)| (ready.0 <= now).then_some((actor, summon.owner))).collect();
    for (actor, owner) in ready {
        let Some(target) = world.get::<combat::networking::CombatTarget>(owner).and_then(|target| target.0) else { continue; };
        let (Some(from), Some(to)) = (world.get::<Position>(actor), world.get::<Position>(target)) else { continue; };
        let range = world.resource::<SpellStore>().get(444634).expect("native Undeath range");
        let (_, maximum) = spell_cast::targets::spell_range(range, true);
        let distance_sq = (from.x - to.x).powi(2) + (from.y - to.y).powi(2) + (from.z - to.z).powi(2);
        if distance_sq > maximum.powi(2) { continue; }
        // SimC76f8118 sc_death_knight.cpp5190–5197: Whitemane uses the native
        // range action, then the DK owns Undeath; its cooldown is the DoT duration.
        if !spell_cast::try_cast_triggered(world, actor, 444634, Some(target), 1)
            || !spell_cast::try_cast_triggered(world, owner, 444633, Some(target), 1) {
            warn!(?actor, ?target, "Whitemane Undeath cast refused"); continue;
        }
        let duration = world.resource::<SpellStore>().get(444633).and_then(|spell| spell.misc.as_ref())
            .and_then(|misc| misc.duration).expect("native Undeath duration").duration_ms;
        assert!(duration > 0, "native Undeath duration must be positive");
        world.get_mut::<UndeathReady>(actor).unwrap().0 = now + f64::from(duration) / 1000.0;
    }
}

fn spread(world: &mut World, ctx: &EffectCtx<'_>, target: Entity, _: bool) {
    if ctx.spell.id != 55090 || !combat::auras::has_aura(world, ctx.caster, 444033) { return; }
    let Some(stacks) = owned_stacks(world, ctx.caster, target) else { return; };
    // SimC76f8118 sc_death_knight.cpp12576–12578/13728–13749: a real
    // Scourge Strike grows its owned disease and spreads to a least-stacked target.
    let add = combat::spell_cast::scripts::script_effect_value(world, ctx.caster, 444633, 2) as u8;
    combat::auras::set_owned_stacks(world, ctx.caster, target, 444633, stacks.saturating_add(add));
    let Some(position) = world.get::<Position>(ctx.caster) else { return; };
    let center = Vec3::new(position.x, position.y, position.z);
    let range = world.resource::<SpellStore>().get(444633).expect("native Undeath range");
    let radius = spell_cast::targets::spell_range(range, true).1;
    let mut targets = spell_cast::targets::area_units(world, ctx.caster, center, radius, spell_cast::targets::Relation::Enemy, 0);
    targets.retain(|candidate| *candidate != target);
    targets.sort_by_key(|candidate| owned_stacks(world, ctx.caster, *candidate).unwrap_or(0));
    let Some(other) = targets.first().copied() else { return; };
    if let Some(stacks) = owned_stacks(world, ctx.caster, other) {
        combat::auras::set_owned_stacks(world, ctx.caster, other, 444633, stacks.saturating_add(add));
    } else {
        combat::auras::clone_owned_undeath(world, ctx.caster, target, other);
    }
}

fn owned_stacks(world: &World, owner: Entity, target: Entity) -> Option<u8> {
    world.get::<ActiveAuras>(target)?.iter()
        .find(|aura| aura.spell_id == 444633 && aura.caster == owner).map(|aura| aura.stacks)
}

fn grow(world: &mut World, ctx: &EffectCtx<'_>, target: Entity) {
    if ctx.spell.id != 444633 { return; }
    let Some(stacks) = world.get::<ActiveAuras>(target).and_then(|auras| auras.iter()
        .find(|aura| aura.spell_id == 444633 && aura.caster == ctx.caster).map(|aura| aura.stacks)) else { return; };
    // SimC76f8118 sc_death_knight.cpp8078–8089 increments after each actual tick.
    combat::auras::set_owned_stacks(world, ctx.caster, target, 444633, stacks.saturating_add(1));
}
