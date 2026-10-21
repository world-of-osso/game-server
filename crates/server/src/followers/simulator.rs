//! Explicit transport-free simulator seam; real initializer and kit controller.
use super::*;
use bevy::ecs::system::RunSystemOnce;
use shared::components::{Level, Player, Position, UnitFactionTemplate};

#[derive(Component)]
struct LeaderProfile(Profile);

pub(crate) struct Party {
    pub owner: Entity,
    pub tank: Entity,
    pub members: Vec<Entity>,
}

const DEFAULT_PARTY: [u32; 5] = [209057, 209072, 209053, 209059, 209065];

struct PartyConfiguration {
    level: u8,
    entries: [u32; 5],
}

impl PartyConfiguration {
    fn read() -> Result<Self, String> {
        let level = read_override("DUNGEON_SIMULATOR_LEVEL")?;
        let healer = read_override("DUNGEON_SIMULATOR_HEALER_ENTRY")?;
        let party = read_override("DUNGEON_SIMULATOR_PARTY_ENTRIES")?;
        Self::parse(level.as_deref(), healer.as_deref(), party.as_deref())
    }

    fn parse(
        level: Option<&str>,
        healer: Option<&str>,
        party: Option<&str>,
    ) -> Result<Self, String> {
        let level = level
            .unwrap_or("32")
            .parse::<u8>()
            .map_err(|error| format!("Invalid DUNGEON_SIMULATOR_LEVEL: {error}"))?;
        if level == 0 || u32::from(level) > crate::spell_info::MAX_LEVEL {
            return Err(format!(
                "Simulator level must be1..={}",
                crate::spell_info::MAX_LEVEL
            ));
        }
        let mut entries = match party {
            Some(party) => parse_party_entries(party)?,
            None => DEFAULT_PARTY,
        };
        if let Some(healer) = healer {
            entries[1] = healer
                .parse()
                .map_err(|error| format!("Invalid DUNGEON_SIMULATOR_HEALER_ENTRY: {error}"))?;
        }
        Ok(Self { level, entries })
    }
}

fn read_override(name: &str) -> Result<Option<String>, String> {
    match std::env::var(name) {
        Ok(value) => Ok(Some(value)),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(error) => Err(format!("Invalid {name}: {error}")),
    }
}

fn parse_party_entries(value: &str) -> Result<[u32; 5], String> {
    let entries: Vec<_> = value
        .split(',')
        .map(|entry| entry.trim().parse::<u32>())
        .collect::<Result<_, _>>()
        .map_err(|error| format!("Invalid DUNGEON_SIMULATOR_PARTY_ENTRIES: {error}"))?;
    entries.try_into().map_err(|_| {
        "DUNGEON_SIMULATOR_PARTY_ENTRIES requires tank,healer,dps,dps,leader-dps".into()
    })
}

fn read_party_profiles(world: &World, entries: [u32; 5]) -> Result<Vec<Profile>, String> {
    use shared::group::GroupRole::{Dps, Healer, Tank};
    let catalog = world.resource::<FollowerCatalog>();
    entries
        .into_iter()
        .zip([Tank, Healer, Dps, Dps, Dps])
        .enumerate()
        .map(|(slot, (entry, role))| {
            let profile = catalog
                .profiles
                .iter()
                .find(|profile| profile.entry == entry)
                .ok_or_else(|| format!("Native simulator follower profile{entry} absent"))?;
            if profile.role != role {
                return Err(format!(
                    "Simulator party slot{} requires{role:?}, profile{entry} is{:?}",
                    slot + 1,
                    profile.role
                ));
            }
            Ok(profile.clone())
        })
        .collect()
}

#[test]
fn simulator_configuration_rejects_invalid_levels_and_incomplete_rosters() {
    for level in ["0", "91", "not-a-level"] {
        assert!(PartyConfiguration::parse(Some(level), None, None).is_err());
    }
    for party in ["209057,209072", "209057,209072,no,209059,209065"] {
        assert!(PartyConfiguration::parse(None, None, Some(party)).is_err());
    }
}

