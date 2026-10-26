//! Pack Leader's earned beast cycle and Stampede; native records own all values.
use super::{cast_relationships::has_talent, combat};
use bevy::prelude::*;
use combat::auras::{self, ActiveAuras, AuraInstance, RemoveReason, scripts::AuraScripts};
use combat::spell_cast::script_registry::{SpellScriptHook, TargetPlanHook};
use combat::spell_cast::{self, PlannedEffect, TargetPlan, effects::EffectCtx, targets};
use combat::spell_info::{SpellInfo, SpellStore};
use shared::components::CombatStatus;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::OwnsPolicyProc(owns_proc),
    SpellScriptHook::AfterQueuedTriggers(begin_combat),
    SpellScriptHook::AfterElementalLaunch(after_launch),
    SpellScriptHook::EarlyAuraRegistration(register),
    SpellScriptHook::TargetPlan(target_plan),
];

const HOWL: u32 = 471876;
const COOLDOWN: u32 = 471877;
const READY: [u32; 3] = [471878, 472324, 472325];
const SUMMON: [u32; 3] = [1222271, 0, 471990];
const STAMPEDE: u32 = 472741;
const INCOMING: u32 = 1258338;
const STAMPEDE_TRIGGER: u32 = 1258344;
const STAMPEDE_DAMAGE: u32 = 201594;

#[derive(Component, Default)]
struct NextBeast(usize);

#[derive(Component)]
struct HowlCombatStarted;

#[derive(Component)]
struct StampedeTarget(Entity);

fn owns_proc(spell: u32) -> bool {
    // SimC76f8118 sc_hunter.cpp3776–3817: only Kill Command consumes
    // earned beasts and their Incoming Stampede. Boar/Bear readiness and
    // Incoming have broad one-charge cast-success procs; ordinary Cobra,
    // Barbed and nested summon casts must not spend these script-owned states.
    spell == INCOMING || READY.contains(&spell)
}

fn register(scripts: &mut AuraScripts) {
    scripts.0.entry(COOLDOWN).or_default().on_natural_end = Some(earn_beast);
    for ready in READY {
        scripts.0.entry(ready).or_default().on_remove = Some(rearm);
    }
    scripts.0.entry(STAMPEDE_TRIGGER).or_default().on_periodic = Some(stampede_tick);
}

fn apply_self(world: &mut World, caster: Entity, spell: u32) {
    if !spell_cast::try_cast_triggered(world, caster, spell, Some(caster), 1) {
        warn!(?caster, spell, "Pack Leader native aura refused");
    }
}

fn begin_combat(world: &mut World) {
    let actors: Vec<_> = world
        .query::<(Entity, &CombatStatus, &ActiveAuras)>()
        .iter(world)
        .filter(|(_, status, auras)| status.0 && auras.has_spell(HOWL))
        .map(|(unit, _, _)| unit)
        .collect();
    for actor in actors {
        if world.get::<HowlCombatStarted>(actor).is_none() {
            if world.get::<NextBeast>(actor).is_none() {
                world.entity_mut(actor).insert(NextBeast::default());
            }
            world.entity_mut(actor).insert(HowlCombatStarted);
            // SimC76f8118 sc_hunter.cpp8698–8699: first combat starts
            // the native cooldown, not an unearned ready or damage event.
            apply_self(world, actor, COOLDOWN);
        }
    }
}

fn earn_beast(world: &mut World, caster: Entity, _: &AuraInstance) {
    advance_beast(world, caster);
}

fn advance_beast(world: &mut World, caster: Entity) {
    if !auras::has_aura(world, caster, HOWL) {
        return;
    }
    // SimC76f8118 sc_hunter.cpp3824–3841: Wyvern → Boar → Bear.
    let index = world.get::<NextBeast>(caster).map_or(0, |next| next.0);
    world
        .entity_mut(caster)
        .insert(NextBeast((index + 1) % READY.len()));
    apply_self(world, caster, READY[index]);
}

fn rearm(world: &mut World, caster: Entity, _: &AuraInstance, reason: RemoveReason) {
    // SimC76f8118 sc_hunter.cpp8240–8270: losing readiness starts a
    // cooldown only if one is not already running, including natural expiry.
    if matches!(reason, RemoveReason::Consumed | RemoveReason::Expired)
        && auras::has_aura(world, caster, HOWL)
        && !auras::has_aura(world, caster, COOLDOWN)
    {
        apply_self(world, caster, COOLDOWN);
    }
}

