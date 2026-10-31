//! Shared Evoker Living/Chrono Flame native hit relationships.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::scripts::script_effect_value;
use combat::spell_cast::{
    self, CastFailure,
    script_registry::{HitHook, SpellScriptHook},
    targets::{self, Relation, relation_matches},
};

#[path = "evoker_flames/chrono_flame.rs"]
mod chrono_flame;

/// Living Flame (Red): DUMMY 0 only.
const LIVING_FLAME: u32 = 361_469;
/// Living Flame damage: SCHOOL_DAMAGE 0.
const LIVING_FLAME_DAMAGE: u32 = 361_500;
/// Living Flame heal.
const LIVING_FLAME_HEAL: u32 = 361_509;
/// Energizing Flame's percentage of the Living Flame mana cost.
const ENERGIZING_FLAME: u32 = 400_006;
/// SimC76f8118 sc_evoker.cpp1321 `evoker.heal_eb_chance` default.
const HEAL_ESSENCE_BURST_CHANCE: f32 = 0.9;
const RUBY_EMBERS: u32 = 365_937;

pub const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::DeclaredReach(declared_reach),
    SpellScriptHook::EffectHit(effect_hit),
    SpellScriptHook::CheckTarget(check_target),
    SpellScriptHook::EffectEnabled(ruby_embers_periodic),
    SpellScriptHook::HealthChangeObserved(chrono_flame::record_health_change),
    SpellScriptHook::RawHeal(record_scarlet_heal),
    SpellScriptHook::NamedAuraFields(&[(431442, 0), (431442, 1), (431442, 2), (1260647, 0)]),
];

fn declared_reach(
    world: &World,
    caster: Entity,
    spell: &combat::spell_info::SpellInfo,
) -> combat::spell_cast::script_registry::ScriptReach {
    let selected =
        combat::spell_cast::script_registry::reach_source_selected(world, caster, 369939)
            || combat::auras::has_aura(world, caster, 370901);
    if spell.id != LIVING_FLAME || !selected {
        return Default::default();
    }
    combat::spell_cast::script_registry::ScriptReach::caster(targets::spell_range(spell, true).1)
}

fn effect_hit(spell: u32, index: u8) -> Option<HitHook> {
    match (spell, index) {
        (LIVING_FLAME, 0) => Some(living_flame),
        (chrono_flame::CHRONO_FLAMES, 0) => Some(ordinary_flame),
        (LIVING_FLAME_DAMAGE | LIVING_FLAME_HEAL, 0) => Some(chrono_flame::repeat_landed_flame),
        _ => None,
    }
}

/// TC a352b1fa spell_evoker.cpp620–647: Living Flame's periodic effect1
/// selects no target unless the caster has Ruby Embers.
fn ruby_embers_periodic(world: &World, caster: Entity, spell: u32, index: u8) -> bool {
    let periodic = matches!(spell, LIVING_FLAME_DAMAGE | LIVING_FLAME_HEAL) && index == 1;
    !periodic || combat::auras::has_aura(world, caster, RUBY_EMBERS)
}

fn check_target(world: &World, _: Entity, spell: u32, _: Entity) -> Result<(), CastFailure> {
    if spell == chrono_flame::CHRONO_FLAMES {
        chrono_flame::check_children(world)?;
    }
    Ok(())
}

fn record_scarlet_heal(world: &mut World, ctx: &spell_cast::effects::EffectCtx<'_>, raw: f32) {
    let caster = ctx.caster;
    if raw <= 0.0 || !combat::auras::has_aura(world, caster, 372469) {
        return;
    }
    let store = world.resource::<combat::spell_info::SpellStore>();
    if !ctx
        .spell
        .class_options
        .as_ref()
        .is_some_and(|options| options.spell_class_set == 224)
    {
        return;
    }
    let coefficient = store
        .get(LIVING_FLAME_DAMAGE)
        .expect("native Living Flame damage")
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("native Living Flame coefficient")
        .bonus_coefficient;
    let cap = combat::spell_cast::effects::power_stats(world, caster, false).0 * coefficient;
    // SimC76f8118 sc_evoker.cpp1320/4347–4362: raw healing, 50% modeled overheal,
    // the selected native talent percentage, and the live Living Flame SP cap.
    let fraction = script_effect_value(world, caster, 372469, 0) / 100.0;
    let added = raw * fraction * 0.5;
    if scarlet_value(world, caster).is_none() {
        combat::auras::apply_aura(
            world,
            &combat::auras::AuraApplication {
                caster,
                target: caster,
                spell_id: 372470,
                effect_mask: u32::MAX,
                source: combat::auras::AuraSource::Cast,
            },
        );
    }
    let (instance, current) = scarlet_value(world, caster).expect("native Scarlet Adaptation aura");
    assert!(combat::auras::set_effect_amount(
        world,
        caster,
        instance,
        0,
        (current + added).min(cap)
    ));
}

fn scarlet_value(world: &World, caster: Entity) -> Option<(u32, f32)> {
    let aura = world
        .get::<combat::auras::ActiveAuras>(caster)?
        .iter()
        .find(|aura| aura.spell_id == 372470 && aura.caster == caster)?;
    let value = aura
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("native Scarlet Adaptation value")
        .amount;
    Some((aura.instance_id, value))
}

