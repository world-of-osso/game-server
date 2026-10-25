//! Experience and levelling, Retail rules as TrinityCore master implements
//! them: kill XP `Trinity::XP` (`Formulas.h`) distributed by `KillRewarder`,
//! quest XP `Quest::XPValue`, and `Player::GiveXP` / `GiveLevel`.
//!
//! Tables come from Retail 12.1.0.69933 (`xp_data.rs`: gtXp, QuestXP DB2).
//! XP into the current level is stored per character in redb `character_xp`;
//! the level stays in `CharacterData`. Kill and quest rewards queue into
//! `XpAwards`; `apply_xp_awards` adds them and, on level-up, saves the level
//! and sets `Level`/`UnitLevel`. Class progression, stats and trait budgets
//! react to `Changed<Level>`; `refill_after_level_up` then sets health and
//! mana to the new maxima (`GiveLevel` `SetFullHealth` + mana
//! `SetToMaxOnLevelUp`). Each applied gain sends `LogXpGain`; the XP bar
//! (`PlayerXpUpdate`) is sent on enter world, after a gain and on any level
//! change.
//!
//! There is no ContentTuning level scaling, so every creature is an older
//! expansion than a level 30+ player; the `Gain` "10% XP for creatures of
//! lower expansion" rule is omitted, or levelling would stall past 30; the
//! same holds for the `XPValue` ÷9 on old-expansion quests at level 80+.
use std::collections::HashSet;

use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::{Health, Level, Mana, Npc, Player, Position, PowerType, UnitLevel};
use shared::death::DeathState as PlayerLifeState;
use shared::level_scaling::{LevelScaling, gray_level, level_for_viewer};
use shared::protocol::{ExperienceChannel, LogXpGain, PlayerXpUpdate, XpGainReason};

use crate::auth::InWorld;
use crate::combat::DeathState;
use crate::creature_tap::CreatureTap;
use crate::death::PlayerDeathState;
use crate::group::GroupRegistry;
use crate::networking::PlayerLink;
use crate::persistence::{self, ClientId, GameDatabase};
use crate::persistence_progression::{load_xp, save_xp};
use crate::rest::PlayerRestState;
use crate::spell_cast::powers::{PowerPools, PowerTypeTable};
use crate::world_data::WorldData;
use crate::xp_data::{QUEST_XP, XP_DIVISOR, XP_PER_KILL, XP_TO_NEXT_LEVEL};

/// Level cap of this build (gtXp row 90 is the 99999999 sentinel).
pub(crate) const MAX_LEVEL: u8 = crate::spell_info::MAX_LEVEL as u8;

/// Group members this close to the killed creature share kill XP
/// (TrinityCore `CONFIG_GROUP_XP_DISTANCE` default).
const GROUP_XP_DISTANCE: f32 = 74.0;

fn row<T: Copy>(table: &[T], level: u8) -> T {
    table[usize::from(level.max(1) - 1).min(table.len() - 1)]
}

/// XP from `level` to `level + 1` (gtXp Total); 0 at the cap.
pub(crate) fn xp_to_next_level(level: u8) -> u32 {
    if level >= MAX_LEVEL {
        0
    } else {
        row(&XP_TO_NEXT_LEVEL, level)
    }
}

/// `Trinity::XP::GetZeroDifference`.
fn zero_difference(player_level: u8) -> u8 {
    match player_level {
        0..4 => 5,
        4..10 => 6,
        10..12 => 7,
        12..16 => 8,
        16..20 => 9,
        20..30 => 11,
        30..40 => 12,
        40..45 => 13,
        45..50 => 14,
        50..55 => 15,
        55..60 => 16,
        _ => 17,
    }
}

/// `Trinity::XP::BaseGain` over gtXp PerKill/Divisor.
pub(crate) fn base_gain(player_level: u8, mob_level: u8) -> u32 {
    if mob_level >= player_level {
        let diff = f32::from((mob_level - player_level).min(4));
        return (row(&XP_PER_KILL, player_level) * (1.0 + 0.05 * diff)).round() as u32;
    }
    if mob_level <= gray_level(player_level) {
        return 0;
    }
    let below = f32::from(player_level - mob_level) / f32::from(zero_difference(player_level));
    let divisor = row(&XP_DIVISOR, mob_level) / row(&XP_DIVISOR, player_level);
    (row(&XP_PER_KILL, mob_level) * ((1.0 - below) * divisor)).round() as u32
}

