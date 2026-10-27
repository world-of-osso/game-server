//! Putrefy's real summoned ghoul travels before delivering its native payload.
use super::combat;
use bevy::prelude::*;
use combat::class_progression::TraitLoadout;
use combat::spell_cast::{effects::EffectCtx, script_registry::SpellScriptHook, summon};

const PUTREFY_GHOUL: u32 = 1277098;
const PUTREFY_SINGLE: u32 = 1277016;
const PUTREFY_AREA: u32 = 390220;
const LORD_OF_THE_DEAD: u32 = 1256813;
const MAGUS_SUMMON: u32 = 1242294;
const LORD_SUMMON: u32 = 1292072;
const DREAD_PLAGUE: u32 = 1240996;
const FORBIDDEN_KNOWLEDGE: u32 = 1256566;

#[derive(Component)]
struct PutrefyImpacted;

#[derive(Component, Default)]
struct ForbiddenKnowledgeAttempts(u32);

#[derive(Component)]
struct PendingLordDuration(f64);

#[derive(Component)]
struct LordMagusDuration(f64);

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::SummonInitialized(start_putrefy_ghoul),
    SpellScriptHook::SummonInitialized(set_lord_magus_lifetime),
    SpellScriptHook::SummonInitialized(convert_magi_to_lord),
    SpellScriptHook::SummonInitialized(retain_lord_duration),
    SpellScriptHook::SummonExpired(raise_magus),
    SpellScriptHook::PeriodicDamageFinished(roll_forbidden_knowledge),
    SpellScriptHook::TargetContextDamageMultiplier(lord_duration_multiplier),
];

fn roll_forbidden_knowledge(world: &mut World, ctx: &EffectCtx<'_>) {
    if ctx.spell.id != DREAD_PLAGUE
        || !world.get::<TraitLoadout>(ctx.caster).is_some_and(|traits| {
            traits
                .active_talent_spells()
                .any(|talent| talent.spell_id == FORBIDDEN_KNOWLEDGE)
        })
    {
        return;
    }
    // SimC76f8118 sc_death_knight.cpp7979–7980/14832–14833: each
    // Dread Plague tick rolls the selected native effect3 through PRD.
    let chance = f64::from(combat::spell_cast::scripts::script_effect_value(
        world,
        ctx.caster,
        FORBIDDEN_KNOWLEDGE,
        2,
    )) / 100.0;
    let constant = combat::random::accumulated_proc_constant(chance);
    if world
        .get::<ForbiddenKnowledgeAttempts>(ctx.caster)
        .is_none()
    {
        world
            .entity_mut(ctx.caster)
            .insert(ForbiddenKnowledgeAttempts::default());
    }
    let mut attempts = world
        .get_mut::<ForbiddenKnowledgeAttempts>(ctx.caster)
        .unwrap();
    attempts.0 = attempts.0.saturating_add(1);
    let success = combat::random::random::<f64>() < constant * f64::from(attempts.0);
    if !success {
        return;
    }
    attempts.0 = 0;
    if !combat::spell_cast::try_cast_triggered(
        world,
        ctx.caster,
        PUTREFY_GHOUL,
        Some(ctx.caster),
        1,
    ) {
        warn!(caster = ?ctx.caster, "Forbidden Knowledge Putrefy ghoul summon refused");
    }
}

fn start_putrefy_ghoul(world: &mut World, actor: Entity) {
    let Some(summon) = world.get::<summon::SummonedCreature>(actor) else {
        return;
    };
    if summon.spell_id != PUTREFY_GHOUL {
        return;
    }
    let owner = summon.owner;
    let Some(target) = world
        .get::<combat::networking::CombatTarget>(owner)
        .and_then(|target| target.0)
    else {
        warn!(?owner, "Putrefy ghoul has no selected target");
        return;
    };
    // SimC76f8118 sc_death_knight.cpp3711–3740: the actual ghoul travels,
    // then its owner delivers Putrefy's native payload and dismisses it.
    summon::start_melee_attack(world, actor, target);
    world.entity_mut(actor).insert(summon::SummonArrival {
        target,
        on_arrival: impact_and_dismiss,
    });
}

fn set_lord_magus_lifetime(world: &mut World, actor: Entity) {
    let Some(summon) = world.get::<summon::SummonedCreature>(actor) else {
        return;
    };
    let owner = summon.owner;
    // SimC76f8118 sc_death_knight.cpp6788–6798: Army/DT Magi and Lord
    // Magi use their respective selected native talent durations.
    let (talent, effect) = match summon.spell_id {
        317776 if combat::auras::has_aura(world, owner, 390196) => (390196, 0),
        MAGUS_SUMMON if combat::auras::has_aura(world, owner, LORD_OF_THE_DEAD) => {
            (LORD_OF_THE_DEAD, 1)
        }
        _ => return,
    };
    let duration_ms = combat::spell_cast::scripts::script_effect_value(world, owner, talent, effect);
    assert!(
        duration_ms > 0.0,
        "Lord of the Dead Magus duration must be positive"
    );
    let expiry = combat::spell_cast::now(world) + f64::from(duration_ms) / 1000.0;
    world
        .get_mut::<summon::SummonedCreature>(actor)
        .unwrap()
        .expires_at = Some(expiry);
}

