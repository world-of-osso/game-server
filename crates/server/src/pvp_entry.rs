use bevy::prelude::*;
use shared::battleground::{BgTeam, template_by_id};

use super::{PlayerPvpState, PvpServerState};
use crate::maps::{InMap, InstanceManager, MapStore, TeleportOptions, WorldLocation};

// Local195843/195838 effect0 MOD_FACTION243 EffectMiscValue_0.
const MERCENARY_ALLIANCE_REACTION: u32 = 1;
const MERCENARY_HORDE_REACTION: u32 = 2;

pub(super) fn enter_battleground(
    world: &mut World,
    state: &mut PvpServerState,
    player: Entity,
    character_id: u64,
    match_id: u64,
) -> Result<(), String> {
    let arena = state
        .battleground_matches
        .get(&match_id)
        .and_then(|matched| matched.arena.as_ref());
    let (mut destination, transport_spawn) =
        entry_location(world, player, arena.map(|arena| arena.source.starts))?;
    let team = world
        .get::<PlayerPvpState>(player)
        .and_then(|pvp| pvp.battleground.as_ref())
        .expect("validated invitation")
        .team;
    // TC Battleground::AddPlayer casts mercenary reactions only outside arenas.
    let reaction = match arena {
        Some(_) => None,
        None => read_team_reaction(world, player)?,
    };
    let sources = world
        .get_resource::<crate::game_objects::GameObjectSpawns>()
        .ok_or("gameobject catalog unavailable")?;
    let required = if arena.is_some() {
        Vec::new()
    } else {
        authored_object_entries(destination.map_id)?
    };
    for entry in required {
        if !sources
            .0
            .iter()
            .any(|spawn| spawn.map_id == destination.map_id && spawn.info.entry == entry)
        {
            return Err(format!(
                "authored battleground object {entry} is absent; rebuild Retail source content"
            ));
        }
    }
    let current = world
        .get::<InMap>(player)
        .ok_or("current map transfer is still pending")?;
    let origin = crate::maps::transfer::current_location(world, player, current.map_id);
    let dead_entrant = world
        .get::<shared::components::Health>(player)
        .is_some_and(|health| health.current <= 0.0);
    let map = create_match_map(world, state, match_id)?;
    if let Some(spawn_id) = transport_spawn {
        destination = crate::transport::prepare_entry(world, player, map, destination, spawn_id)?;
    }
    // TC BattleGroundHandler.cpp:380-384 resurrects before battlefield port.
    if dead_entrant {
        crate::death::resurrect_player_fully(world, player);
    }
    if !crate::maps::teleport_to(world, player, destination, TeleportOptions::default()) {
        crate::transport::cancel_entry(world, player);
        return Err("battleground map entry was refused".into());
    }
    // ConditionMgr uses ArenaFaction for both BG and arena, unlike reactions.
    world
        .entity_mut(player)
        .insert(shared::components::CurrentPvpFaction(match team {
            BgTeam::A => 2,
            BgTeam::B => 1,
        }));
    apply_team_reaction(world, player, reaction);
    let entry = super::reconnect::Entry::new(
        match_id,
        state.battleground_matches[&match_id]
            .map
            .expect("created match copy"),
        origin,
        world
            .get::<PlayerPvpState>(player)
            .expect("participant PvP state")
            .battleground_original_faction,
    );
    super::reconnect::save(world, character_id, &entry);
    state
        .battleground_matches
        .get_mut(&match_id)
        .expect("validated invitation")
        .origins
        .insert(character_id, origin);
    Ok(())
}

pub(super) fn restore_team_reaction(world: &mut World, player: Entity) {
    let arena = world
        .get::<PlayerPvpState>(player)
        .and_then(|state| state.battleground.as_ref())
        .is_some_and(|snapshot| snapshot.arena.is_some());
    if !arena {
        let reaction =
            read_team_reaction(world, player).expect("restored battleground participant");
        apply_team_reaction(world, player, reaction);
    }
}

/// TC Battleground::AddPlayer's mercenary reactions: local195838/195843
/// effect0 MOD_FACTION243 changes Horde/Alliance reactions to template2/1.
/// Preserve matching racial reactions and restore the original value on Leave.
fn read_team_reaction(world: &World, player: Entity) -> Result<Option<(u32, u32)>, String> {
    let race = world
        .get::<shared::components::Player>(player)
        .ok_or("battleground player race missing")?
        .race;
    let original = world
        .get::<shared::components::UnitFactionTemplate>(player)
        .ok_or("battleground player faction missing")?
        .0;
    let team = world
        .get::<PlayerPvpState>(player)
        .and_then(|state| state.battleground.as_ref())
        .ok_or("battleground invitation missing")?
        .team;
    let assigned = match team {
        BgTeam::A => MERCENARY_ALLIANCE_REACTION,
        BgTeam::B => MERCENARY_HORDE_REACTION,
    };
    Ok(
        (u32::from(crate::faction::faction_for_race(race)) != assigned)
            .then_some((original, assigned)),
    )
}

fn apply_team_reaction(world: &mut World, player: Entity, reaction: Option<(u32, u32)>) {
    if let Some((original, assigned)) = reaction {
        world
            .get_mut::<shared::components::UnitFactionTemplate>(player)
            .expect("validated player faction")
            .0 = assigned;
        world
            .get_mut::<PlayerPvpState>(player)
            .expect("validated participant")
            .battleground_original_faction = Some(original);
    }
}