/// What a killed creature contributes to kill XP.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct KillVictim {
    pub level: UnitLevel,
    /// A tuned creature is killed at each player's scaled level.
    pub scaling: Option<LevelScaling>,
    pub elite: bool,
    pub experience_modifier: f32,
    pub gives_experience: bool,
    /// `1 - 2 * m_PlayerDamageReq / max health` while players dealt less than half
    /// its health (`creature_tap::CreatureTap::xp_factor`).
    pub player_damage_factor: f32,
}

impl KillVictim {
    /// `victim->GetLevelForTarget(player)` (Formulas.h:190, KillRewarder.cpp:110).
    fn level_for(&self, player_level: u8) -> u8 {
        level_for_viewer(self.level, self.scaling.as_ref(), player_level)
    }
}

/// `Trinity::XP::Gain` for an open-world kill (no dungeon maps exist, so
/// elites get the 2x world bonus, not 2.75x).
pub(crate) fn kill_gain(player_level: u8, victim: &KillVictim) -> u32 {
    if !victim.gives_experience {
        return 0;
    }
    let gain = base_gain(player_level, victim.level_for(player_level));
    let elite = if victim.elite { 2.0 } else { 1.0 };
    (gain as f32 * elite * victim.experience_modifier * victim.player_damage_factor) as u32
}

/// `Trinity::XP::xp_in_group_rate` outside raids.
fn group_rate(count: usize) -> f32 {
    match count {
        0..=2 => 1.0,
        3 => 1.166,
        4 => 1.3,
        _ => 1.4,
    }
}

/// A player the kill can reward: the killer, or a group member within
/// reward distance of the creature.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct KillMember {
    pub character_id: u64,
    pub level: u8,
    pub alive: bool,
    pub is_killer: bool,
}

/// Kill XP per member plus the group rate `GiveXP` reports.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct KillRewards {
    pub group_bonus: f32,
    /// `(character_id, xp)` for every member with XP.
    pub shares: Vec<(u64, u32)>,
}

/// `KillRewarder::Reward` XP. `grouped` is whether the killer is in a group.
pub(crate) fn kill_rewards(
    victim: &KillVictim,
    members: &[KillMember],
    grouped: bool,
) -> KillRewards {
    if !grouped {
        let shares = members
            .iter()
            .filter(|member| member.is_killer && member.alive)
            .map(|member| (member.character_id, kill_gain(member.level, victim)))
            .filter(|&(_, xp)| xp > 0)
            .collect();
        return KillRewards {
            group_bonus: 1.0,
            shares,
        };
    }
    // `_InitGroupData`: the killer plus alive members count.
    let counted: Vec<&KillMember> = members
        .iter()
        .filter(|member| member.is_killer || member.alive)
        .collect();
    let sum_level: f32 = counted.iter().map(|member| f32::from(member.level)).sum();
    let max_level = counted.iter().map(|member| member.level).max().unwrap_or(0);
    let Some(max_not_gray) = counted
        .iter()
        .filter(|member| victim.level_for(member.level) > gray_level(member.level))
        .map(|member| member.level)
        .max()
    else {
        return KillRewards {
            group_bonus: 1.0,
            shares: Vec::new(),
        };
    };
    let full_xp = max_level == max_not_gray;
    let xp = kill_gain(max_not_gray, victim);
    let group_rate = group_rate(counted.len());
    let shares = members
        .iter()
        .filter(|member| member.alive && member.level <= max_not_gray)
        .map(|member| {
            let rate = group_rate * f32::from(member.level) / sum_level;
            let share = if full_xp {
                (xp as f32 * rate) as u32
            } else {
                (xp as f32 * rate / 2.0) as u32 + 1
            };
            (member.character_id, share)
        })
        .filter(|&(_, xp)| xp > 0)
        .collect();
    KillRewards {
        group_bonus: group_rate,
        shares,
    }
}

/// `Quest::RoundXPValue`.
fn round_xp_value(xp: u32) -> u32 {
    match xp {
        0..=100 => 5 * ((xp + 2) / 5),
        101..=500 => 10 * ((xp + 5) / 10),
        501..=1000 => 25 * ((xp + 12) / 25),
        _ => 50 * ((xp + 25) / 50),
    }
}