fn convert_magi_to_lord(world: &mut World, actor: Entity) {
    let Some(summon) = world.get::<summon::SummonedCreature>(actor) else {
        return;
    };
    if summon.spell_id != MAGUS_SUMMON {
        return;
    }
    let owner = summon.owner;
    if !combat::auras::has_aura(world, owner, LORD_OF_THE_DEAD) {
        return;
    }
    // SimC76f8118 sc_death_knight.cpp4545–4574: after the actual Magus
    // appears, consume the oldest active Magi to summon the native Lord.
    world.commands().queue(move |world: &mut World| {
        let required =
            combat::spell_cast::scripts::script_effect_value(world, owner, LORD_OF_THE_DEAD, 4)
                as usize;
        assert!(required > 0, "Lord of the Dead requires native Magi count");
        let now = combat::auras::aura_now(world);
        let mut magi: Vec<_> = world
            .query::<(Entity, &summon::SummonedCreature)>()
            .iter(world)
            .filter_map(|(entity, summon)| {
                (summon.owner == owner && summon.spell_id == MAGUS_SUMMON)
                    .then_some(
                        summon
                            .expires_at
                            .filter(|expiry| *expiry > now)
                            .map(|expiry| (entity, expiry)),
                    )
                    .flatten()
            })
            .collect();
        if magi.len() < required {
            return;
        }
        magi.sort_by(|a, b| a.1.total_cmp(&b.1));
        let duration = magi
            .iter()
            .take(required)
            .map(|(_, expiry)| expiry - now)
            .sum();
        world
            .entity_mut(owner)
            .insert(PendingLordDuration(duration));
        for (magus, _) in magi.into_iter().take(required) {
            world
                .get_mut::<summon::SummonedCreature>(magus)
                .unwrap()
                .expires_at = Some(now);
        }
        if !combat::spell_cast::try_cast_triggered(world, owner, LORD_SUMMON, Some(owner), 1) {
            world.entity_mut(owner).remove::<PendingLordDuration>();
            warn!(?owner, "Native Lord of the Dead summon refused");
        }
    });
}

fn retain_lord_duration(world: &mut World, actor: Entity) {
    let Some(summon) = world.get::<summon::SummonedCreature>(actor) else {
        return;
    };
    if summon.spell_id != LORD_SUMMON {
        return;
    }
    let owner = summon.owner;
    let duration = world
        .entity_mut(owner)
        .take::<PendingLordDuration>()
        .expect("Lord summon requires consumed Magus durations");
    world
        .entity_mut(actor)
        .insert(LordMagusDuration(duration.0));
}

fn lord_duration_multiplier(world: &World, ctx: &EffectCtx<'_>, _: Entity, _: bool) -> f32 {
    let Some(duration) = world.get::<LordMagusDuration>(ctx.caster) else {
        return 1.0;
    };
    // SimC76f8118 sc_death_knight.cpp4640–4647: every remaining second
    // of the three consumed Magi adds one percent to the actual Lord's damage.
    1.0 + 0.01 * duration.0 as f32
}

fn impact_and_dismiss(world: &mut World, actor: Entity, target: Entity) {
    let owner = world.get::<summon::SummonedCreature>(actor).unwrap().owner;
    for spell in [PUTREFY_SINGLE, PUTREFY_AREA] {
        if !combat::spell_cast::try_cast_triggered(world, owner, spell, Some(target), 1) {
            warn!(?owner, ?target, spell, "Native Putrefy impact refused");
        }
    }
    world.entity_mut(actor).insert(PutrefyImpacted);
    let now = combat::spell_cast::now(world);
    world
        .get_mut::<summon::SummonedCreature>(actor)
        .unwrap()
        .expires_at = Some(now);
}

fn raise_magus(world: &mut World, actor: Entity) {
    if world.get::<PutrefyImpacted>(actor).is_none() {
        return;
    }
    let owner = world.get::<summon::SummonedCreature>(actor).unwrap().owner;
    if !combat::auras::has_aura(world, owner, LORD_OF_THE_DEAD) {
        return;
    }
    // SimC76f8118 sc_death_knight.cpp3716–3743: the native Lord chance
    // summons a Magus only after the putrefied ghoul is dismissed.
    let chance =
        combat::spell_cast::scripts::script_effect_value(world, owner, LORD_OF_THE_DEAD, 0);
    if combat::random::random::<f32>() * 100.0 >= chance {
        return;
    }
    world.commands().queue(move |world: &mut World| {
        if !combat::spell_cast::try_cast_triggered(world, owner, MAGUS_SUMMON, Some(owner), 1) {
            warn!(?owner, "Native Lord of the Dead Magus summon refused");
        }
    });
}
