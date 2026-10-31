//! Army's selected real Abomination and its Disease Cloud.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{
    self, script_registry::SpellScriptHook, scripts::AfterCastHook, summon::SummonedCreature,
};
use combat::spell_info::SpellInfo;

#[derive(Component)]
struct PendingAbomination(f64);
#[derive(Component)]
struct DiseaseCloud {
    next: f64,
    period: f64,
}

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AdditionalReachActions(autonomous_actions),
    SpellScriptHook::AuraRollAfterCast(after_cast),
    SpellScriptHook::SummonInitialized(initialize),
    SpellScriptHook::AfterCastUpdate(update),
];

fn autonomous_actions(world: &World, actor: Entity) -> Vec<u32> {
    if world.get::<DiseaseCloud>(actor).is_some() {
        vec![1244347]
    } else {
        Vec::new()
    }
}

fn after_cast(spell: u32) -> Option<AfterCastHook> {
    (spell == 42650).then_some(schedule)
}

fn schedule(world: &mut World, caster: Entity, _: &SpellInfo) {
    if !combat::auras::has_aura(world, caster, 1242608) {
        return;
    }
    // SimC76f8118 sc_death_knight.cpp9002–9008: the Abomination appears
    // with the final Army ghoul, using Army's native count and 0.5s spacing.
    let count = spell_cast::scripts::script_effect_value(world, caster, 42650, 0);
    let due = combat::auras::aura_now(world) + f64::from(count) * 0.5;
    world.entity_mut(caster).insert(PendingAbomination(due));
}

fn initialize(world: &mut World, actor: Entity) {
    let Some(summon) = world.get::<SummonedCreature>(actor) else {
        return;
    };
    if summon.spell_id != 288853 {
        return;
    }
    let owner = summon.owner;
    let target = world
        .get::<combat::networking::CombatTarget>(owner)
        .and_then(|target| target.0);
    let Some(target) = target else {
        warn!(?owner, "Abomination has no selected target");
        return;
    };
    // SimC76f8118 sc_death_knight.cpp5438–5504: 3.6s melee and an
    // immediate Disease Cloud followed by the selected talent's native period.
    let period = f64::from(spell_cast::scripts::script_effect_value(
        world, owner, 1242608, 0,
    ));
    assert!(
        period > 0.0,
        "native Abomination cloud period must be positive"
    );
    let next = combat::auras::aura_now(world);
    world.entity_mut(actor).insert((
        combat::combat::CreatureAttackTime(3.6),
        DiseaseCloud { next, period },
    ));
    spell_cast::summon::start_melee_attack(world, actor, target);
}

fn update(world: &mut World) {
    let now = combat::auras::aura_now(world);
    let due: Vec<_> = world
        .query::<(Entity, &PendingAbomination)>()
        .iter(world)
        .filter_map(|(owner, pending)| (now >= pending.0).then_some(owner))
        .collect();
    for owner in due {
        world.entity_mut(owner).remove::<PendingAbomination>();
        if !spell_cast::try_cast_triggered(world, owner, 288853, Some(owner), 1) {
            warn!(?owner, "native Abomination summon refused");
        }
    }
    let clouds: Vec<_> = world
        .query::<(Entity, &mut DiseaseCloud)>()
        .iter_mut(world)
        .filter_map(|(actor, mut cloud)| {
            if now < cloud.next {
                return None;
            }
            cloud.next += cloud.period;
            Some(actor)
        })
        .collect();
    for actor in clouds {
        if !spell_cast::try_cast_triggered(world, actor, 1244347, Some(actor), 1) {
            warn!(?actor, "native Abomination Disease Cloud refused");
        }
    }
}