fn entry_location(
    world: &World,
    player: Entity,
    arena_starts: Option<[u32; 2]>,
) -> Result<(WorldLocation, Option<u32>), String> {
    let invitation = world
        .get::<PlayerPvpState>(player)
        .and_then(|state| state.battleground.as_ref())
        .ok_or("player has no battleground invitation")?;
    let starts = arena_starts
        .or_else(|| shared::battleground::flag_rules(invitation.map_id).map(|rules| rules.starts))
        .or_else(|| shared::battleground::node_rules(invitation.map_id).map(|rules| rules.starts))
        .or_else(|| shared::battleground::eye_rules(invitation.map_id).map(|rules| rules.starts))
        .or_else(|| (invitation.map_id == 998).then_some(shared::battleground::KOTMOGU_STARTS))
        .or_else(|| (invitation.map_id == 727).then_some(shared::battleground::SILVERSHARD_STARTS))
        .or_else(|| (invitation.map_id == 1803).then_some(shared::battleground::SEETHING_STARTS))
        .or_else(|| (invitation.map_id == 2656).then_some(shared::battleground::DEEPHAUL_STARTS))
        .or_else(|| (invitation.map_id == 30).then_some([611, 610]))
        // TDB battleground_template 30 AllianceStartLoc/HordeStartLoc.
        .or_else(|| (invitation.map_id == 628).then_some([1299, 1245]))
        .ok_or("this battleground has no implemented entry lifecycle")?;
    let location_id = starts[match invitation.team {
        BgTeam::A => 0,
        BgTeam::B => 1,
    }];
    let store = world
        .get_resource::<MapStore>()
        .ok_or("map catalog is unavailable")?;
    let transport = store.safe_loc_transports.get(&location_id).copied();
    if let Some(spawn_id) = transport {
        crate::transport::validate_entry(world, spawn_id, invitation.map_id)?;
    }
    let destination = store
        .safe_locs
        .get(&location_id)
        .copied()
        .ok_or_else(|| format!("battleground start location {location_id} is absent"))?;
    if destination.map_id != invitation.map_id {
        return Err("battleground start location disagrees with its map".into());
    }
    Ok((destination, transport))
}

fn authored_object_entries(map_id: u32) -> Result<Vec<u32>, String> {
    let entries = shared::battleground::objective_object_entries(map_id);
    if !shared::battleground::has_implemented_objectives(map_id) {
        return Err("this battleground has no implemented objectives".into());
    }
    // Seething objects are dynamic summons; its source catalog validates templates
    // and twelve fissure/ADT areas before reserving a copy, not static GO placements.
    Ok(if map_id == 1803 { Vec::new() } else { entries })
}

fn create_match_map(
    world: &mut World,
    state: &mut PvpServerState,
    match_id: u64,
) -> Result<InMap, String> {
    let invitation = state
        .battleground_matches
        .get(&match_id)
        .ok_or("battleground invitation expired")?;
    if let Some(map) = invitation.map {
        return Ok(map);
    }
    let template = match &invitation.arena {
        Some(arena) => arena.template(),
        None => template_by_id(invitation.instance.bg_id).ok_or("battleground template missing")?,
    };
    let roster: Vec<_> = invitation
        .instance
        .team_a_players
        .iter()
        .chain(&invitation.instance.team_b_players)
        .copied()
        .collect();
    let carts = if template.map_id == 727 {
        Some(super::pvp_silvershard::CartRuntime::from_source(
            world
                .get_resource::<crate::world_data::WorldData>()
                .ok_or("creature catalog unavailable")?,
        )?)
    } else {
        None
    };
    let seething = if template.map_id == 1803 {
        Some(super::pvp_seething::SeethingRuntime::from_source(world)?)
    } else {
        None
    };
    let alterac = if matches!(template.map_id, 30 | 628) {
        Some(super::pvp_alterac::BannerRuntime::from_source(
            world,
            template.map_id,
        )?)
    } else {
        None
    };
    let deephaul = if template.map_id == 2656 {
        Some(super::pvp_deephaul::DeephaulRuntime::from_source(world)?)
    } else {
        None
    };
    let capacity = u32::from(template.team_size) * 2;
    let arena = invitation.arena.is_some();
    let now = crate::maps::difficulty::now(world);
    let map = world
        .resource_scope(|world, mut instances: Mut<InstanceManager>| {
            instances.create_battleground_map(
                world.resource::<MapStore>(),
                template.map_id,
                &roster,
                capacity,
                now,
            )
        })
        .map_err(|reason| format!("battleground map reservation refused: {reason:?}"))?;
    state
        .battleground_matches
        .get_mut(&match_id)
        .expect("validated match")
        .map = Some(map);
    state
        .battleground_matches
        .get_mut(&match_id)
        .expect("validated match")
        .seething = seething;
    crate::maps::transfer::spawn_map_content(world, map);
    if template.map_id == 1803 {
        crate::transport::spawn_ships(world, map);
    }
    if arena {
        super::arena_objects::prepare_map(world, map);
        super::arena_sight::prepare_map(world, map);
    }
    if let Some(alterac) = alterac {
        let states = alterac.battle.world_states(
            &state
                .battleground_matches
                .get(&match_id)
                .expect("validated match")
                .instance,
        );
        alterac.synchronize(world, map, &states);
        state
            .battleground_matches
            .get_mut(&match_id)
            .expect("validated match")
            .alterac = Some(alterac);
    }
    if let Some(mut deephaul) = deephaul {
        deephaul.synchronize_carts(world, map);
        deephaul.synchronize_crystal(world, map);
        state
            .battleground_matches
            .get_mut(&match_id)
            .expect("validated match")
            .deephaul = Some(deephaul);
    }
    if let Some(mut carts) = carts {
        carts.synchronize(world, map);
        state
            .battleground_matches
            .get_mut(&match_id)
            .expect("validated match")
            .silvershard = Some(carts);
    }
    Ok(map)
}
