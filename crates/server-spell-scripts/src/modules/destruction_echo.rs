//! Embers of Nihilam: actual native Echo/Vision and earned MID2 target marks.
use super::combat;
use bevy::prelude::*;
use combat::auras::{ActiveAuras, AuraSource};
use combat::class_progression::TraitLoadout;
use combat::random as rand;
use combat::spell_cast::effects::EffectCtx;
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_cast::scripts::{AfterCastHook, script_effect_value};
use combat::spell_cast::targets;
use combat::spell_info::{SpellInfo, SpellStore};
use shared::components::{ActiveSpec, CombatStatus};

const EMBERS_I: u32 = 1265770;
const EMBERS_II: u32 = 1265772;
const EMBERS_III: u32 = 1265774;
const ECHO: u32 = 1265884;
const VISION: u32 = 1265939;
const TIER_FOUR: u32 = 1296572;
const MARK: u32 = 1305711;
const RAIN_OF_FIRE: u32 = 5740;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::SpellResidual(after_damage),
    SpellScriptHook::AuraRollAfterCast(after_cast),
    SpellScriptHook::OwnsPolicyProc(owns_proc),
    SpellScriptHook::TargetFilter(filter_echo_secondary),
];

#[derive(Component, Default)]
struct EchoProgress {
    attempts: u32,
    spender_ready_at: f64,
}

fn has_trait(world: &World, caster: Entity, spell: u32) -> bool {
    world.get::<TraitLoadout>(caster).is_some_and(|traits| {
        traits
            .active_talent_spells()
            .any(|talent| talent.spell_id == spell)
    })
}

fn is_destruction(world: &World, caster: Entity) -> bool {
    world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 267)
}

fn owns_proc(spell: u32) -> bool {
    // actions5168–5171: actual Echo impacts, not the broad native hint.
    spell == TIER_FOUR
}

fn after_damage(world: &mut World, ctx: &EffectCtx<'_>, target: Entity, _amount: f32) {
    if !is_destruction(world, ctx.caster) {
        return;
    }
    match ctx.spell.id {
        ECHO => apply_earned_mark(world, ctx.caster, target, ctx.depth),
        // actions3861/3937/5210: these actual hits alone advance PRD(I),
        // including FnB recipients. Echo/Wither/other copied damage cannot recurse.
        29722 | 434506 if has_trait(world, ctx.caster, EMBERS_I) => {
            if roll_echo(world, ctx.caster) {
                emit_echo(world, ctx.caster, target, ctx.depth + 1);
            }
        }
        // actions4337/4813: real landed spender impacts share native III ICD.
        116858 | 17877 => try_spender_echo(world, ctx.caster, Some(target), ctx.depth + 1),
        _ => {}
    }
}

fn ensure_progress(world: &mut World, caster: Entity) {
    if world.get::<EchoProgress>(caster).is_none() {
        world.entity_mut(caster).insert(EchoProgress::default());
    }
}

fn roll_echo(world: &mut World, caster: Entity) -> bool {
    // init1428–1434: accumulated PRD from native modified chance. Native
    // label modifiers, including the real2pc, own the10→20 value relationship.
    let chance = f64::from(script_effect_value(world, caster, EMBERS_I, 0)) / 100.0;
    let constant = rand::accumulated_proc_constant(chance);
    ensure_progress(world, caster);
    let mut state = world.get_mut::<EchoProgress>(caster).unwrap();
    state.attempts = state.attempts.saturating_add(1);
    let success = rand::random::<f64>() < constant * f64::from(state.attempts);
    if success {
        state.attempts = 0;
    }
    success
}

fn read_spender_cooldown(world: &World) -> f64 {
    let options = world
        .resource::<SpellStore>()
        .get(EMBERS_III)
        .expect("native Echo III spell")
        .aura_options
        .as_ref()
        .expect("native Echo III ProcCategoryRecovery");
    f64::from(options.proc_category_recovery_ms) / 1000.0
}

