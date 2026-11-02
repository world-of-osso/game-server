//! SimC76f8118 sc_priest_shadow.cpp1491–1514/sc_priest.cpp1606–1683/3543–3562.
//! Native Torrent starts a hasted Rift driver at its actual hostile primary.
use super::{combat, shadow_collapse, shadow_priest, shadow_void_blast, shadow_voidheart};
use bevy::prelude::*;
use combat::auras::scripts::{AuraScript, AuraScripts};
use combat::auras::{self, ActiveAuras, AuraInstance, RemoveReason};
use combat::spell_cast::script_registry::{SpellScriptHook, TargetPlanHook};
use combat::spell_cast::{
    self, PlannedEffect, TargetPlan, effects::EffectCtx, summon::SummonedCreature, targets,
};
use combat::spell_info::{SpellInfo, SpellStore};
use shared::components::{ActiveSpec, Health};

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::DeclaredReach(declared_reach),
    SpellScriptHook::AfterAuraApplication(start_rift),
    SpellScriptHook::AfterElementalLaunch(grow_rift),
    SpellScriptHook::EarlyAuraRegistration(register),
    SpellScriptHook::TargetPlan(target_plan),
    SpellScriptHook::DotMasteryMultiplier(voidwraith_mastery),
    SpellScriptHook::TargetContextDamageMultiplier(voidwraith_health_bonus),
    SpellScriptHook::ResolvedDamage(voidwraith_insanity),
    SpellScriptHook::ConsumedEffectValues(&[(VOID_FLAY, 1)]),
];
const TORRENT: u32 = 263165;
const DRIVER: u32 = 459314;
const OBJECT: u32 = 447445;
const DAMAGE: u32 = 447448;
const VOIDWRAITH_TALENT: u32 = 451234;
const VOIDWRAITH: u32 = 451235;
const VOID_FLAY: u32 = 451435;
const SHADOWFIEND_INSANITY: u32 = 262485;
const MAX_GROWTH: u8 = 5;
const GROWTH_RADIUS_INCREMENT: f32 = 0.5;

fn declared_reach(
    world: &World,
    _caster: Entity,
    spell: &SpellInfo,
) -> combat::spell_cast::script_registry::ScriptReach {
    if !matches!(spell.id, TORRENT | DRIVER | DAMAGE) {
        return Default::default();
    }
    combat::spell_cast::script_registry::ScriptReach::target(
        read_base_radius(world) + f32::from(MAX_GROWTH) * GROWTH_RADIUS_INCREMENT,
    )
}

#[derive(Component)]
struct Rift {
    primary: Entity,
    ticks: u32,
    growth: u8,
}

fn start_rift(world: &mut World, primary: Entity, aura: &AuraInstance) {
    let caster = aura.caster;
    if aura.spell_id != TORRENT || primary == caster {
        return;
    }
    let shadow = world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 258);
    if !shadow {
        return;
    }
    // Native aura effects bypass ordinary EffectHit dispatch. The stored enemy
    // Torrent aura supplies its actual recipient before its first periodic update.
    // SimC1677–1681 expires the old window before starting the new driver clock.
    auras::remove_auras(world, caster, RemoveReason::Cancelled, |aura| {
        aura.spell_id == DRIVER && aura.caster == caster
    });
    world.entity_mut(caster).insert(Rift {
        primary,
        ticks: 0,
        growth: 0,
    });
    spell_cast::cast_triggered(world, caster, DRIVER, Some(caster), 1);
}

fn register(scripts: &mut AuraScripts) {
    scripts.0.insert(
        DRIVER,
        AuraScript {
            on_periodic: Some(pulse),
            on_remove: Some(clear_rift),
            ..Default::default()
        },
    );
    scripts.0.entry(TORRENT).or_default().on_remove = Some(finish_channel);
}

fn pulse(world: &mut World, unit: Entity, _: &AuraInstance, index: u8) {
    if index != 0 {
        return;
    }
    let Some(mut rift) = world.get_mut::<Rift>(unit) else {
        return;
    };
    rift.ticks += 1;
    // SimC3555–3562: the first travelling tick cannot reliably reach enemies.
    if rift.ticks < 2 {
        return;
    }
    let primary = rift.primary;
    let growth = rift.growth;
    let Some(mut damage) = spell_cast::load_triggered_spell(world, DAMAGE, 1) else {
        return;
    };
    damage.cast_damage_multiplier = Some(shadow_collapse::growth_multiplier(world, unit, growth));
    spell_cast::launch_triggered_spell(world, unit, &damage, Some(primary), 1);
}

fn grow_rift(world: &mut World, caster: Entity, spell: &SpellInfo, foreground: bool) {
    if !foreground || spell.id != 335467 || !auras::has_aura(world, caster, 448403) {
        return;
    }
    if let Some(mut rift) = world.get_mut::<Rift>(caster) {
        rift.growth = (rift.growth + 1).min(MAX_GROWTH);
    }
}

