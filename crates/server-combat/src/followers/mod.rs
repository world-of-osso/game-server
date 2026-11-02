//! User-approved player-spec combat bridge for authenticated Retail followers.
#[cfg(test)]
mod action_knowledge_tests;
mod action_spells;
mod ai;
mod candlelight;
#[cfg(test)]
mod cast_while_casting_tests;
mod catalog;
pub mod class_profiles;
pub mod class_spawn;
#[cfg(test)]
mod darkflame_drop_tests;
#[cfg(test)]
mod floodgate_healer_tests;
#[cfg(test)]
mod form_combat_tests;
pub mod gear;
mod hazards;
#[cfg(test)]
mod holy_policy_native_tests;
#[cfg(test)]
mod holy_priest_tests;
#[cfg(test)]
mod holy_threat_tests;
#[cfg(test)]
mod kit_empower_tests;
#[cfg(test)]
mod kit_hunter_policy_tests;
#[cfg(test)]
mod kit_light_of_titans_tests;
pub(crate) mod live_facts;
#[cfg(test)]
mod mistweaver_defensive_tests;
#[cfg(test)]
mod native_action_override_tests;
#[cfg(test)]
mod native_book_tests;
#[cfg(test)]
mod proc_definition_tests;
pub(crate) mod transport_follow;
#[cfg(test)]
mod transport_follow_tests;

#[cfg(test)]
mod fire_efficiency_tests;
#[cfg(test)]
mod formation_tests;
#[cfg(test)]
mod giantkiller_tests;
#[cfg(test)]
mod intensifying_flame_tests;
#[cfg(test)]
mod kit_elemental_resource_tests;
#[cfg(test)]
mod kit_proc_fidelity_tests;
#[cfg(test)]
mod kit_rotation_fidelity_tests;
#[cfg(test)]
mod meteor_tests;
mod owner_trail;
mod pack_identity;
pub mod parity_profile;
pub(crate) mod party;
mod positioning;
pub(crate) use positioning::CombatMovementExclusions;
mod pull_admission;
#[cfg(test)]
mod pull_admission_tests;
mod release;
mod target_reach;
use server_follower_policy::followers::pull_budget;
#[cfg(test)]
mod pull_corner_tests;
#[cfg(test)]
mod pull_eligibility_tests;
#[cfg(test)]
mod pull_endpoint_tests;
#[cfg(test)]
mod pull_forecast_tests;
#[cfg(test)]
mod pull_goal_tests;
pub(crate) mod pull_history;
#[cfg(test)]
mod pull_history_tests;
#[cfg(test)]
mod pull_position_tests;
mod pull_readiness;
#[cfg(test)]
mod pull_readiness_tests;
mod pull_route;
#[cfg(test)]
mod pull_route_tests;
mod pull_set;
mod pull_sources;
#[cfg(test)]
mod pull_sources_tests;
#[cfg(test)]
mod pull_stat_provenance_tests;
pub mod retreat;
#[cfg(test)]
mod rotation_dummy_tests;
#[cfg(any(test, feature = "test-support"))]
pub mod simulator;
#[cfg(any(test, feature = "test-support"))]
mod spawning;
pub mod spec_selection;
mod spell_safety;
mod stockade_diagnostic;
#[cfg(any(test, feature = "test-support"))]
pub mod stockade_scenario;
mod tank_frontal;
mod tank_planner;
#[cfg(test)]
mod tank_planner_tests;
mod tank_pull_back;
#[cfg(test)]
mod tank_pull_back_native_tests;
mod tank_pull_back_party;
mod trail_follow;
mod unintended_reach;
#[cfg(test)]
mod unintended_reach_tests;
pub fn is_boss(world: &World, unit: Entity) -> bool {
    const DUNGEON_BOSS_FLAG: u32 = 0x1000_0000;
    const WORLD_BOSS_FLAG: u32 = 0x4;
    let Some(npc) = world.get::<shared::components::Npc>(unit) else {
        return false;
    };
    world
        .resource::<crate::world_data::WorldData>()
        .templates
        .get(&npc.template_id)
        .is_some_and(|template| {
            template.flags_extra & DUNGEON_BOSS_FLAG != 0
                || template.type_flags & WORLD_BOSS_FLAG != 0
        })
}
#[cfg(test)]
mod utility_buff_tests;
#[cfg(test)]
mod utility_policy_tests;
pub mod visual;

use bevy::prelude::*;
use shared::group::GroupRole;

/// A follower installed from a player class profile, not a story NPC.
#[derive(Component)]
pub struct ClassCombatant;

pub(crate) fn player_like_combatant(player: bool, class_combatant: bool) -> bool {
    player || class_combatant
}

pub(crate) fn is_player_like_combatant(world: &World, unit: Entity) -> bool {
    player_like_combatant(
        world.get::<shared::components::Player>(unit).is_some(),
        world.get::<ClassCombatant>(unit).is_some(),
    )
}

#[cfg(test)]
mod group_buff_tests;

pub use ai::{AplControlled, FollowerReach, acting_follower, engaged_targets, party_units};

