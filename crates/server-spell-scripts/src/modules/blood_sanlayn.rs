//! Blood San'layn's cooldown-driven Gift of the San'layn.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, RemoveReason};
use combat::class_progression::KnownSpells;
use combat::spell_cast::{
    self, TriggeredSpellOverrides,
    effects::{self, EffectCtx, EffectHandler},
    script_registry::SpellScriptHook,
};
use combat::spell_info::SpellStore;
use combat::spell_info::{SpellEffect, SpellInfo};
use shared::components::{ActiveSpec, Health};

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AfterForegroundSpecial(grant_gift),
    SpellScriptHook::AfterForegroundSpecial(vampiric_proc),
    SpellScriptHook::EffectHandler(vampiric_heal_handler),
    SpellScriptHook::EffectEnabled(blood_vampiric_damage_effect),
    SpellScriptHook::AfterPolicyDirectDamage(infliction_of_sorrow),
    SpellScriptHook::TargetFilter(limit_vampiric_targets),
    SpellScriptHook::AfterForegroundSpecial(summon_blood_beast),
    SpellScriptHook::SummonInitialized(initialize_blood_beast),
    SpellScriptHook::LandedDamagePolicy(accumulate_beast_damage),
    SpellScriptHook::SummonExpired(erupt_blood_beast),
];

#[derive(Component, Default)]
struct BloodBeastDamage(f32);

// SimC76f8118 sc_death_knight.cpp4692–4793/9529–9530: the real beast
// attacks, records its actual damage and erupts when its summon expires.
fn summon_blood_beast(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if spell.id == 49028
        && world
            .get::<ActiveSpec>(caster)
            .is_some_and(|spec| spec.0 == 250)
        && world
            .get::<KnownSpells>(caster)
            .is_some_and(|known| known.contains(434260))
    {
        spell_cast::cast_triggered(world, caster, 434237, Some(caster), 1);
    }
}

fn initialize_blood_beast(world: &mut World, beast: Entity) {
    let Some(summon) = world.get::<spell_cast::summon::SummonedCreature>(beast) else {
        return;
    };
    if summon.spell_id != 434237 {
        return;
    }
    let owner = summon.owner;
    world.entity_mut(beast).insert((
        BloodBeastDamage::default(),
        combat::combat::CreatureAttackTime(1.0),
    ));
    let Some(target) = world
        .get::<combat::networking::CombatTarget>(owner)
        .and_then(|target| target.0)
    else {
        warn!(?owner, "Blood Beast has no selected target");
        return;
    };
    spell_cast::summon::start_melee_attack(world, beast, target);
}

fn accumulate_beast_damage(
    world: &mut World,
    attacker: Entity,
    _: Entity,
    spell: Option<u32>,
    amount: f32,
    periodic: bool,
) {
    if !periodic
        && matches!(spell, None | Some(434574))
        && let Some(mut state) = world.get_mut::<BloodBeastDamage>(attacker)
    {
        state.0 += amount.max(0.0);
    }
}

fn erupt_blood_beast(world: &mut World, beast: Entity) {
    let Some(state) = world.get::<BloodBeastDamage>(beast) else {
        return;
    };
    let damage = state.0;
    let owner = world
        .get::<spell_cast::summon::SummonedCreature>(beast)
        .expect("Blood Beast summon")
        .owner;
    let Some(target) = world
        .get::<combat::networking::CombatTarget>(owner)
        .and_then(|target| target.0)
    else {
        return;
    };
    let Some(position) = world.get::<shared::components::Position>(target).copied() else {
        return;
    };
    let percent = world
        .resource::<SpellStore>()
        .get(434260)
        .expect("The Blood is Life")
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("Blood eruption percentage")
        .base_points
        / 100.0;
    let Some(mut eruption) = spell_cast::load_triggered_spell(world, 434246, 1) else {
        return;
    };
    let effect = eruption
        .effects
        .iter_mut()
        .find(|effect| effect.index == 0)
        .expect("Blood eruption damage");
    effect.base_points = damage * percent;
    eruption.cast_destination = Some([position.x, position.y, position.z]);
    eruption
        .misc
        .as_mut()
        .expect("Blood eruption attributes")
        .attributes[2] |= 0x2000_0000;
    spell_cast::launch_triggered_spell(world, owner, &eruption, Some(target), 1);
}