fn clear_rift(world: &mut World, unit: Entity, _: &AuraInstance, _: RemoveReason) {
    let rift = world
        .get::<Rift>(unit)
        .map(|rift| (rift.primary, rift.growth));
    if let Some((primary, growth)) = rift {
        // SimC3568–3571 resolves the payload before removing Voidheart.
        shadow_collapse::burst(world, unit, primary, growth);
    }
    shadow_void_blast::expire_override(world, unit);
    shadow_voidheart::replace_with_touch_tail(world, unit);
    if let Some((primary, _)) = rift
        && auras::has_aura(world, unit, VOIDWRAITH_TALENT)
    {
        summon_voidwraith(world, unit, primary);
    }
    world.entity_mut(unit).remove::<Rift>();
}

fn summon_voidwraith(world: &mut World, caster: Entity, primary: Entity) {
    let Some(position) = targets::position(world, primary) else {
        warn!(?primary, "Voidwraith Rift position unavailable");
        return;
    };
    let Some(mut summon) = spell_cast::load_triggered_spell(world, VOIDWRAITH, 1) else {
        return;
    };
    summon.cast_destination = Some(position.to_array());
    if !spell_cast::launch_triggered_spell(world, caster, &summon, None, 1) {
        warn!(?caster, "Voidwraith summon rejected");
    }
}

fn voidwraith_owner(world: &World, caster: Entity, spell: u32) -> Option<Entity> {
    let summon = world.get::<SummonedCreature>(caster)?;
    let shadow = world
        .get::<ActiveSpec>(summon.owner)
        .is_some_and(|spec| spec.0 == 258);
    (spell == VOID_FLAY && summon.spell_id == VOIDWRAITH && shadow).then_some(summon.owner)
}

fn voidwraith_mastery(world: &World, caster: Entity, target: Entity, spell: u32) -> f32 {
    let Some(owner) = voidwraith_owner(world, caster, spell) else {
        return 1.0;
    };
    shadow_priest::weaving_multiplier(world, owner, target, spell)
}

fn voidwraith_health_bonus(world: &World, ctx: &EffectCtx<'_>, target: Entity, _: bool) -> f32 {
    if voidwraith_owner(world, ctx.caster, ctx.spell.id).is_none() {
        return 1.0;
    }
    let Some(health) = world.get::<Health>(target) else {
        return 1.0;
    };
    let percent = spell_cast::scripts::script_effect_value(world, ctx.caster, VOID_FLAY, 1) / 100.0;
    1.0 + percent * (health.current / health.max).clamp(0.0, 1.0)
}

fn voidwraith_insanity(world: &mut World, ctx: &EffectCtx<'_>, _: Entity, _: bool, damage: f32) {
    if damage <= 0.0 {
        return;
    }
    if let Some(owner) = voidwraith_owner(world, ctx.caster, ctx.spell.id) {
        spell_cast::cast_triggered(
            world,
            owner,
            SHADOWFIEND_INSANITY,
            Some(owner),
            ctx.depth + 1,
        );
    }
}

fn finish_channel(world: &mut World, unit: Entity, aura: &AuraInstance, _: RemoveReason) {
    // The root has both enemy and caster auras; perform last_tick once at the caster.
    if unit != aura.caster {
        return;
    }
    // SimC1491–1501 restores each buff's own native duration, not tick phase.
    for spell in [DRIVER, shadow_voidheart::BUFF] {
        let duration = world.get::<ActiveAuras>(unit).and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.caster == unit && aura.spell_id == spell)
                .map(|aura| aura.duration_ms)
        });
        if let Some(duration) = duration {
            auras::set_owned_duration(world, unit, unit, spell, duration);
        }
    }
}

fn target_plan(spell: u32) -> Option<(targets::Relation, TargetPlanHook)> {
    (spell == DAMAGE).then_some((targets::Relation::Enemy, plan_pulse))
}

fn plan_pulse(
    world: &mut World,
    caster: Entity,
    _: &SpellInfo,
    primary: Option<Entity>,
) -> TargetPlan {
    let primary = primary.expect("native Rift damage retains source primary");
    let Some(center) = targets::position(world, primary) else {
        return Vec::new();
    };
    let growth = world.get::<Rift>(caster).map_or(0, |rift| rift.growth);
    // Pinned SimC bugs=true applies half a yard for each growth stack.
    let radius = read_base_radius(world) + f32::from(growth) * GROWTH_RADIUS_INCREMENT;
    let units = targets::area_units(world, caster, center, radius, targets::Relation::Enemy, 0);
    vec![PlannedEffect {
        index: 0,
        units,
        dest: None,
        chain_multipliers: Vec::new(),
        selected_units: Vec::new(),
        blocked_units: Vec::new(),
        missed_units: Vec::new(),
        prevent_hit_script: false,
    }]
}

fn read_base_radius(world: &World) -> f32 {
    // SimC1611: the native object radius, halved, not the explicit damage row's range.
    world
        .resource::<SpellStore>()
        .get(OBJECT)
        .expect("native Rift object")
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("native Rift radius effect")
        .radius[0]
        .expect("native Rift radius")
        .radius_max
        / 2.0
}