pub(crate) fn spawn_party(
    world: &mut World,
    copy: crate::maps::InMap,
    at: Position,
) -> Result<Party, String> {
    let configuration = PartyConfiguration::read()?;
    let profiles = read_party_profiles(world, configuration.entries)?;
    let leader = profiles[4].clone();
    let owner = world
        .spawn((
            Player {
                name: leader.name.clone(),
                race: 1,
                class: leader.class_id,
                appearance: default(),
            },
            Level(configuration.level),
            shared::components::UnitLevel(configuration.level),
            UnitFactionTemplate(1),
            at,
            copy,
            crate::persistence::ClientId(1),
            crate::maps::DungeonDifficulty(1),
            crate::networking::CombatTarget::default(),
            crate::networking::Moving(false),
            shared::components::UnitPowers::default(),
        ))
        .id();
    // Run native Player initialization before the profile installs PowerPools;
    // otherwise its Without<PowerPools> query never installs health regeneration.
    world
        .run_system_once(crate::spell_cast::powers::init_player_powers)
        .map_err(|error| {
            format!("initialize simulator leader through native player powers: {error}")
        })?;
    spawning::initialize(world, owner, owner, leader.clone())?;
    world
        .entity_mut(owner)
        .remove::<(Follower, shared::components::UnitSummonedBy)>()
        .insert(LeaderProfile(leader));
    let members = spawning::spawn_members(world, owner, &profiles[..4])?;
    party::join_members(world, owner, shared::group::GroupRole::Dps, &members)?;
    world
        .entity_mut(owner)
        .insert(stockade_diagnostic::DiagnosticParty {
            copy,
            members: members.clone(),
            player_role: shared::group::GroupRole::Dps,
        });
    let tank = members
        .iter()
        .copied()
        .find(|member| {
            world.get::<Follower>(*member).unwrap().profile.role == shared::group::GroupRole::Tank
        })
        .ok_or("simulator party lacks tank")?;
    Ok(Party {
        owner,
        tank,
        members,
    })
}

pub(crate) fn update_leader(world: &mut World) {
    let leaders: Vec<_> = world
        .query::<(Entity, &LeaderProfile)>()
        .iter(world)
        .map(|(entity, profile)| (entity, profile.0.clone()))
        .collect();
    for (owner, profile) in leaders {
        if crate::spell_cast::targets::is_alive(world, owner) {
            ai::update_follower(world, owner, &Follower { owner, profile });
        }
    }
}

pub(crate) fn has_uncleared_fight(world: &World, owner: Entity) -> bool {
    pull_history::has_uncleared_fight(world, owner)
}

pub(crate) fn pull(
    world: &mut World,
    tank: Entity,
    guids: &std::collections::BTreeSet<u32>,
) -> Result<(bool, String), String> {
    pull_history::sample_all(world);
    let decision = pull_admission::read_pull_budget(world, tank, guids, 1)?;
    let allowed = matches!(decision, pull_budget::Decision::Allow { .. });
    Ok((allowed, format!("{decision:?}")))
}

pub(crate) fn inspect_tank_movement(
    world: &mut World,
    tank: Entity,
) -> Result<Option<bool>, String> {
    let target = world
        .get::<crate::networking::CombatTarget>(tank)
        .and_then(|target| target.0)
        .ok_or("tank has no selected victim")?;
    let follower = world.get::<Follower>(tank).unwrap().clone();
    tank_planner::read_movement_admission(world, tank, target, &follower, None)
}

pub(crate) fn regroup(world: &mut World, unit: Entity, owner: Entity) {
    ai::regroup(world, unit, owner);
}

pub(crate) fn opening_spell(world: &World, tank: Entity) -> (u32, f32) {
    let spell_id = world.get::<Follower>(tank).unwrap().profile.attack_spell;
    let spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(spell_id)
        .unwrap();
    (
        spell_id,
        crate::spell_cast::targets::spell_range(spell, true).1,
    )
}

pub(crate) fn cast_opening(
    world: &mut World,
    tank: Entity,
    target: Entity,
) -> Result<(), crate::spell_cast::CastFailure> {
    let (spell, _) = opening_spell(world, tank);
    crate::spell_cast::creature_cast(world, tank, spell, Some(target), false)
}
