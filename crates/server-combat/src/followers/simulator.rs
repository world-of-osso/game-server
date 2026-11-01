//! Explicit transport-free simulator seam: class actors driven by their spec APL.
use super::ai::LeaderProfile;
pub use super::catalog::load_catalog as load_parity_catalog;
use super::class_profiles::ClassProfile;
use super::class_spawn::GearSource;
use super::*;
use bevy::ecs::system::RunSystemOnce;
use shared::components::{Level, Player, Position, UnitFactionTemplate};

pub struct Party {
    pub owner: Entity,
    pub tank: Entity,
    pub members: Vec<Entity>,
}

/// Tank, healer, two DPS and the leader DPS: Protection Paladin, Restoration
/// Druid, Elemental Shaman, Fire Mage and Beast Mastery Hunter.
pub const DEFAULT_PARTY: [u32; 5] = [66, 105, 262, 63, 253];

struct PartyConfiguration {
    level: u8,
    specs: [u32; 5],
}

impl PartyConfiguration {
    fn read() -> Result<Self, String> {
        let level = read_override("DUNGEON_SIMULATOR_LEVEL")?;
        let healer = read_override("DUNGEON_SIMULATOR_HEALER_SPEC")?;
        let party = read_override("DUNGEON_SIMULATOR_PARTY_SPECS")?;
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
        let mut specs = match party {
            Some(party) => parse_party_specs(party)?,
            None => DEFAULT_PARTY,
        };
        if let Some(healer) = healer {
            specs[1] = healer
                .parse()
                .map_err(|error| format!("Invalid DUNGEON_SIMULATOR_HEALER_SPEC: {error}"))?;
        }
        Ok(Self { level, specs })
    }
}

fn read_override(name: &str) -> Result<Option<String>, String> {
    match std::env::var(name) {
        Ok(value) => Ok(Some(value)),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(error) => Err(format!("Invalid {name}: {error}")),
    }
}

fn parse_party_specs(value: &str) -> Result<[u32; 5], String> {
    let specs: Vec<_> = value
        .split(',')
        .map(|spec| spec.trim().parse::<u32>())
        .collect::<Result<_, _>>()
        .map_err(|error| format!("Invalid DUNGEON_SIMULATOR_PARTY_SPECS: {error}"))?;
    specs
        .try_into()
        .map_err(|_| "DUNGEON_SIMULATOR_PARTY_SPECS requires tank,healer,dps,dps,leader-dps".into())
}

/// The class profiles for `specs`, each checked against its party slot's role.
fn read_party_profiles(specs: [u32; 5]) -> Result<Vec<ClassProfile>, String> {
    select_party_profiles(specs, &class_profiles::read_profiles()?)
}