pub(crate) fn mark_owner_returning(world: &mut World, owner: Entity) {
    if party::read_admission(world, owner).is_some() {
        world
            .entity_mut(owner)
            .insert(release::ReturningFromGraveyard);
    }
}

/// Whether a follower is approaching or casting a resurrection, or the owner has an offer.
pub fn resurrection_pending(world: &mut World, owner: Entity) -> bool {
    world
        .get::<crate::death::resurrection::PendingResurrection>(owner)
        .is_some()
        || world
            .query::<(&Follower, &release::ResurrectionApproach)>()
            .iter(world)
            .any(|(follower, _)| follower.owner == owner)
}

#[cfg(feature = "test-support")]
pub fn set_test_pull_back(world: &mut World, owner: Entity, tank: Entity) {
    world
        .entity_mut(owner)
        .insert(tank_pull_back::WaitingForPullBack(tank));
}

#[cfg(feature = "test-support")]
pub fn test_ready_heal(
    world: &mut World,
    tank: Entity,
    party: &[Entity],
) -> Result<Option<(Entity, u32)>, String> {
    Ok(pull_readiness::read_ready_heal(world, tank, party)?
        .map(|ready| (ready.healer, ready.spell_id)))
}

/// Program-bound class actions and selected equipped on-use spells.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct FollowerActionSpells(pub Vec<u32>);

/// Installed by the class APL host before any class follower spawns.
#[derive(Resource, Clone, Copy)]
pub struct FollowerActionProvider {
    /// Configure acquisition observers before passive auras are installed.
    pub before_passives: fn(&mut World, Entity, u32) -> Result<(), String>,
    /// Program-bound class actions, available after class progression and before gear.
    pub source_actions: fn(&mut World, Entity, u32) -> Result<Vec<u32>, String>,
    /// Program-selected equipped on-use actions, available after gear is equipped.
    pub equipped_actions: fn(&mut World, Entity, u32) -> Result<Vec<u32>, String>,
}

/// Whether this class follower must hold damage while its tank repositions a pull.
pub fn holding_pull_damage(world: &World, unit: Entity) -> bool {
    world.get::<AplControlled>(unit).is_some()
        && world.get::<Follower>(unit).is_some_and(|follower| {
            follower.profile.role != GroupRole::Tank
                && tank_pull_back::party_should_wait(world, follower.owner)
        })
}
pub use catalog::{FollowerCatalog, load_catalog};
/// A tank-role follower, or a unit whose active spec has the tank role.
pub fn is_tank(world: &World, unit: Entity) -> bool {
    if world
        .get::<Follower>(unit)
        .is_some_and(|follower| follower.profile.role == GroupRole::Tank)
    {
        return true;
    }
    let Some(spec) = world.get::<crate::class_progression::ActiveSpec>(unit) else {
        return false;
    };
    world
        .get_resource::<crate::spell_info::ClassSpells>()
        .is_some_and(|classes| {
            classes
                .classes
                .iter()
                .flat_map(|class| &class.specs)
                .any(|entry| entry.spec_id == spec.0 && entry.role == 0)
        })
}
pub use live_facts::{ObservedEvents, observe_unit};
pub(crate) use party::{DisconnectedRuns, queue_player, reconnect, stage_follower_party};
pub(crate) use spell_safety::summon_targets_safe;
pub use spell_safety::{action_safe, chain_candidate_allowed};
pub use unintended_reach::{party_unintended_reach, spell_unintended_reach};

#[derive(Clone, Debug)]
pub struct Profile {
    pub entry: u32,
    pub name: String,
    pub class_id: u8,
    pub spec_id: u32,
    pub role: GroupRole,
    pub attack_spell: u32,
    pub cooldown_spell: u32,
    pub heal_spell: u32,
    pub taunt_spell: u32,
    pub form_spell: u32,
    pub melee: bool,
}

#[derive(Component, Clone)]
pub struct Follower {
    pub owner: Entity,
    pub profile: Profile,
}

impl Follower {
    /// Runtime-only companion identities occupy the high half of the group ID space;
    /// they are not persisted characters and never enter auth/character storage.
    pub fn member_id(entity: Entity) -> u64 {
        entity.to_bits() | (1 << 63)
    }

    pub fn online_character(
        &self,
        entity: Entity,
        level: u8,
        dungeon_difficulty: u32,
        visual: &shared::components::CharacterVisual,
        equipment: Option<&shared::components::EquipmentAppearance>,
    ) -> crate::group::OnlineMember {
        let head = equipment
            .and_then(|equipment| {
                equipment
                    .entries
                    .iter()
                    .find(|entry| entry.slot == shared::components::EquipmentVisualSlot::Head)
            })
            .cloned();
        crate::group::OnlineMember {
            entity: entity.to_bits(),
            name: self.profile.name.clone(),
            class: self.profile.class_id,
            level,
            portrait: shared::protocol::GroupPortraitAppearance {
                creature_display_id: None,
                race: visual.race,
                appearance: visual.appearance.clone(),
                head,
            },
            dungeon_difficulty,
            raid_difficulty: Default::default(),
        }
    }

