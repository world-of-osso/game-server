//! Whitemane applies and grows the owner's native Undeath disease.
use super::combat;
use bevy::prelude::*;
use combat::auras::ActiveAuras;
use combat::spell_cast::{
    self, effects::EffectCtx, script_registry::SpellScriptHook, summon::SummonedCreature,
};
use combat::spell_cast::scripts::script_effect_value;
use combat::spell_info::{SpellInfo, SpellStore};
use shared::components::{Npc, Position};

#[derive(Component)]
struct UndeathReady(f64);

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::SummonInitialized(initialize),
    SpellScriptHook::AfterForegroundSpender(whitemane_coil),
    SpellScriptHook::AfterCastUpdate(update),
    SpellScriptHook::AfterPeriodicDamagePolicies(grow),
];

fn whitemane_coil(world: &mut World, owner: Entity, spell: &SpellInfo, target: Option<Entity>) {
    if !matches!(spell.id, 47541 | 1242174) || !combat::auras::has_aura(world, owner, 1265949) {
        return;
    }
    let Some(target) = target else { return };
    let now = combat::auras::aura_now(world);
    let rider = world
        .query::<(Entity, &SummonedCreature)>()
        .iter(world)
        .find_map(|(actor, summon)| {
            (summon.owner == owner && summon.spell_id == 444251
                && summon.expires_at.is_some_and(|expiry| expiry > now))
                .then_some(actor)
        });
    let Some(rider) = rider else { return };
    // SimC76f8118 sc_death_knight.cpp5091–5096/13575–13579:
    // each actual Coil commands the active Whitemane's background Coil.
    let multiplier = script_effect_value(world, owner, 1265949, 1) / 100.0;
    if !spell_cast::cast_triggered_with_overrides(
        world, rider, 445513, Some(target), 1,
        spell_cast::TriggeredSpellOverrides { damage_multiplier: Some(multiplier), ..Default::default() },
    ) {
        warn!(?rider, "Whitemane Let Terror Reign Coil refused");
    }
}

fn initialize(world: &mut World, actor: Entity) {
    if world
        .get::<SummonedCreature>(actor)
        .is_some_and(|summon| summon.spell_id == 444251)
        && world
            .get::<Npc>(actor)
            .is_some_and(|npc| npc.template_id == 221633)
    {
        let now = combat::auras::aura_now(world);
        world.entity_mut(actor).insert(UndeathReady(now));
    }
}

fn update(world: &mut World) {
    let now = combat::auras::aura_now(world);
    let ready: Vec<_> = world
        .query::<(Entity, &SummonedCreature, &UndeathReady)>()
        .iter(world)
        .filter_map(|(actor, summon, ready)| (ready.0 <= now).then_some((actor, summon.owner)))
        .collect();
    for (actor, owner) in ready {
        let Some(target) = world
            .get::<combat::networking::CombatTarget>(owner)
            .and_then(|target| target.0)
        else {
            continue;
        };
        let (Some(from), Some(to)) = (world.get::<Position>(actor), world.get::<Position>(target))
        else {
            continue;
        };
        let range = world
            .resource::<SpellStore>()
            .get(444634)
            .expect("native Undeath range");
        let (_, maximum) = spell_cast::targets::spell_range(range, true);
        let distance_sq =
            (from.x - to.x).powi(2) + (from.y - to.y).powi(2) + (from.z - to.z).powi(2);
        if distance_sq > maximum.powi(2) {
            continue;
        }
        // SimC76f8118 sc_death_knight.cpp5190–5197: Whitemane uses the native
        // range action, then the DK owns Undeath; its cooldown is the DoT duration.
        if !spell_cast::try_cast_triggered(world, actor, 444634, Some(target), 1)
            || !spell_cast::try_cast_triggered(world, owner, 444633, Some(target), 1)
        {
            warn!(?actor, ?target, "Whitemane Undeath cast refused");
            continue;
        }
        let duration = world
            .resource::<SpellStore>()
            .get(444633)
            .and_then(|spell| spell.misc.as_ref())
            .and_then(|misc| misc.duration)
            .expect("native Undeath duration")
            .duration_ms;
        assert!(duration > 0, "native Undeath duration must be positive");
        world.get_mut::<UndeathReady>(actor).unwrap().0 = now + f64::from(duration) / 1000.0;
    }
}

fn grow(world: &mut World, ctx: &EffectCtx<'_>, target: Entity) {
    if ctx.spell.id != 444633 {
        return;
    }
    let Some(stacks) = world.get::<ActiveAuras>(target).and_then(|auras| {
        auras
            .iter()
            .find(|aura| aura.spell_id == 444633 && aura.caster == ctx.caster)
            .map(|aura| aura.stacks)
    }) else {
        return;
    };
    // SimC76f8118 sc_death_knight.cpp8078–8089 increments after each actual tick.
    combat::auras::set_owned_stacks(world, ctx.caster, target, 444633, stacks.saturating_add(1));
}
