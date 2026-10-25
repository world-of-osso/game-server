//! Online guild zone authority: existing terrain area -> AreaTable zone -> real label.
use super::{OnlineMemberState, class_name_from_id};
use crate::area_table::AreaTable;
use crate::ground::GroundMap;
use crate::maps::InMap;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use shared::components::{GuildMembership, Level, Player, Position};
use std::collections::BTreeMap;

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
}

impl OnlinePlayers<'_, '_> {
    pub(super) fn member_map(&mut self, guild_id: u32) -> BTreeMap<String, OnlineMemberState> {
        let Self {
            players,
            ground,
            areas,
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
                        online_zone: read_member_zone(ground, areas, position, map),
                    },
                ))
            })
            .collect()
    }
}

fn read_member_zone(
    ground: &mut GroundMap,
    areas: &AreaTable,
    position: Option<&Position>,
    map: Option<&InMap>,
) -> Option<String> {
    let position = position?;
    let map = map?;
    let feet = Vec3::new(position.x, position.y, position.z);
    let area = ground.area_at(map.map_id, feet)?;
    areas.zone_name(area, map.map_id).map(str::to_owned)
}