/// `Quest::XPValue`. Imported quests carry a fixed `QuestLevel` (-1 = the
/// player's level) instead of Retail ContentTuning, so a quest far below the
/// player keeps the `diffFactor` reduction (10/10 down to 1/10). The ÷9 for
/// old-expansion quests at level 80+ is omitted: all imported quests are
/// Wrath content, so it would cut every quest past 80.
pub(crate) fn quest_xp(quest_level: i32, player_level: u8, difficulty: u8) -> u32 {
    let quest_level = if quest_level > 0 {
        quest_level
    } else {
        i32::from(player_level)
    };
    let Some(xp_row) = QUEST_XP.get((quest_level - 1) as usize) else {
        return 0;
    };
    let Some(&base) = xp_row.get(usize::from(difficulty)) else {
        return 0;
    };
    let diff_factor = (2 * (quest_level - i32::from(player_level)) + 12).clamp(1, 10) as u32;
    round_xp_value(diff_factor * base / 10)
}

/// `Player::GiveXP` levelling loop: `(level, xp)` after adding `amount`.
/// Nothing changes at the cap.
pub(crate) fn give_xp(level: u8, xp: u32, amount: u32) -> (u8, u32) {
    if level >= MAX_LEVEL {
        return (level, xp);
    }
    let mut level = level;
    let mut xp = xp + amount;
    while level < MAX_LEVEL && xp >= xp_to_next_level(level) {
        xp -= xp_to_next_level(level);
        level += 1;
    }
    (level, xp)
}

/// One queued gain; `victim` is set for kill XP.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct XpAward {
    pub character_id: u64,
    pub amount: u32,
    pub victim: Option<Entity>,
    pub group_bonus: f32,
    pub reason: XpGainReason,
}

impl XpAward {
    pub(crate) fn quest(character_id: u64, amount: u32) -> Self {
        Self {
            character_id,
            amount,
            victim: None,
            group_bonus: 1.0,
            reason: XpGainReason::Quest,
        }
    }
}

/// XP to add this frame.
#[derive(Resource, Default)]
pub(crate) struct XpAwards(pub Vec<XpAward>);

/// Characters whose XP changed this frame and need a `PlayerXpUpdate`.
#[derive(Resource, Default)]
pub(crate) struct XpDirty(pub(crate) HashSet<u64>);

/// Set on a level-up; health and mana refill once stats are recomputed.
#[derive(Component, Debug)]
pub(crate) struct RefillAfterLevelUp;

pub struct ExperiencePlugin;

impl Plugin for ExperiencePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<crate::achievement::CriteriaCredit>();
        app.init_resource::<XpAwards>();
        app.init_resource::<XpDirty>();
        app.add_systems(
            Update,
            refill_after_level_up.after(crate::stats::update_character_stats),
        );
        app.add_systems(
            PostUpdate,
            (award_kill_xp, apply_xp_awards, send_xp_updates)
                .chain()
                .before(ReplicationSystems::Send),
        );
    }
}

fn distance(a: &Position, b: &Position) -> f32 {
    ((a.x - b.x).powi(2) + (a.y - b.y).powi(2) + (a.z - b.z).powi(2)).sqrt()
}

fn is_alive(death: Option<&PlayerDeathState>) -> bool {
    death.is_none_or(|death| death.state == PlayerLifeState::Alive)
}

fn victim_of(
    world_data: &WorldData,
    npc: &Npc,
    level: &UnitLevel,
    scaling: Option<&LevelScaling>,
) -> Option<KillVictim> {
    let template = world_data.templates.get(&npc.template_id)?;
    Some(KillVictim {
        level: *level,
        scaling: scaling.copied(),
        elite: matches!(template.rank, 1 | 2),
        experience_modifier: template.experience_modifier,
        gives_experience: template.gives_experience,
        player_damage_factor: 1.0,
    })
}

type KillPlayers<'w, 's> = Query<
    'w,
    's,
    (
        &'static ClientId,
        &'static Level,
        &'static Position,
        Option<&'static PlayerDeathState>,
        &'static crate::maps::InMap,
        Option<&'static crate::auras::ActiveAuras>,
    ),
    With<Player>,
>;