    pub fn online(
        &self,
        entity: Entity,
        level: u8,
        dungeon_difficulty: u32,
        display_id: u32,
    ) -> crate::group::OnlineMember {
        crate::group::OnlineMember {
            entity: entity.to_bits(),
            name: self.profile.name.clone(),
            class: self.profile.class_id,
            level,
            portrait: shared::protocol::GroupPortraitAppearance {
                creature_display_id: Some(display_id),
                ..Default::default()
            },
            dungeon_difficulty,
            raid_difficulty: Default::default(),
        }
    }
}

#[cfg(test)]
pub(crate) fn select_party(profiles: &[Profile], role: GroupRole) -> Result<Vec<Profile>, String> {
    let mut party = Vec::with_capacity(4);
    for (required, count) in [
        (GroupRole::Tank, 1),
        (GroupRole::Healer, 1),
        (GroupRole::Dps, 3),
    ] {
        let needed = count - usize::from(role == required);
        let available: Vec<_> = profiles
            .iter()
            .filter(|profile| profile.role == required)
            .take(needed)
            .cloned()
            .collect();
        if available.len() != needed {
            return Err(format!("Follower roster lacks {required:?} members"));
        }
        party.extend(available);
    }
    Ok(party)
}

/// Approved content bridge: follower copies use the same map's Normal population.
/// Instance identity, creature difficulty, spell execution and loot remain difficulty205.
/// Retire when local follower populations and entrances are complete.
pub(crate) fn population_difficulty(difficulty: u32) -> u32 {
    if difficulty == catalog::FOLLOWER_DIFFICULTY {
        crate::maps::store::DIFFICULTY_NORMAL
    } else {
        difficulty
    }
}

pub(crate) fn retire_owner(world: &mut World, owner: Entity) {
    party::retire_owner(world, owner);
}

pub fn update_ai(world: &mut World) {
    ai::update(world);
}

pub struct FollowerPlugin;
impl Plugin for FollowerPlugin {
    fn build(&self, app: &mut App) {
        crate::maps::units::install(app);
        app.init_resource::<party::DisconnectedRuns>()
            .init_resource::<FollowerCatalog>()
            .add_systems(
                Startup,
                (
                    catalog::load_startup,
                    gear::load_startup,
                    stockade_diagnostic::load_request,
                ),
            )
            .add_systems(
                Update,
                (stockade_diagnostic::advance, party::advance_runs).chain(),
            )
            .add_systems(
                FixedUpdate,
                (release::update, update_ai)
                    .chain()
                    .after(crate::creature_aggro::update_victim_system)
                    .before(crate::creature_ai::chase_system),
            );
    }
}

#[cfg(test)]
mod ara_kara_ixin_tests;
#[cfg(test)]
mod ara_kara_ledge_tests;
#[cfg(test)]
mod catalog_tests;
#[cfg(test)]
mod cc_tests;
#[cfg(test)]
mod combat_position_native_tests;
#[cfg(test)]
mod combat_refs_tests;
#[cfg(test)]
mod control_cast_stop_tests;
#[cfg(test)]
mod hazard_reaction_tests;
#[cfg(test)]
mod hazard_tests;
#[cfg(test)]
mod pull_set_native_tests;
#[cfg(test)]
mod regroup_combat_tests;
#[cfg(test)]
pub(crate) fn fixture_hazard_escape(world: &mut World, unit: Entity) -> bool {
    hazards::escape(world, unit)
}
#[cfg(test)]
mod flight_follow_tests;
#[cfg(test)]
mod kit_abundance_tests;
#[cfg(test)]
mod kit_clearcasting_tests;
#[cfg(test)]
mod kit_dot_ownership_tests;
#[cfg(test)]
mod kit_germination_tests;
#[cfg(test)]
mod kit_lifebloom_refresh_tests;
#[cfg(test)]
mod kit_soul_of_forest_tests;
#[cfg(test)]
mod kit_source_tests;
#[cfg(test)]
mod kit_swiftmend_consumption_tests;
#[cfg(test)]
mod kit_swiftmend_tests;
#[cfg(test)]
pub(crate) mod kit_tests;
#[cfg(test)]
mod kit_tranquil_mind_tests;
#[cfg(test)]
mod kit_utility_tests;
#[cfg(test)]
mod kit_wild_growth_tests;
#[cfg(any(test, feature = "test-support"))]
pub mod native_fixture;
#[cfg(test)]
mod native_interrupt_tests;
#[cfg(test)]
mod navigation_tests;
#[cfg(test)]
mod passive_stats_tests;
#[cfg(test)]
mod pet_coverage_tests;
#[cfg(test)]
mod pet_tests;
#[cfg(test)]
pub(crate) mod profile_behavior_tests;
#[cfg(test)]
mod pyroclasm_tests;
#[cfg(test)]
mod rotation_imp_tests;
#[cfg(test)]
mod rotation_measurements;
#[cfg(test)]
mod rotation_proc_tests;
#[cfg(test)]
mod rotation_refresh_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod utility_tests;
#[cfg(test)]
mod wire_tests;
