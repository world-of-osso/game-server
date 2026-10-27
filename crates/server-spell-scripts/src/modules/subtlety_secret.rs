//! SimC76f8118 sc_rogue.cpp5331–5501: Secret Technique snapshots its
//! effective points and crit before its player and two delayed clone attacks.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, ActiveAuras, handlers, spell_mods};
use combat::spell_cast::{
    self,
    effects::{self, EffectCtx},
    script_registry::{self, SpellScriptHook},
    scripts::script_effect_value,
};
use combat::spell_info::{SpellInfo, SpellStore};
use shared::components::{ActiveSpec, PowerType};

const PARENT: u32 = 280719;
const PLAYER: u32 = 280720;
const CLONE: u32 = 282449;
const ANCIENT_ARTS: u32 = 1269163;

#[derive(Component)]
struct SecretCast {
    parent: SpellInfo,
    target: Entity,
    player: SpellInfo,
    clone: SpellInfo,
    player_crit: f32,
    clone_crit: f32,
    due: Vec<f64>,
}

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::ResourceOnlyContext(&[PARENT]),
    SpellScriptHook::BeforeNativeOnCast(snapshot_secret),
    SpellScriptHook::AfterForegroundSpecial(launch_player),
    SpellScriptHook::AfterQueuedTriggers(launch_due_clones),
    SpellScriptHook::CriticalChance(snapshot_critical_chance),
    SpellScriptHook::TargetContextDamageMultiplier(secondary_fraction),
];

fn snapshot_secret(world: &mut World, caster: Entity, parent: &SpellInfo, depth: u8) {
    if depth != 0
        || parent.id != PARENT
        || !world
            .get::<ActiveSpec>(caster)
            .is_some_and(|spec| spec.0 == 261)
    {
        return;
    }
    let target = parent
        .cast_primary_target
        .expect("native Secret Technique primary");
    let points = parent
        .cast_policy_context
        .as_ref()
        .expect("native Secret Technique resource snapshot")
        .resources
        .iter()
        .find(|&&(power, _)| power == PowerType::ComboPoints)
        .expect("native Secret Technique effective points")
        .1;
    let player = snapshot_child(world, caster, PLAYER, points);
    let clone = snapshot_child(world, caster, CLONE, points);
    let player_crit =
        effects::crit_chance(world, caster, target, &player) - target_crit_modifier(world, target);
    let clone_crit =
        effects::crit_chance(world, caster, target, &clone) - target_crit_modifier(world, target);
    let now = spell_cast::now(world);
    let due = if auras::has_aura(world, caster, ANCIENT_ARTS) {
        vec![now + 0.5, now + 0.5]
    } else {
        vec![now + 1.0, now + 1.3]
    };
    world.entity_mut(caster).insert(SecretCast {
        parent: parent.clone(),
        target,
        player,
        clone,
        player_crit,
        clone_crit,
        due,
    });
}

fn snapshot_child(world: &World, caster: Entity, id: u32, points: f32) -> SpellInfo {
    let mut child = world
        .resource::<SpellStore>()
        .get(id)
        .expect("native Secret Technique payload")
        .clone();
    child.cast_damage_multiplier = Some(points);
    child.cast_charge_mods = Some(spell_mods::snapshot_charged_modifiers(
        world, caster, &child,
    ));
    child
}

fn launch_player(world: &mut World, caster: Entity, parent: &SpellInfo) {
    if parent.id != PARENT {
        return;
    }
    let Some(state) = world.get::<SecretCast>(caster) else {
        return;
    };
    let target = state.target;
    let spell = state.player.clone();
    spell_cast::launch_triggered_spell(world, caster, &spell, Some(target), 1);
}

fn launch_due_clones(world: &mut World) {
    let now = spell_cast::now(world);
    let mut due = Vec::new();
    for (caster, mut state) in world.query::<(Entity, &mut SecretCast)>().iter_mut(world) {
        let count = state.due.iter().filter(|&&time| time <= now).count();
        if count > 0 {
            state.due.retain(|&time| time > now);
            due.push((caster, state.target, state.clone.clone(), count));
        }
    }
    for (caster, target, spell, count) in due {
        for _ in 0..count {
            if world.get_entity(target).is_ok() {
                spell_cast::launch_triggered_spell(world, caster, &spell, Some(target), 1);
            }
        }
        if world
            .get::<SecretCast>(caster)
            .is_some_and(|state| state.due.is_empty())
        {
            world.entity_mut(caster).remove::<SecretCast>();
        }
    }
}

pub(super) fn parent_snapshot(world: &World, caster: Entity) -> Option<SpellInfo> {
    world
        .get::<SecretCast>(caster)
        .map(|state| state.parent.clone())
}

fn target_crit_modifier(world: &World, target: Entity) -> f32 {
    world.get::<ActiveAuras>(target).map_or(0.0, |auras| {
        auras.modifier(handlers::MOD_ATTACKER_MELEE_CRIT_CHANCE)
    })
}

fn snapshot_critical_chance(
    world: &World,
    caster: Entity,
    target: Entity,
    spell: u32,
) -> Option<f32> {
    let state = world.get::<SecretCast>(caster)?;
    let chance = match spell {
        PLAYER => state.player_crit,
        CLONE => state.clone_crit,
        _ => return None,
    };
    let child = if spell == PLAYER {
        &state.player
    } else {
        &state.clone
    };
    // crit_chance adds live target modifiers and guaranteed-crit policies after
    // this hook; subtract them so the captured launch chance remains unchanged.
    let guaranteed = if script_registry::guarantees_crit(world, caster, child) {
        100.0
    } else {
        0.0
    };
    Some(
        chance - guaranteed + target_crit_modifier(world, state.target)
            - target_crit_modifier(world, target),
    )
}

fn secondary_fraction(world: &World, ctx: &EffectCtx, target: Entity, _: bool) -> f32 {
    if !matches!(ctx.spell.id, PLAYER | CLONE)
        || ctx.spell.cast_primary_target == Some(target)
    {
        return 1.0;
    }
    // sc_rogue.cpp5392–5398: the physical player and Secret Technique
    // clones apply the native effect3 fraction to secondary recipients.
    script_effect_value(world, ctx.caster, PARENT, 2) / 100.0
}
