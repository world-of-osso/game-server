//! Hunter-only source inheritance: TC Pet.cpp879/898–925 and StatSystem1187–1310.
use bevy::prelude::*;
use shared::components::Health;

use crate::hunter_pet::HunterPet;
use crate::level_scaling::CreatureArmor;
use crate::stats::CharacterStats;
use crate::world_data::{CreatureTemplate, WorldData};

const OWNER_STAMINA_FRACTION: f32 = 0.30;
const OWNER_ARMOR_FRACTION: f32 = 0.70;
const HEALTH_PER_BONUS_STAMINA: f32 = 10.0;
const BASE_ARMOR_PER_LEVEL: f32 = 50.0;

#[derive(Component)]
pub(crate) struct HunterInheritedStats {
    level: u8,
    bonus_health: f32,
}

fn owner_bonus_health(owner: &CharacterStats) -> f32 {
    // Guardian::UpdateStats stores the resulting stat as int32 before HP math.
    (owner.primary.stamina * OWNER_STAMINA_FRACTION).trunc() * HEALTH_PER_BONUS_STAMINA
}

fn inherited_armor(level: u8, owner: &CharacterStats) -> f32 {
    (f32::from(level) * BASE_ARMOR_PER_LEVEL + owner.armor * OWNER_ARMOR_FRACTION).trunc()
}

pub(crate) fn native_base_health(data: &WorldData, template: &CreatureTemplate, level: u8) -> f32 {
    let tuning = template
        .tuning
        .as_ref()
        .map_or(0, |tuning| tuning.content_tuning_id);
    let (health, _) = data.expected_stats.creature_health_armor(
        level,
        template.health_scaling_expansion,
        tuning,
        template.unit_class,
    );
    // pet_levelstats is not imported. Use TC's ExpectedStat create-health path,
    // never the legacy class-level placeholder or TC's explicitly fake stats.
    (health * template.health_modifier).max(1.0).trunc()
}

pub(crate) fn spawn_stats(
    level: u8,
    base_health: f32,
    owner: &CharacterStats,
) -> (f32, CreatureArmor, HunterInheritedStats) {
    let bonus_health = owner_bonus_health(owner);
    (
        base_health + bonus_health,
        CreatureArmor(inherited_armor(level, owner)),
        HunterInheritedStats {
            level,
            bonus_health,
        },
    )
}

/// TC SetMaxHealth preserves current HP on growth and clamps it on shrink.
/// Apply only the inherited delta so unrelated pet health auras stay intact.
pub(crate) fn refresh_owner_stats(
    owners: Query<&CharacterStats, Changed<CharacterStats>>,
    mut pets: Query<(
        &HunterPet,
        &mut HunterInheritedStats,
        &mut Health,
        &mut CreatureArmor,
    )>,
) {
    for (pet, mut inherited, mut health, mut armor) in &mut pets {
        let Ok(owner) = owners.get(pet.owner) else {
            continue;
        };
        let bonus = owner_bonus_health(owner);
        let max = (health.max + bonus - inherited.bonus_health).max(1.0);
        if health.max != max {
            health.max = max;
            health.current = health.current.min(max);
        }
        inherited.bonus_health = bonus;
        let value = inherited_armor(inherited.level, owner);
        if armor.0 != value {
            armor.0 = value;
        }
    }
}
