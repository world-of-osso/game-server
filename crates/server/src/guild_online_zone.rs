//! Online guild zone authority: existing terrain area -> AreaTable zone -> real label.
use super::{OnlineMemberState, class_name_from_id};
use crate::area_table::AreaTable;
use crate::ground::GroundMap;
use crate::maps::InMap;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use rusqlite::{Connection, OpenFlags};
use shared::components::{GuildMembership, Level, Player, Position};
use std::collections::{BTreeMap, HashMap};

#[derive(Resource)]
pub(super) struct GuildZoneNames(HashMap<u32, String>);

pub(super) fn load_names(conn: &Connection) -> rusqlite::Result<GuildZoneNames> {
    let mut query = conn.prepare("SELECT ID, AreaName_lang FROM area_table")?;
    let rows = query.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
    Ok(GuildZoneNames(rows.collect::<rusqlite::Result<_>>()?))
}

pub(super) fn load_startup(mut commands: Commands) {
    let conn = Connection::open_with_flags("data/world.db", OpenFlags::SQLITE_OPEN_READ_ONLY)
        .expect("Open world.db for authoritative guild zone names");
    let names = load_names(&conn).expect("Load AreaTable guild zone labels");
    commands.insert_resource(names);
}

type MemberQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static Player,
        &'static Level,
        Option<&'static GuildMembership>,
        Option<&'static Position>,
        Option<&'static InMap>,
    ),
>;

#[derive(SystemParam)]
pub(super) struct OnlinePlayers<'w, 's> {
    players: MemberQuery<'w, 's>,
    ground: ResMut<'w, GroundMap>,
    areas: Res<'w, AreaTable>,
    names: Res<'w, GuildZoneNames>,
}

impl OnlinePlayers<'_, '_> {
    pub(super) fn member_map(&mut self, guild_id: u32) -> BTreeMap<String, OnlineMemberState> {
        let Self {
            players,
            ground,
            areas,
            names,
        } = self;
        players
            .iter()
            .filter_map(|(player, level, membership, position, map)| {
                let membership = membership?;
                if membership.guild_id != guild_id {
                    return None;
                }
                Some((
                    player.name.clone(),
                    OnlineMemberState {
                        level: level.0 as u16,
                        class_name: class_name_from_id(player.class),
                        online_zone: read_member_zone(ground, areas, names, position, map),
                    },
                ))
            })
            .collect()
    }
}

fn read_member_zone(
    ground: &mut GroundMap,
    areas: &AreaTable,
    names: &GuildZoneNames,
    position: Option<&Position>,
    map: Option<&InMap>,
) -> Option<String> {
    let position = position?;
    let map = map?;
    let feet = Vec3::new(position.x, position.y, position.z);
    let area = ground.area_at(map.map_id, feet)?;
    let zone = areas.zone_id(area, map.map_id)?;
    names.0.get(&zone).filter(|name| !name.is_empty()).cloned()
}
