//! Huntmaster Call counts genuine beasts and earns a native companion.
use super::{cast_relationships::has_talent, combat};
use bevy::prelude::*;
use combat::auras::{self, ActiveAuras, RemoveReason};
use combat::spell_cast::{
    self, TriggeredSpellOverrides, script_registry::SpellScriptHook, summon::SummonedCreature,
};
use combat::spell_info::{SpellInfo, SpellStore};

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::AfterElementalLaunch(after_summon)];
const TALENT: u32 = 459730;
const COUNTER: u32 = 459731;
const CONTRIBUTORS: &[u32] = &[219199, 442419, 471990];
const COMPANIONS: &[(u32, u32)] = &[(459733, 459735), (459739, 459738)];
const RAVENOUS_LEAP: u32 = 459753;

#[derive(Component)]
struct Counted;
#[derive(Component)]
struct CompanionStarted;

fn owned_summons(world: &mut World, owner: Entity, spell: u32) -> Vec<Entity> {
    world
        .query::<(Entity, &SummonedCreature)>()
        .iter(world)
        .filter(|(_, summon)| summon.owner == owner && summon.spell_id == spell)
        .map(|(actor, _)| actor)
        .collect()
}

fn counter_stacks(world: &World, owner: Entity) -> u32 {
    world
        .get::<ActiveAuras>(owner)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.caster == owner && aura.spell_id == COUNTER)
        })
        .map_or(0, |aura| u32::from(aura.stacks))
}

fn earn_companion(world: &mut World, owner: Entity, primary: Entity) {
    let (summon, buff) = COMPANIONS[usize::from(combat::random::random::<bool>())];
    if !spell_cast::try_cast_triggered(world, owner, buff, Some(owner), 1) {
        warn!(?owner, buff, "native Huntmaster buff refused");
        return;
    }
    // SimC76f8118 hunter3723–3734: the selected buff owns lifetime; replace
    // an existing companion of the same kind before summoning the real actor.
    let deadline = world
        .get::<ActiveAuras>(owner)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.caster == owner && aura.spell_id == buff)
        })
        .and_then(|aura| aura.expires_at)
        .expect("actual Huntmaster buff must have a finite deadline");
    let remaining_ms = ((deadline - spell_cast::now(world)) * 1000.0).round() as i32;
    assert!(remaining_ms > 0, "Huntmaster buff must still be active");
    for actor in owned_summons(world, owner, summon) {
        spell_cast::summon::despawn_summon(world, actor);
    }
    let values = TriggeredSpellOverrides {
        duration_ms: Some(remaining_ms),
        ..Default::default()
    };
    if !spell_cast::cast_triggered_with_overrides(world, owner, summon, Some(primary), 1, values) {
        warn!(?owner, summon, "native Huntmaster companion refused");
    }
}

fn count_new_beasts(world: &mut World, owner: Entity, spell: u32, primary: Entity) {
    let fresh: Vec<_> = owned_summons(world, owner, spell)
        .into_iter()
        .filter(|actor| world.get::<Counted>(*actor).is_none())
        .collect();
    for actor in fresh {
        world.entity_mut(actor).insert(Counted);
        if !spell_cast::try_cast_triggered(world, owner, COUNTER, Some(owner), 1) {
            warn!(?owner, "native Huntmaster counter refused");
            continue;
        }
        let cap = world
            .resource::<SpellStore>()
            .get(COUNTER)
            .expect("native Huntmaster counter")
            .aura_options
            .as_ref()
            .expect("native Huntmaster stack cap")
            .cumulative_aura;
        if counter_stacks(world, owner) >= cap {
            auras::remove_auras(world, owner, RemoveReason::Consumed, |aura| {
                aura.spell_id == COUNTER && aura.caster == owner
            });
            earn_companion(world, owner, primary);
        }
    }
}

fn start_new_companions(world: &mut World, owner: Entity, spell: u32, primary: Entity) {
    let fresh: Vec<_> = owned_summons(world, owner, spell)
        .into_iter()
        .filter(|actor| world.get::<CompanionStarted>(*actor).is_none())
        .collect();
    for actor in fresh {
        world.entity_mut(actor).insert(CompanionStarted);
        // SimC76f8118 hunter1869–1873/1977–1982: real DireCritters attack;
        // Fenryr immediately applies its own native Ravenous Leap.
        spell_cast::summon::start_melee_attack(world, actor, primary);
        if spell == 459733
            && !spell_cast::try_cast_triggered(world, actor, RAVENOUS_LEAP, Some(primary), 1)
        {
            warn!(?actor, ?primary, "native Ravenous Leap refused");
        }
    }
}

fn after_summon(world: &mut World, owner: Entity, spell: &SpellInfo, _: bool) {
    if !has_talent(world, owner, TALENT) {
        return;
    }
    let Some(primary) = spell.cast_primary_target else {
        return;
    };
    if CONTRIBUTORS.contains(&spell.id) {
        count_new_beasts(world, owner, spell.id, primary);
    } else if COMPANIONS.iter().any(|(summon, _)| *summon == spell.id) {
        start_new_companions(world, owner, spell.id, primary);
    }
}