fn try_spender_echo(world: &mut World, caster: Entity, target: Option<Entity>, depth: u8) {
    if !has_trait(world, caster, EMBERS_III) {
        return;
    }
    ensure_progress(world, caster);
    let now = combat::spell_cast::now(world);
    if world.get::<EchoProgress>(caster).unwrap().spender_ready_at > now {
        return;
    }
    let Some(target) = find_native_target(world, caster, target) else {
        return;
    };
    // actions6039–6042: pinned reference bugs=true uses full native payload,
    // not the tooltip50/60% spender effectiveness. No coefficient override.
    if emit_echo(world, caster, target, depth) {
        let ready_at = now + read_spender_cooldown(world);
        world
            .get_mut::<EchoProgress>(caster)
            .unwrap()
            .spender_ready_at = ready_at;
    }
}

fn emit_echo(world: &mut World, caster: Entity, target: Entity, depth: u8) -> bool {
    if !combat::spell_cast::try_cast_triggered(world, caster, ECHO, Some(target), depth) {
        warn!(?caster, ?target, "Native Echo1265884 refused");
        return false;
    }
    // actions450–455/5614–5649: proc activation earns the native Vision;
    // native talent label mods supply its haste/crit values, not stat writes.
    if has_trait(world, caster, EMBERS_II)
        && !combat::spell_cast::try_cast_triggered(world, caster, VISION, Some(caster), depth)
    {
        warn!(?caster, "Native Vision1265939 refused");
    }
    true
}

fn is_live_enemy(world: &World, caster: Entity, target: Entity) -> bool {
    targets::is_alive(world, target)
        && targets::same_map(world, caster, target)
        && targets::is_hostile(world, caster, target)
}

fn collect_owned_dot_targets(world: &mut World, caster: Entity) -> Vec<Entity> {
    let now = combat::spell_cast::now(world);
    world
        .query::<(Entity, &ActiveAuras)>()
        .iter(world)
        .filter(|(target, auras)| {
            is_live_enemy(world, caster, *target)
                && auras.iter().any(|aura| {
                    let owned_dot =
                        aura.caster == caster && matches!(aura.spell_id, 157736 | 445474);
                    owned_dot && aura.expires_at.is_none_or(|expiry| expiry > now)
                })
        })
        .map(|(target, _)| target)
        .collect()
}

fn find_native_target(world: &mut World, caster: Entity, target: Option<Entity>) -> Option<Entity> {
    if let Some(target) = target.filter(|target| is_live_enemy(world, caster, *target)) {
        return Some(target);
    }
    // actions5623–5641 explicitly selects an actual owned ticking-DoT target
    // when the original target is absent/dead. No synthetic candidate is supplied.
    let candidates = collect_owned_dot_targets(world, caster);
    if candidates.is_empty() {
        return None;
    }
    Some(candidates[rand::Rng::gen_range(&mut rand::thread_rng(), 0..candidates.len())])
}

fn after_cast(spell: u32) -> Option<AfterCastHook> {
    (spell == RAIN_OF_FIRE).then_some(after_rain_of_fire)
}

fn after_rain_of_fire(world: &mut World, caster: Entity, _spell: &SpellInfo) {
    let in_combat = world
        .get::<CombatStatus>(caster)
        .is_some_and(|status| status.0);
    let eligible = is_destruction(world, caster) && in_combat;
    if !eligible {
        return;
    }
    // actions4605/5619–5621: one ordinary RoF execute; no periodic/OOC proc.
    let target = world
        .get::<combat::networking::CombatTarget>(caster)
        .and_then(|target| target.0);
    try_spender_echo(world, caster, target, 1);
}

fn filter_echo_secondary(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    index: u8,
    primary: Option<Entity>,
    units: &mut Vec<Entity>,
) {
    let secondary = spell.id == ECHO && index == 1;
    if secondary && is_destruction(world, caster) {
        // Native24a0c65f22 RED: primary took both effects. Source actions5160–5161
        // gives primary its own coefficient and secondary recipients effect1 only.
        units.retain(|unit| Some(*unit) != primary);
    }
}

fn apply_earned_mark(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    let equipped = world.get::<ActiveAuras>(caster).is_some_and(|auras| {
        auras
            .iter()
            .any(|aura| aura.spell_id == TIER_FOUR && matches!(aura.source, AuraSource::ItemSet(_)))
    });
    if equipped
        && !combat::spell_cast::try_cast_triggered(world, caster, MARK, Some(target), depth + 1)
    {
        warn!(
            ?caster,
            ?target,
            "Native earned Dark Titan mark1305711 refused"
        );
    }
}
