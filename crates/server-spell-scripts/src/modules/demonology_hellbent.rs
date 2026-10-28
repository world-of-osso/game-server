//! Hellbent Commander's source-qualified demon lifecycle and heartbeat.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, ActiveAuras};
use combat::hunter_pet::HunterPet;
use combat::random::Rng;
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_cast::summon::{self, SummonedCreature};
use combat::spell_cast::{self, targets};
use shared::components::{ActiveSpec, Npc};

const TALENT: u32 = 1250897;
const BUFF: u32 = 1281559;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::CombatStatusChanged(combat_changed),
    SpellScriptHook::SummonInitialized(summon_started),
    SpellScriptHook::SummonRemoved(summon_removed),
    SpellScriptHook::AfterQueuedTriggers(heartbeat),
];

#[derive(Component)]
struct HellbentClock(f64);

fn selected(world: &World, owner: Entity) -> bool {
    world
        .get::<ActiveSpec>(owner)
        .is_some_and(|spec| spec.0 == 266)
        && auras::has_aura(world, owner, TALENT)
}

fn combat_changed(world: &mut World, owner: Entity, active: bool) {
    if !active {
        world.entity_mut(owner).remove::<HellbentClock>();
        return;
    }
    if selected(world, owner) {
        // SimC sim.cpp1965–1967: uniform first heartbeat phase.
        let next = spell_cast::now(world) + combat::random::thread_rng().gen_range(0.001..5.249);
        world.entity_mut(owner).insert(HellbentClock(next));
    }
}

fn summon_started(world: &mut World, actor: Entity) {
    let Some(summon) = world.get::<SummonedCreature>(actor) else {
        return;
    };
    let owner = summon.owner;
    // pets.cpp1310–1341: Vilefiend variants count immediately on arise.
    if matches!(summon.spell_id, 1251781 | 455465 | 455476) && selected(world, owner) {
        add_stack(world, owner);
    }
}

fn summon_removed(world: &mut World, actor: Entity) {
    let Some(summon) = world.get::<SummonedCreature>(actor) else {
        return;
    };
    let owner = summon.owner;
    let root = summon.spell_id;
    if !selected(world, owner) {
        return;
    }
    // pets.cpp992–1006/1035–1040: ordinary Hand Imps lose two on normal
    // demise and one on Implosion; alternate Imps lose one/zero. Dogs lose one.
    let lost = match root {
        104317 => {
            if summon::is_imploded(world, actor) {
                1
            } else {
                2
            }
        }
        279910 => {
            if summon::is_imploded(world, actor) {
                0
            } else {
                1
            }
        }
        193331 | 193332 => 1,
        _ => 0,
    };
    remove_stacks(world, owner, lost);
}

fn add_stack(world: &mut World, owner: Entity) {
    if !spell_cast::try_cast_triggered(world, owner, BUFF, Some(owner), 1) {
        warn!(?owner, "native Hellbent Commander stack refused");
    }
}

fn remove_stacks(world: &mut World, owner: Entity, count: usize) {
    for _ in 0..count {
        let instance = world.get::<ActiveAuras>(owner).and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == BUFF && aura.caster == owner)
                .map(|aura| aura.instance_id)
        });
        if let Some(instance) = instance {
            auras::spend_stack(world, owner, instance);
        }
    }
}

fn heartbeat(world: &mut World) {
    let now = spell_cast::now(world);
    let due: Vec<_> = world
        .query::<(Entity, &HellbentClock)>()
        .iter(world)
        .filter(|(_, clock)| clock.0 <= now)
        .map(|(owner, _)| owner)
        .collect();
    for owner in due {
        if !selected(world, owner) {
            world.entity_mut(owner).remove::<HellbentClock>();
            continue;
        }
        reconcile(world, owner);
        // sim.cpp1216–1222: each following heartbeat is Gaussian 5.25s ±25ms.
        let mut rng = combat::random::thread_rng();
        let uniform: f64 = rng.gen_range(f64::MIN_POSITIVE..1.0);
        let angle = rng.gen_range(0.0..std::f64::consts::TAU);
        let interval = 5.25 + 0.025 * (-2.0 * uniform.ln()).sqrt() * angle.cos();
        world.get_mut::<HellbentClock>(owner).unwrap().0 += interval;
    }
}

fn reconcile(world: &mut World, owner: Entity) {
    let summons = world
        .query::<(Entity, &SummonedCreature)>()
        .iter(world)
        .filter(|(actor, summon)| {
            summon.owner == owner
                && matches!(
                    summon.spell_id,
                    104317
                        | 279910
                        | 193331
                        | 193332
                        | 265187
                        | 1276452
                        | 455465
                        | 455476
                        | 1276182
                        | 1276283
                        | 1282501
                        | 1282502
                )
                && targets::is_alive(world, *actor)
        })
        .count();
    let pets = world
        .query::<(Entity, &HunterPet, &Npc)>()
        .iter(world)
        .filter(|(actor, pet, npc)| {
            pet.owner == owner && npc.template_id == 17252 && targets::is_alive(world, *actor)
        })
        .count();
    let count = summons + pets;
    let current = world
        .get::<ActiveAuras>(owner)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == BUFF && aura.caster == owner)
        })
        .map_or(0, |aura| usize::from(aura.stacks));
    if count > current {
        for _ in current..count {
            add_stack(world, owner);
        }
    } else {
        remove_stacks(world, owner, current - count);
    }
}