/// Kill XP for each tapper of a dead creature (`creature_tap`), as `KillRewarder::Reward`:
/// a grouped tapper's group once, an ungrouped tapper alone.
fn award_kill_xp(
    killed: Query<
        (
            Entity,
            &Npc,
            &UnitLevel,
            Option<&LevelScaling>,
            &CreatureTap,
            &Health,
            &Position,
            &crate::maps::InMap,
        ),
        Added<DeathState>,
    >,
    players: KillPlayers,
    groups: Option<Res<GroupRegistry>>,
    world_data: Option<Res<WorldData>>,
    mut awards: ResMut<XpAwards>,
) {
    let Some(world_data) = world_data else {
        return;
    };
    for (npc_entity, npc, level, scaling, tap, health, position, map) in &killed {
        let Some(mut victim) = victim_of(&world_data, npc, level, scaling) else {
            continue;
        };
        victim.player_damage_factor = tap.xp_factor(health.max);
        let mut processed_groups: Vec<u64> = Vec::new();
        for &killer in tap.tappers.iter().filter(|&&tapper| {
            players
                .iter()
                .any(|(id, _, _, _, tapper_map, _)| id.0 == tapper && tapper_map == map)
        }) {
            let group = groups
                .as_deref()
                .map_or_else(Vec::new, |groups| groups.group_members(killer));
            if let Some(&group_key) = group.iter().min() {
                if processed_groups.contains(&group_key) {
                    continue;
                }
                processed_groups.push(group_key);
            }
            let members: Vec<KillMember> = players
                .iter()
                .filter(|(id, _, member_position, _, member_map, _)| {
                    id.0 == killer
                        || (group.contains(&id.0)
                            && *member_map == map
                            && distance(member_position, position) <= GROUP_XP_DISTANCE)
                })
                .map(|(id, level, _, death, _, _)| KillMember {
                    character_id: id.0,
                    level: level.0,
                    alive: is_alive(death),
                    is_killer: id.0 == killer,
                })
                .collect();
            let rewards = kill_rewards(&victim, &members, !group.is_empty());
            awards
                .0
                .extend(rewards.shares.into_iter().map(|(character_id, amount)| {
                    XpAward {
                        character_id,
                        amount: crate::auras::reward_mods::kill_xp(
                            amount,
                            players
                                .iter()
                                .find(|(id, ..)| id.0 == character_id)
                                .and_then(|(_, _, _, _, _, auras)| auras),
                        ),
                        victim: Some(npc_entity),
                        group_bonus: rewards.group_bonus,
                        reason: XpGainReason::Kill,
                    }
                }));
        }
    }
}

type XpPlayers<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static ClientId,
        &'static Player,
        &'static mut Level,
        Option<&'static mut UnitLevel>,
        Option<&'static PlayerDeathState>,
        Option<&'static mut PlayerRestState>,
    ),
    With<Player>,
>;

type XpConnections<'w, 's> = Query<'w, 's, (Entity, Ref<'static, InWorld>, &'static PlayerLink)>;

fn connection_of(connections: &XpConnections, character_id: u64) -> Option<Entity> {
    connections
        .iter()
        .find(|(_, in_world, _)| in_world.character_id == character_id)
        .map(|(connection, ..)| connection)
}

