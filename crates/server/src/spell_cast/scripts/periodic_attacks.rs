//! Source-qualified extra attacks on native periodic trigger ordinals.
use crate::auras::ActiveAuras;
use crate::spell_info::SpellStore;
use bevy::prelude::*;
use shared::components::ActiveSpec;

struct PeriodicAttack {
    parents: &'static [u32],
    effect: u8,
    passive: u32,
    multiplier_effect: u8,
    actions: &'static [(u32, u32)],
}

const ATTACKS: &[PeriodicAttack] = &[
    // SimC76f8118 sc_warrior.cpp3257–3275/3313–3335: odd scheduled
    // Bladestorm ticks (initial tick0 excluded) cast Arms MS or live Fury BT.
    // 2644–2646/2880–2882/3057–3059: native Unhinged effect2 scales its child.
    PeriodicAttack {
        parents: &[227847, 446035],
        effect: 0,
        passive: 386628,
        multiplier_effect: 1,
        actions: &[(71, 12294), (72, 23881)],
    },
];

pub(crate) fn after_periodic_trigger(
    world: &mut World,
    caster: Entity,
    parent: u32,
    effect: u8,
    ordinal: u64,
) {
    if ordinal % 2 != 1 {
        return;
    }
    for policy in ATTACKS
        .iter()
        .filter(|policy| policy.parents.contains(&parent) && policy.effect == effect)
    {
        let Some((action, multiplier)) = read_attack(world, caster, policy) else {
            continue;
        };
        cast_periodic_attack(world, caster, action, multiplier);
    }
}

fn read_attack(world: &World, caster: Entity, policy: &PeriodicAttack) -> Option<(u32, f32)> {
    let spec = world.get::<ActiveSpec>(caster)?.0;
    let (_, action) = policy
        .actions
        .iter()
        .find(|(allowed, _)| *allowed == spec)?;
    let passive = world
        .get::<ActiveAuras>(caster)?
        .iter()
        .find(|aura| aura.spell_id == policy.passive && aura.caster == caster)?;
    let percent = passive
        .effects
        .iter()
        .find(|effect| effect.index == policy.multiplier_effect)
        .expect("native periodic attack multiplier effect")
        .amount;
    let action = crate::auras::overrides::resolve_spell(world, caster, *action);
    Some((action, percent / 100.0))
}

fn cast_periodic_attack(world: &mut World, caster: Entity, action: u32, multiplier: f32) {
    let Some(target) = world
        .get::<crate::networking::CombatTarget>(caster)
        .and_then(|selected| selected.0)
    else {
        return;
    };
    if !crate::spell_cast::targets::is_alive(world, target) {
        return;
    }
    let mut child = world
        .resource::<SpellStore>()
        .get(action)
        .expect("native periodic attack action")
        .clone();
    child.cast_damage_multiplier = Some(multiplier);
    child.cast_primary_target = Some(target);
    if !super::super::launch_triggered_spell(world, caster, &child, Some(target), 1) {
        warn!(?caster, action, "Native periodic attack refused");
    }
}