// SimC76f8118 sc_death_knight.cpp11325–11329: Blood's Vampiric Strike
// uses effect 5, not the separate Unholy damage effect 1.
fn blood_vampiric_damage_effect(world: &World, caster: Entity, spell: u32, index: u8) -> bool {
    !(spell == 433895
        && index == 0
        && world
            .get::<ActiveSpec>(caster)
            .is_some_and(|spec| spec.0 == 250))
}

// SimC76f8118 sc_death_knight.cpp13778–13830: only an actual owned
// Blood Plague supplies remaining damage; extend it without restarting ticks.
fn infliction_of_sorrow(world: &mut World, ctx: &EffectCtx<'_>, target: Entity, _: bool) {
    if ctx.spell.id != 433895
        || !world
            .get::<ActiveSpec>(ctx.caster)
            .is_some_and(|spec| spec.0 == 250)
        || !world
            .get::<KnownSpells>(ctx.caster)
            .is_some_and(|known| known.contains(434143))
    {
        return;
    }
    let Some(plague) = world
        .get::<auras::ActiveAuras>(target)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == 55078 && aura.caster == ctx.caster)
        })
        .cloned()
    else {
        return;
    };
    let Some(expiry) = plague.expires_at else {
        return;
    };
    let Some(state) = plague
        .effects
        .iter()
        .find(|effect| effect.index == 0 && effect.period_ms > 0)
    else {
        return;
    };
    let now = auras::aura_now(world);
    if expiry <= now {
        return;
    }
    let period = f64::from(state.period_ms) / 1000.0;
    let ticks = (1.0 + (expiry - state.next_tick_at()) / period).max(0.0) as f32;
    let store = world.resource::<SpellStore>();
    let disease = store.get(55078).expect("active Blood Plague spell");
    let effect = disease
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("Blood Plague damage effect");
    let talent = store.get(434143).expect("Infliction of Sorrow talent");
    let base_percent = talent
        .effects
        .iter()
        .find(|effect| effect.index == 1)
        .expect("Infliction damage percent")
        .base_points;
    // SimC76f8118 sc_death_knight.cpp13821 reads the modified talent effect;
    // Blood's 137008 effect 14 adds its native points through label 4138.
    let percent = auras::spell_mods::modify(
        world,
        ctx.caster,
        talent,
        auras::spell_mods::SpellModOp::PointsIndex1,
        base_percent,
    ) / 100.0;
    let extension = talent
        .effects
        .iter()
        .find(|effect| effect.index == 2)
        .expect("Infliction extension")
        .base_points as u32;
    let disease_ctx = EffectCtx {
        caster: ctx.caster,
        spell: disease,
        depth: ctx.depth,
        dest: None,
        chain_multiplier: 1.0,
    };
    let damage = effects::DamageInput {
        amount: auras::periodic_live_amount(world, ctx.caster, disease, state)
            * plague.stack_multiplier(),
        crit: false,
        periodic: true,
        resolved_source: false,
        effect: Some(effect),
        bonus_caster: ctx.caster,
    };
    let remaining = effects::damage_before_mitigation(world, &disease_ctx, target, &damage) * ticks;
    auras::extend_owned_duration(world, ctx.caster, target, &[55078], extension);
    if remaining > 0.0 {
        spell_cast::cast_triggered_with_effect0_amount(
            world,
            ctx.caster,
            434144,
            Some(target),
            ctx.depth + 1,
            remaining * percent,
        );
    }
}

// SimC76f8118 sc_death_knight.cpp11234–11253: Blood Vampiric Strike
// shares Heart Strike's target cap and Death and Decay bonus.
fn limit_vampiric_targets(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    effect: u8,
    primary: Option<Entity>,
    targets: &mut Vec<Entity>,
) {
    if spell.id != 433895
        || effect != 4
        || !world
            .get::<ActiveSpec>(caster)
            .is_some_and(|spec| spec.0 == 250)
    {
        return;
    }
    let store = world.resource::<SpellStore>();
    let heart = store.get(206930).expect("Blood Heart Strike");
    let base = heart
        .effects
        .iter()
        .find(|effect| effect.index == 1)
        .expect("Heart Strike damage")
        .chain_targets
        .max(0) as usize;
    let bonus = world
        .get::<auras::ActiveAuras>(caster)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == 188290 && aura.caster == caster)
                .and_then(|aura| aura.effects.iter().find(|effect| effect.index == 2))
                .map(|effect| effect.amount.max(0.0) as usize)
        })
        .unwrap_or(0);
    if let Some(primary) = primary
        && let Some(index) = targets.iter().position(|target| *target == primary)
    {
        targets.swap(0, index);
    }
    targets.truncate(base + bonus);
}