/// `Player::GiveXP` for queued awards: dead players and the cap gain nothing; kill XP
/// adds the rested bonus it spends (Player.cpp:2173 `GetRestBonusFor`), `original`
/// counting it and `amount` not (`LogXPGain`).
fn apply_xp_awards(
    time: Res<Time>,
    mut awards: ResMut<XpAwards>,
    mut dirty: ResMut<XpDirty>,
    db: Option<Res<GameDatabase>>,
    mut players: XpPlayers,
    mut criteria: MessageWriter<crate::achievement::CriteriaCredit>,
    connections: XpConnections,
    mut senders: Query<&mut MessageSender<LogXpGain>>,
    mut commands: Commands,
) {
    let Some(db) = db else {
        awards.0.clear();
        return;
    };
    for award in std::mem::take(&mut awards.0) {
        let XpAward {
            character_id,
            amount,
            ..
        } = award;
        let Some((entity, _, player, mut level, unit_level, death, rest)) =
            players.iter_mut().find(|(_, id, ..)| id.0 == character_id)
        else {
            continue;
        };
        if !is_alive(death) || level.0 >= MAX_LEVEL {
            continue;
        }
        let original = match (award.victim, rest) {
            (Some(_), Some(mut rest)) => rest.spend_on_kill(amount, time.elapsed_secs_f64()),
            _ => amount,
        };
        let (new_level, xp) = give_xp(level.0, load_xp(&db.db, character_id), original);
        save_xp(&db.db, character_id, xp);
        dirty.0.insert(character_id);
        if let Some(mut sender) = connection_of(&connections, character_id)
            .and_then(|connection| senders.get_mut(connection).ok())
        {
            sender.send::<ExperienceChannel>(LogXpGain {
                victim: award.victim.map(Entity::to_bits),
                original,
                amount,
                group_bonus: award.group_bonus,
                reason: award.reason,
            });
        }
        if new_level == level.0 {
            continue;
        }
        if let Some(mut data) = persistence::load_character(&db.db, character_id) {
            data.level = u16::from(new_level);
            persistence::save_character(&db.db, character_id, &data);
        }
        let gained_levels = new_level - level.0;
        level.0 = new_level;
        // TC Player::GiveLevel2290-2294: earned transition only, not loaded/admin Level.
        for event in [
            crate::achievement::CriteriaEvent::ActivelyReachLevel(new_level),
            crate::achievement::CriteriaEvent::GainLevels(gained_levels),
        ] {
            criteria.write(crate::achievement::CriteriaCredit {
                player: entity,
                event,
            });
        }
        if new_level == MAX_LEVEL {
            criteria.write(crate::achievement::CriteriaCredit {
                player: entity,
                event: crate::achievement::CriteriaEvent::ReachMaxLevel(player.class),
            });
        }
        if let Some(mut unit_level) = unit_level {
            unit_level.0 = new_level;
        }
        commands.entity(entity).insert(RefillAfterLevelUp);
    }
}

/// `PlayerXpUpdate` on enter world, after a gain and on any level change.
fn send_xp_updates(
    mut dirty: ResMut<XpDirty>,
    db: Option<Res<GameDatabase>>,
    connections: XpConnections,
    players: Query<(Ref<Level>, Option<&PlayerRestState>)>,
    mut senders: Query<&mut MessageSender<PlayerXpUpdate>>,
) {
    let changed = std::mem::take(&mut dirty.0);
    let Some(db) = db else {
        return;
    };
    for (connection, in_world, link) in &connections {
        let Ok((level, rest)) = players.get(link.0) else {
            continue;
        };
        let character_id = in_world.character_id;
        if !(in_world.is_added() || level.is_changed() || changed.contains(&character_id)) {
            continue;
        }
        let Ok(mut sender) = senders.get_mut(connection) else {
            continue;
        };
        sender.send::<ExperienceChannel>(PlayerXpUpdate {
            xp: load_xp(&db.db, character_id),
            next_level_xp: xp_to_next_level(level.0),
            rested_xp: rest.map_or(0, PlayerRestState::rested_xp),
        });
    }
}

type RefillQuery = (
    Entity,
    &'static mut Health,
    Option<&'static mut Mana>,
    Option<&'static mut PowerPools>,
);

/// `Player::GiveLevel` (Player.cpp:2266-2270): `SetFullHealth`, then `SetFullPower` for
/// every class power whose `PowerType` has `SetToMaxOnLevelUp` (Mana, Energy, Soul Shards,
/// Essence); Rage, Focus and Runic Power keep their value.
pub(crate) fn refill_after_level_up(
    mut commands: Commands,
    mut players: Query<RefillQuery, With<RefillAfterLevelUp>>,
    table: Option<Res<PowerTypeTable>>,
    replication_dirty: Option<Res<ReplicationDirtyQueue>>,
) {
    let fills = |power: PowerType| {
        table
            .as_deref()
            .and_then(|table| table.get(power))
            .is_some_and(|info| info.set_to_max_on_level_up)
    };
    for (entity, mut health, mana, pools) in &mut players {
        health.current = health.max;
        if let Some(mut mana) = mana.filter(|_| fills(PowerType::Mana)) {
            mana.current = mana.max;
        }
        if let Some(mut pools) = pools {
            for pool in pools.pools.iter_mut().filter(|pool| fills(pool.power)) {
                pool.current = pool.max;
            }
        }
        commands.entity(entity).remove::<RefillAfterLevelUp>();
        if let Some(dirty) = replication_dirty.as_deref() {
            dirty.mark(entity);
        }
    }
}

#[cfg(test)]
#[path = "experience_tests.rs"]
mod tests;