fn select_party_profiles(
    specs: [u32; 5],
    profiles: &[ClassProfile],
) -> Result<Vec<ClassProfile>, String> {
    use shared::group::GroupRole::{Dps, Healer, Tank};
    specs
        .into_iter()
        .zip([Tank, Healer, Dps, Dps, Dps])
        .enumerate()
        .map(|(slot, (spec, role))| {
            let profile = profiles
                .iter()
                .find(|profile| profile.spec_id == spec)
                .ok_or_else(|| format!("Simulator class follower spec{spec} absent"))?;
            let actual = profile.group_role()?;
            if actual != role {
                return Err(format!(
                    "Simulator party slot{} requires{role:?}, spec{spec} is{actual:?}",
                    slot + 1
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
    for party in ["66,105", "66,105,no,63,253"] {
        assert!(PartyConfiguration::parse(None, None, Some(party)).is_err());
    }
}

pub fn spawn_party(
    world: &mut World,
    copy: crate::maps::InMap,
    at: Position,
) -> Result<Party, String> {
    spawn_configured_party(world, copy, at, PartyConfiguration::read()?)
}

/// Move a parity follower and its diagnostic admission to a test copy.
pub fn relocate_parity_follower(world: &mut World, unit: Entity, copy: crate::maps::InMap) {
    let owner = world.get::<Follower>(unit).expect("parity follower").owner;
    world
        .get_mut::<stockade_diagnostic::DiagnosticParty>(owner)
        .expect("parity diagnostic party")
        .copy = copy;
    world.entity_mut(owner).insert(copy);
    world.entity_mut(unit).insert(copy);
}

/// Explicit benchmark actor selection, without process-global environment mutation.
pub fn spawn_party_with_healer(
    world: &mut World,
    copy: crate::maps::InMap,
    at: Position,
    level: u8,
    spec: u32,
) -> Result<Party, String> {
    let mut specs = DEFAULT_PARTY;
    specs[1] = spec;
    spawn_profiled_party(
        world,
        copy,
        at,
        level,
        read_party_profiles(specs)?,
        GearSource::Journal,
    )
}

/// Attach four class companions to an existing parity healer player.
pub fn spawn_party_for_healer_player(world: &mut World, healer: Entity) -> Result<Party, String> {
    let copy = *world
        .get::<crate::maps::InMap>(healer)
        .ok_or("healer copy absent")?;
    let profiles = read_party_profiles(DEFAULT_PARTY)?;
    let companions: Vec<_> = [0, 2, 3, 4]
        .into_iter()
        .map(|index| profiles[index].clone())
        .collect();
    let members = spawn_companions(world, healer, &companions, GearSource::Journal)?;
    party::join_members(world, healer, shared::group::GroupRole::Healer, &members)?;
    world
        .entity_mut(healer)
        .insert(stockade_diagnostic::DiagnosticParty {
            copy,
            members: members.clone(),
            player_role: shared::group::GroupRole::Healer,
        });
    Ok(Party {
        owner: healer,
        tank: members[0],
        members,
    })
}

/// Party sexes are cosmetic; one fixed draw keeps simulator runs reproducible.
const SIMULATOR_SEX_SEED: u64 = 0;

fn spawn_companions(
    world: &mut World,
    owner: Entity,
    profiles: &[ClassProfile],
    gear: GearSource,
) -> Result<Vec<Entity>, String> {
    class_spawn::spawn_members(world, owner, profiles, SIMULATOR_SEX_SEED, gear)
}

/// Assign the headless reference player its distinct group identity.
pub fn assign_reference_player_id(world: &mut World, player: Entity, id: u64) {
    world
        .entity_mut(player)
        .insert(crate::persistence::ClientId(id));
}

fn spawn_configured_party(
    world: &mut World,
    copy: crate::maps::InMap,
    at: Position,
    configuration: PartyConfiguration,
) -> Result<Party, String> {
    let profiles = read_party_profiles(configuration.specs)?;
    spawn_profiled_party(
        world,
        copy,
        at,
        configuration.level,
        profiles,
        GearSource::Journal,
    )
}

/// Spawn an explicitly selected class party using the qualified key-band builds.
pub fn spawn_keystone_party(
    world: &mut World,
    copy: crate::maps::InMap,
    at: Position,
    level: u8,
    specs: [u32; 5],
    key_level: u32,
) -> Result<Party, String> {
    let classes = world
        .get_resource::<crate::spell_info::ClassSpells>()
        .ok_or("follower classes unavailable")?;
    let spells = world
        .get_resource::<crate::spell_info::SpellStore>()
        .ok_or("follower spells unavailable")?;
    let profiles = super::gear::wcl::class_profiles(classes, key_level)?;
    let qualified = class_profiles::validate_profiles(&profiles, classes, spells, level);
    let selected = select_party_profiles(specs, &qualified)?;
    spawn_profiled_party(
        world,
        copy,
        at,
        level,
        selected,
        GearSource::KeystoneBand(key_level),
    )
}

/// Party units that wear gear with the profile that selects it: follower
/// members plus a synthetic leader. Player owners keep their own equipment.
pub fn geared_profiles(world: &World, party: &Party) -> Vec<(Entity, Profile)> {
    let members = party.members.iter().filter_map(|&unit| {
        world
            .get::<Follower>(unit)
            .map(|follower| (unit, follower.profile.clone()))
    });
    let leader = world
        .get::<LeaderProfile>(party.owner)
        .map(|leader| (party.owner, leader.0.clone()));
    members.chain(leader).collect()
}

fn spawn_profiled_party(
    world: &mut World,
    copy: crate::maps::InMap,
    at: Position,
    level: u8,
    profiles: Vec<ClassProfile>,
    gear: GearSource,
) -> Result<Party, String> {
    let difficulty = copy_difficulty(world, copy)?;
    let (owner, leader) =
        spawn_class_owner(world, &profiles[4], level, copy, at, difficulty, gear)?;
    world.entity_mut(owner).insert(LeaderProfile(leader));
    let members = spawn_companions(world, owner, &profiles[..4], gear)?;
    party::join_members(world, owner, shared::group::GroupRole::Dps, &members)?;
    world
        .entity_mut(owner)
        .insert(stockade_diagnostic::DiagnosticParty {
            copy,
            members: members.clone(),
            player_role: shared::group::GroupRole::Dps,
        });
    Ok(Party {
        owner,
        tank: members[0],
        members,
    })
}

fn copy_difficulty(
    world: &World,
    copy: crate::maps::InMap,
) -> Result<crate::maps::DungeonDifficulty, String> {
    let difficulty = world
        .resource::<crate::maps::InstanceManager>()
        .copy_difficulty(copy)
        .ok_or("simulator copy lacks native difficulty")?;
    Ok(crate::maps::DungeonDifficulty(difficulty.difficulty_id))
}

pub enum ParityActor {
    Follower,
    Player { race: u8 },
}

/// One real attacker, without unrelated party damage or buff sources.
pub fn spawn_parity_attacker(
    world: &mut World,
    spec_id: u32,
    level: u8,
    copy: crate::maps::InMap,
    actor: ParityActor,
) -> Result<Entity, String> {
    let difficulty = copy_difficulty(world, copy)?;
    if let ParityActor::Player { race } = actor {
        return spawn_parity_player(world, spec_id, level, race, copy, difficulty);
    }
    let source = super::class_profiles::read_profiles()?
        .into_iter()
        .find(|source| source.spec_id == spec_id)
        .ok_or_else(|| format!("class follower spec{spec_id} absent"))?;
    // An inert native Player owns the attacker's party and runs no rotation.
    let owner = spawn_parity_player(world, spec_id, level, source.race, copy, difficulty)?;
    let unit = super::class_spawn::spawn_class(
        world,
        owner,
        &source,
        0,
        0,
        super::class_spawn::GearSource::Journal,
    )?;
    // A living native owner can maintain /startattack without landing out-of-range swings.
    world.entity_mut(owner).insert(Position {
        x: 20.0,
        y: 0.0,
        z: 0.0,
    });
    // advance_runs removes followers without a real or admitted diagnostic run.
    world
        .entity_mut(owner)
        .insert(stockade_diagnostic::DiagnosticParty {
            copy,
            members: vec![unit],
            player_role: shared::group::GroupRole::Dps,
        });
    Ok(unit)
}

fn spawn_parity_player(
    world: &mut World,
    spec_id: u32,
    level: u8,
    race: u8,
    copy: crate::maps::InMap,
    difficulty: crate::maps::DungeonDifficulty,
) -> Result<Entity, String> {
    let class = world
        .resource::<crate::spell_info::ClassSpells>()
        .classes
        .iter()
        .find(|class| class.specs.iter().any(|spec| spec.spec_id == spec_id))
        .ok_or_else(|| format!("native player spec{spec_id} absent"))?
        .class_id;
    let player = world
        .spawn((
            Player {
                name: format!("Parity{spec_id}"),
                race,
                class,
                appearance: default(),
            },
            Level(level),
            shared::components::UnitLevel(level),
            crate::class_progression::ActiveSpec(spec_id),
            UnitFactionTemplate(crate::faction::faction_template_for_race(race)),
            Position {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            shared::components::Rotation {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            copy,
            crate::persistence::ClientId(1),
            difficulty,
            crate::networking::CombatTarget::default(),
            crate::networking::Moving(false),
            shared::components::PlayerStandState::default(),
            // Headless Players use native runtime storage, not a fabricated
            // authenticated character or a persisted server account.
            crate::hunter_pet::PetStable {
                character_id: None,
                stored: Default::default(),
                summoned: None,
            },
        ))
        .id();
    install_parity_player_stat_components(world, player);
    // SimC76f8118 sc_warlock_init.cpp1533–1538 initializes the class pool
    // before profile values; a newly spawned Player has not run its first frame.
    world
        .run_system_once(crate::spell_cast::powers::init_player_powers)
        .map_err(|error| {
            format!("initialize parity player through native class powers: {error}")
        })?;
    Ok(player)
}

fn install_parity_player_stat_components(world: &mut World, player: Entity) {
    // These initial full ratios are replaced by native Stats before measurement.
    // Player powers, passives and regeneration initialize through ordinary plugins.
    world.entity_mut(player).insert((
        shared::components::Health {
            current: 1.0,
            max: 1.0,
        },
        shared::components::Mana {
            current: 1.0,
            max: 1.0,
        },
        shared::components::UnitStats::default(),
        shared::components::CombatRatings::default(),
        shared::components::DerivedStats::default(),
    ));
}

/// A headless native Player that owns the party and fights as the class actor
/// `source`, returning the profile its follower brain uses.
fn spawn_class_owner(
    world: &mut World,
    source: &ClassProfile,
    level: u8,
    copy: crate::maps::InMap,
    at: Position,
    difficulty: crate::maps::DungeonDifficulty,
    gear: GearSource,
) -> Result<(Entity, Profile), String> {
    let owner = spawn_parity_player(world, source.spec_id, level, source.race, copy, difficulty)?;
    world.entity_mut(owner).insert(at);
    class_spawn::install_class(world, owner, owner, source, 0, 4, gear)?;
    // install_class makes the actor follow `owner`; the party owner follows no one.
    let follower = world
        .entity_mut(owner)
        .take::<Follower>()
        .ok_or("simulator leader lacks its class profile")?;
    world.entity_mut(owner).remove::<(
        shared::components::UnitSummonedBy,
        crate::creature_ai::FollowOwner,
    )>();
    Ok((owner, follower.profile))
}

pub fn update_leader(world: &mut World) {
    let leaders: Vec<_> = world
        .query_filtered::<Entity, With<LeaderProfile>>()
        .iter(world)
        .collect();
    for owner in leaders {
        if crate::spell_cast::targets::is_alive(world, owner) {
            let leader = ai::acting_follower(world, owner).expect("simulator leader profile");
            ai::update_follower(world, owner, &leader);
        }
    }
}

pub fn observed_cleared_sources(world: &World, owner: Entity) -> Option<Vec<u32>> {
    pull_history::snapshot(world, owner).map(|history| history.cleared_spawns.into_iter().collect())
}

pub fn has_uncleared_fight(world: &World, owner: Entity) -> bool {
    pull_history::has_uncleared_fight(world, owner)
}

pub fn pull(
    world: &mut World,
    tank: Entity,
    guids: &std::collections::BTreeSet<u32>,
) -> Result<(bool, String), String> {
    pull_history::sample_all(world);
    let decision = pull_admission::read_pull_budget(world, tank, guids, 1)?;
    let allowed = matches!(decision, pull_budget::Decision::Allow { .. });
    Ok((allowed, format!("{decision:?}")))
}

pub fn inspect_follower_decision(world: &mut World, unit: Entity) -> serde_json::Value {
    let follower = world.get::<Follower>(unit).unwrap().clone();
    let party = ai::party_units(world, follower.owner);
    let enemies = ai::engaged_enemies(world, &follower, unit, &party);
    let target = enemies
        .iter()
        .map(|(enemy, _)| *enemy)
        .find(|enemy| !crate::auras::has_breakable_by_damage_cc(world, *enemy));
    serde_json::json!({"unit":unit.to_bits(),"spec":follower.profile.spec_id,"target":target.map(Entity::to_bits),
        "enemies":enemies.iter().map(|(enemy,_)| enemy.to_bits()).collect::<Vec<_>>(),
        "self_hazard":super::hazards::target_is_hazardous(world,unit,unit),
        "approach":target.map(|target| super::hazards::approach_is_unsafe(world,unit,target))})
}

pub fn inspect_tank_movement(world: &mut World, tank: Entity) -> Result<Option<bool>, String> {
    let target = world
        .get::<crate::networking::CombatTarget>(tank)
        .and_then(|target| target.0)
        .ok_or("tank has no selected victim")?;
    let follower = world.get::<Follower>(tank).unwrap().clone();
    tank_planner::read_movement_admission(world, tank, target, &follower, None)
}

pub fn regroup(world: &mut World, unit: Entity, owner: Entity) {
    ai::regroup(world, unit, owner);
}

/// The tank opens a pull with its native taunt, resolved when it spawned.
pub fn opening_spell(world: &World, tank: Entity) -> (u32, f32) {
    let spell_id = world.get::<Follower>(tank).unwrap().profile.taunt_spell;
    let spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(spell_id)
        .unwrap();
    (
        spell_id,
        crate::spell_cast::targets::spell_range(spell, true).1,
    )
}

pub fn cast_opening(
    world: &mut World,
    tank: Entity,
    target: Entity,
) -> Result<(), crate::spell_cast::CastFailure> {
    let (spell, _) = opening_spell(world, tank);
    crate::spell_cast::creature_cast(world, tank, spell, Some(target), false)
}