fn vampiric_heal_handler(spell: u32, index: u8) -> Option<EffectHandler> {
    (spell == 434422 && index == 0).then_some(vampiric_heal)
}

// SimC76f8118 sc_death_knight.cpp8206–8216: Blood uses the native second effect's health percentage.
fn vampiric_heal(world: &mut World, ctx: &EffectCtx<'_>, effect: &SpellEffect, target: Entity) {
    if !world
        .get::<ActiveSpec>(ctx.caster)
        .is_some_and(|spec| spec.0 == 250)
    {
        effects::handler(effects::EFFECT_HEAL_PCT).expect("native percentage heal")(
            world, ctx, effect, target,
        );
        return;
    }
    let percent = ctx
        .spell
        .effects
        .iter()
        .find(|effect| effect.index == 1)
        .expect("Blood Vampiric Strike heal percentage")
        .base_points;
    let max = world
        .get::<Health>(target)
        .expect("Vampiric Strike target health")
        .max;
    let mut heal = effect.clone();
    heal.effect = effects::EFFECT_HEAL;
    heal.base_points = max * percent / 100.0;
    effects::handler(effects::EFFECT_HEAL).expect("native critical heal")(
        world, ctx, &heal, target,
    );
}

// SimC76f8118 sc_death_knight.cpp7279–7300/9531–9536: Vampiric Blood
// and Dancing Rune Weapon grant Gift for their native duration.
fn grant_gift(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if !matches!(spell.id, 49028 | 55233)
        || !world
            .get::<ActiveSpec>(caster)
            .is_some_and(|spec| spec.0 == 250)
        || !world
            .get::<KnownSpells>(caster)
            .is_some_and(|known| known.contains(434152))
    {
        return;
    }
    let base_duration = spell
        .misc
        .as_ref()
        .and_then(|misc| misc.duration)
        .expect("Blood cooldown duration")
        .duration_ms;
    let duration = auras::spell_mods::modify(
        world,
        caster,
        spell,
        auras::SpellModOp::Duration,
        base_duration as f32,
    )
    .round() as i32;
    spell_cast::cast_triggered_with_overrides(
        world,
        caster,
        434153,
        Some(caster),
        1,
        TriggeredSpellOverrides {
            duration_ms: Some(duration),
            ..Default::default()
        },
    );
    spell_cast::cast_triggered(world, caster, 433899, Some(caster), 1);
}

// SimC76f8118 sc_death_knight.cpp10131–10134/13835–13853: Death Strike
// may earn a replacement outside Gift; the replacement consumes the proc.
fn vampiric_proc(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if !world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 250)
    {
        return;
    }
    let gift = auras::has_aura(world, caster, 434153);
    if spell.id == 433895 && !gift {
        auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
            aura.spell_id == 433899 && aura.caster == caster
        });
        return;
    }
    if spell.id != 49998
        || gift
        || !world
            .get::<KnownSpells>(caster)
            .is_some_and(|known| known.contains(433901))
    {
        return;
    }
    let store = world.resource::<SpellStore>();
    let effect = |id, index| {
        store
            .get(id)
            .expect("Sanlayn spell")
            .effects
            .iter()
            .find(|effect| effect.index == index)
            .expect("Sanlayn effect")
            .base_points
    };
    let mut chance = effect(433901, 0);
    if world
        .get::<KnownSpells>(caster)
        .is_some_and(|known| known.contains(434263))
        && spell
            .cast_primary_target
            .and_then(|target| world.get::<Health>(target))
            .is_some_and(|health| {
                health.max > 0.0 && health.current * 100.0 <= health.max * effect(434263, 0)
            })
    {
        chance += effect(434263, 1);
    }
    if combat::random::random::<f32>() * 100.0 < chance {
        spell_cast::cast_triggered(world, caster, 433899, Some(caster), 1);
    }
}