fn after_launch(world: &mut World, caster: Entity, spell: &SpellInfo, foreground: bool) {
    if !foreground || !auras::has_aura(world, caster, HOWL) {
        return;
    }
    if spell.id == 19574 && has_talent(world, caster, STAMPEDE) {
        // SimC76f8118 sc_hunter.cpp6780–6781: BW arms Stampede and
        // advances Howl independently of the currently running cooldown.
        apply_self(world, caster, INCOMING);
        advance_beast(world, caster);
    }
    // SimC76f8118 sc_hunter.cpp6604 shares Howl consumption between
    // the specialization-specific native Kill Command roots.
    if matches!(spell.id, 34026 | 259489)
        && let Some(primary) = spell.cast_primary_target
    {
        consume_beasts(world, caster, primary);
    }
}

fn consume_beasts(world: &mut World, caster: Entity, primary: Entity) {
    let mut consumed = false;
    for (index, ready) in READY.into_iter().enumerate() {
        if !auras::has_aura(world, caster, ready) {
            continue;
        }
        // SimC76f8118 sc_hunter.cpp3776–3810: KC consumes every earned
        // beast. Wyvern Cry remains owned by the real native summon hook.
        if index == 1 {
            spell_cast::cast_triggered(world, caster, 471936, Some(primary), 1);
            spell_cast::cast_triggered(world, caster, 471938, Some(primary), 1);
        } else if !summon_beast(world, caster, SUMMON[index], primary) {
            continue;
        }
        auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
            aura.spell_id == ready
        });
        consumed = true;
    }
    if consumed {
        start_earned_stampede(world, caster, primary);
    }
}

fn start_earned_stampede(world: &mut World, caster: Entity, primary: Entity) {
    if !auras::has_aura(world, caster, INCOMING) {
        return;
    }
    // SimC76f8118 sc_hunter.cpp3813–3817: no Stampede without a
    // consumed Howl. Store the actual primary, not a later mutable target.
    auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
        aura.spell_id == INCOMING
    });
    world.entity_mut(caster).insert(StampedeTarget(primary));
    apply_self(world, caster, STAMPEDE_TRIGGER);
}

fn summon_beast(world: &mut World, caster: Entity, spell: u32, primary: Entity) -> bool {
    use combat::spell_cast::summon::SummonedCreature;
    let owned = |world: &mut World| -> Vec<Entity> {
        world
            .query::<(Entity, &SummonedCreature)>()
            .iter(world)
            .filter(|(_, summon)| summon.owner == caster && summon.spell_id == spell)
            .map(|(unit, _)| unit)
            .collect()
    };
    let before = owned(world);
    // The native summon still chooses its caster-relative destination. Retain
    // the consuming Kill Command's hostile primary for the beast's first action.
    spell_cast::cast_triggered(world, caster, spell, Some(primary), 1);
    let created = owned(world).iter().any(|unit| !before.contains(unit));
    if !created {
        warn!(
            ?caster,
            spell, "Howl retains readiness because native summon did not create an actor"
        );
    }
    created
}

fn stampede_tick(world: &mut World, _: Entity, aura: &AuraInstance, effect: u8) {
    if effect != 1 {
        return;
    }
    let Some(primary) = world
        .get::<StampedeTarget>(aura.caster)
        .map(|target| target.0)
    else {
        return;
    };
    let spell = world
        .resource::<SpellStore>()
        .get(STAMPEDE_TRIGGER)
        .expect("native Stampede trigger")
        .clone();
    let ctx = EffectCtx {
        caster: aura.caster,
        spell: &spell,
        depth: 1,
        dest: None,
        chain_multiplier: 1.0,
    };
    // SimC76f8118 sc_hunter.cpp4885–4900: each native tick, including
    // tick zero, launches three hits with 600/900/1200ms travel delays.
    for delay in [600, 900, 1200] {
        spell_cast::queued_triggers::schedule_full_trigger(
            world,
            &ctx,
            primary,
            STAMPEDE_DAMAGE,
            delay,
        );
    }
}

fn target_plan(spell: u32) -> Option<(targets::Relation, TargetPlanHook)> {
    (spell == STAMPEDE_DAMAGE).then_some((targets::Relation::Enemy, stampede_plan))
}

fn stampede_plan(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    explicit: Option<Entity>,
) -> TargetPlan {
    let Some(primary) = explicit else {
        return Vec::new();
    };
    let Some(center) = targets::position(world, primary) else {
        return Vec::new();
    };
    // SimC76f8118 sc_hunter.cpp4870–4877: Stampede's damage is uncapped
    // target AoE. Its native trigger carries the authored area radius.
    let trigger = world
        .resource::<SpellStore>()
        .get(STAMPEDE_TRIGGER)
        .expect("native Stampede area");
    let radius = targets::effect_radius(&trigger.effects[0]);
    let units = targets::area_units(world, caster, center, radius, targets::Relation::Enemy, 0);
    vec![PlannedEffect {
        index: usize::from(spell.effects[0].index),
        units,
        chain_multipliers: Vec::new(),
        dest: None,
        selected_units: Vec::new(),
        blocked_units: Vec::new(),
        prevent_hit_script: false,
    }]
}