fn cast_scarlet_damage(world: &mut World, caster: Entity, target: Entity, depth: u8, bonus: f32) {
    let Some(mut child) = spell_cast::load_triggered_spell(world, LIVING_FLAME_DAMAGE, depth)
    else {
        return;
    };
    child.cast_direct_damage_bonus = bonus;
    if !spell_cast::launch_triggered_spell(world, caster, &child, Some(target), depth) {
        warn!(?caster, ?target, "native Living Flame damage refused");
    }
}

fn take_scarlet(world: &mut World, caster: Entity) -> f32 {
    let value = scarlet_value(world, caster).map_or(0.0, |(_, value)| value);
    combat::auras::remove_auras(
        world,
        caster,
        combat::auras::RemoveReason::Consumed,
        |aura| aura.spell_id == 372470 && aura.caster == caster,
    );
    value
}

/// `spell_evo_living_flame::HandleHitTarget` (spell_evoker.cpp:505-530): heal a
/// unit the caster can assist, damage any other; `HandleLaunchTarget` refunds
/// Energizing Flame's percent of the mana cost on an enemy.
fn living_flame(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    let stacks = world
        .get::<combat::auras::ActiveAuras>(caster)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == 370901 && aura.caster == caster)
        })
        .map_or(0, |aura| aura.stacks);
    combat::auras::remove_auras(
        world,
        caster,
        combat::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 370901 && aura.caster == caster,
    );
    let scarlet = take_scarlet(world, caster);
    if relation_matches(world, caster, target, Relation::Ally) {
        spell_cast::cast_triggered(world, caster, LIVING_FLAME_HEAL, Some(target), depth + 1);
        super::essence_burst::roll_ruby_essence_burst(world, caster);
        return;
    }
    refund_energizing_flame(world, caster);
    cast_scarlet_damage(world, caster, target, depth + 1, scarlet);
    let extras = leaping_extras(world, caster, target, stacks);
    for &extra in &extras {
        cast_scarlet_damage(world, caster, extra, depth + 1, scarlet);
    }
    // SimC76f8118 sc_evoker.cpp6936–6951: unused Leaping Flames heal the caster;
    // each heal hit counts for Ruby Essence Burst at heal_eb_chance.
    let unused = usize::from(stacks) - extras.len();
    let mut heal_hits = 0;
    for _ in 0..unused {
        spell_cast::cast_triggered(world, caster, LIVING_FLAME_HEAL, Some(caster), depth + 1);
        if combat::random::random::<f32>() < HEAL_ESSENCE_BURST_CHANCE {
            heal_hits += 1;
        }
    }
    // sc_evoker.cpp7004–7023: one Ruby roll per damage hit and counted heal hit.
    for _ in 0..1 + extras.len() + heal_hits {
        super::essence_burst::roll_ruby_essence_burst(world, caster);
    }
}

fn leaping_extras(world: &mut World, caster: Entity, target: Entity, stacks: u8) -> Vec<Entity> {
    if stacks == 0 {
        return Vec::new();
    }
    let center = targets::position(world, caster).expect("Living Flame caster position");
    let spell = world
        .resource::<combat::spell_info::SpellStore>()
        .get(LIVING_FLAME)
        .expect("native Living Flame");
    let (_, range) = targets::spell_range(spell, true);
    let mut extras = targets::area_units(world, caster, center, range, Relation::Enemy, 0);
    extras.retain(|&unit| unit != target);
    extras.truncate(usize::from(stacks));
    extras
}

fn ordinary_flame(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    if relation_matches(world, caster, target, Relation::Ally) {
        spell_cast::cast_triggered(world, caster, LIVING_FLAME_HEAL, Some(target), depth + 1);
    } else {
        refund_energizing_flame(world, caster);
        spell_cast::cast_triggered(world, caster, LIVING_FLAME_DAMAGE, Some(target), depth + 1);
    }
}

fn refund_energizing_flame(world: &mut World, caster: Entity) {
    if !combat::auras::has_aura(world, caster, ENERGIZING_FLAME) {
        return;
    }
    let store = world.resource::<combat::spell_info::SpellStore>();
    let (Some(living_flame), Some(energizing)) =
        (store.get(LIVING_FLAME), store.get(ENERGIZING_FLAME))
    else {
        return;
    };
    let percent = energizing
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .map_or(0.0, |effect| effect.base_points);
    let living_flame = living_flame.clone();
    let mana_cost: f32 = spell_cast::applicable_costs(world, caster, &living_flame)
        .unwrap_or_default()
        .into_iter()
        .filter(|(pool, ..)| {
            *pool == spell_cast::CostPool::Power(shared::components::PowerType::Mana)
        })
        .map(|(_, amount, _)| amount)
        .sum();
    if mana_cost > 0.0 {
        spell_cast::powers::change_power(
            world,
            caster,
            shared::components::PowerType::Mana,
            mana_cost * percent / 100.0,
        );
    }
}
