//! Native KillRewarder::_RewardKillCredit (TC a352b1fa KillRewarder.cpp177-188).
use super::*;
use crate::creature_tap::{CreatureTap, RewardPlayers, rewarded_characters};
use crate::death::PlayerDeathState;
use crate::group::GroupRegistry;
use crate::persistence::ClientId;
use shared::components::Position;

type DeadTappedCreatures<'w, 's> = Query<
    'w,
    's,
    (
        &'static Npc,
        Option<&'static shared::components::UnitLevel>,
        &'static CreatureTap,
        &'static Position,
        &'static InMap,
    ),
    (With<Npc>, Added<crate::combat::DeathState>),
>;

type PlayerLives<'w, 's> =
    Query<'w, 's, (Entity, &'static ClientId, Option<&'static PlayerDeathState>), With<Player>>;

type OnlinePlayers<'w, 's> =
    Query<'w, 's, &'static PlayerLink, (With<Authenticated>, With<InWorld>)>;

/// One actual NPC death credits its eligible tap recipients, not just the last attacker.
pub(super) fn publish_rewarded_creature_kills(
    killed: DeadTappedCreatures,
    positions: RewardPlayers,
    lives: PlayerLives,
    groups: Option<Res<GroupRegistry>>,
    data: Option<Res<crate::world_data::WorldData>>,
    connections: OnlinePlayers,
    mut credits: MessageWriter<CriteriaCredit>,
) {
    let online: BTreeSet<_> = connections.iter().map(|link| link.0).collect();
    for (npc, level, tap, position, map) in &killed {
        let creature_type = data
            .as_deref()
            .and_then(|data| data.templates.get(&npc.template_id))
            .map(|template| template.creature_type);
        for character in
            rewarded_characters(&tap.tappers, (map, position), &positions, groups.as_deref())
        {
            let Some((player, _, life)) = lives.iter().find(|(_, id, _)| id.0 == character) else {
                continue;
            };
            let grouped = groups
                .as_deref()
                .is_some_and(|groups| !groups.group_members(character).is_empty());
            let corpse_blocks_credit = life.is_some_and(|life| {
                life.state != shared::death::DeathState::Alive && life.corpse.is_some()
            });
            if online.contains(&player) && (!grouped || !corpse_blocks_credit) {
                credits.write(CriteriaCredit {
                    player,
                    event: CriteriaEvent::KillAnyCreature {
                        creature_type,
                        level: level.map(|level| level.0),
                    },
                });
            }
        }
    }
}
