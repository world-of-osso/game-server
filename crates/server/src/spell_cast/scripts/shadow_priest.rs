//! SIMC_SHADOW_TENTACLE_SLAM_CHILDREN: missing native Dummy relationships.
//! SimC76f8118 sc_priest_shadow.cpp1758–1871; native payloads own all amounts.
use bevy::prelude::*;
use rand::seq::SliceRandom;
use shared::components::Position;

use crate::auras::ActiveAuras;
use crate::class_progression::TraitLoadout;
use crate::random as rand;
use crate::spell_info::SpellStore;

pub(super) const SLAM: u32 = 1227280;
const DAMAGE: u32 = 1227621;
const VT: u32 = 34914;
const MADDENING_TENTACLES: u32 = 1279353;
const MADNESS: u32 = 335467;

/// SimC sc_priest_shadow.cpp1819–1826 explicitly authors500ms, absent in data.
/// Keep the override in script policy, not in fabricated native spell records.
pub(super) fn launch_delay(spell: u32, native: f32) -> f32 {
    if spell == SLAM { 0.5 } else { native }
}

pub(super) fn slam_impact(world: &mut World, caster: Entity, primary: Entity, depth: u8) {
    let depth = depth + 1;
    let Some(damage) = load_slam_damage(world, primary, depth) else {
        return;
    };
    let plan =
        match super::super::plan_triggered_spell(world, caster, &damage, Some(primary), depth) {
            Ok(plan) => plan,
            Err(reason) => {
                warn!(?reason, "Tentacle Slam native area selection failed");
                return;
            }
        };
    let units = super::super::selected_units(&plan);
    apply_slam_dots(world, caster, units, depth);
    // sc_priest_shadow.cpp1853–1869: VT before damage, then weak Madness.
    super::super::launch_triggered_spell(world, caster, &damage, Some(primary), depth);
    cast_maddening_tentacles(world, caster, primary, depth);
}

fn load_slam_damage(
    world: &World,
    primary: Entity,
    depth: u8,
) -> Option<crate::spell_info::SpellInfo> {
    let mut damage = super::super::load_triggered_spell(world, DAMAGE, depth)?;
    let Some(position) = world.get::<Position>(primary) else {
        warn!(?primary, "Tentacle Slam primary position unavailable");
        return None;
    };
    damage.cast_destination = Some([position.x, position.y, position.z]);
    Some(damage)
}

fn apply_slam_dots(world: &mut World, caster: Entity, mut units: Vec<Entity>, depth: u8) {
    let cap = world
        .resource::<SpellStore>()
        .get(SLAM)
        .expect("native Tentacle Slam root")
        .effects
        .iter()
        .find(|effect| effect.index == 2)
        .expect("native Tentacle Slam VT cap")
        .base_points
        .max(0.0) as usize;
    // sc_priest_shadow.cpp1758–1785: shuffle only above cap, then prefer fresh VT.
    if units.len() > cap {
        units.shuffle(&mut rand::thread_rng());
        units.sort_by_key(|&target| owns_vt(world, caster, target));
        units.truncate(cap);
    }
    for target in units {
        super::super::cast_triggered(world, caster, VT, Some(target), depth);
    }
}

fn owns_vt(world: &World, caster: Entity, target: Entity) -> bool {
    let now = super::super::now(world);
    world.get::<ActiveAuras>(target).is_some_and(|auras| {
        auras.iter().any(|aura| {
            aura.caster == caster
                && aura.spell_id == VT
                && aura.expires_at.is_none_or(|expiry| expiry > now)
        })
    })
}

fn cast_maddening_tentacles(world: &mut World, caster: Entity, primary: Entity, depth: u8) {
    let selected = world.get::<TraitLoadout>(caster).is_some_and(|traits| {
        traits
            .active_talent_spells()
            .any(|spell| spell.spell_id == MADDENING_TENTACLES)
    });
    if !selected {
        return;
    }
    let Some(multiplier) = maddening_multiplier(world, caster) else {
        warn!(?caster, "Maddening Tentacles native effect unavailable");
        return;
    };
    let Some(mut child) = super::super::load_triggered_spell(world, MADNESS, depth) else {
        return;
    };
    // sc_priest_shadow.cpp1272–1275: a background Madness at talent E0 percent.
    // Native rolling-periodic refresh retains unpaid damage from the previous DoT.
    child.cast_damage_multiplier = Some(multiplier);
    super::super::launch_triggered_spell(world, caster, &child, Some(primary), depth);
}

fn maddening_multiplier(world: &World, caster: Entity) -> Option<f32> {
    let aura = world
        .get::<ActiveAuras>(caster)?
        .iter()
        .find(|aura| aura.caster == caster && aura.spell_id == MADDENING_TENTACLES)?;
    let effect = aura.effects.iter().find(|effect| effect.index == 0)?;
    Some(effect.amount / 100.0)
}
