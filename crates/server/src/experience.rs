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
//! `SetToMaxOnLevelUp`).
use bevy::prelude::*;
use lightyear::prelude::ReplicationDirtyQueue;
use shared::components::{Health, Level, Mana, Npc, Player, Position, UnitLevel};
use shared::death::DeathState as PlayerLifeState;

use crate::combat::{DeathState, LastDamager};
use crate::death::PlayerDeathState;
use crate::group::GroupRegistry;
use crate::persistence::{self, ClientId, GameDatabase};
use crate::persistence_progression::{load_xp, save_xp};
use crate::world_data::WorldData;
use crate::xp_data::{QUEST_XP, XP_DIVISOR, XP_PER_KILL, XP_TO_NEXT_LEVEL};

/// Level cap of this build (gtXp row 90 is the 99999999 sentinel).
pub(crate) const MAX_LEVEL: u8 = crate::spell_info::MAX_LEVEL as u8;

/// Group members this close to the killed creature share kill XP
/// (TrinityCore `CONFIG_GROUP_XP_DISTANCE` default).
const GROUP_XP_DISTANCE: f32 = 74.0;

/// `EXPANSION_LEVEL_CURRENT` for 12.x: Midnight.
const CURRENT_EXPANSION: u8 = 11;
/// `GetMaxLevelForExpansion(CURRENT_EXPANSION - 1)`: The War Within cap.
const PREVIOUS_EXPANSION_MAX_LEVEL: u8 = 80;
/// Quest content is AzerothCore Wrath data ([[content-import]]).
const QUEST_CONTENT_EXPANSION: u8 = 2;

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

/// `Trinity::XP::GetGrayLevel`: creatures at or below this level are grey.
pub(crate) fn gray_level(player_level: u8) -> u8 {
    match player_level {
        0..7 => 0,
        7..35 => {
            let multiples_of_five_from_15 =
                (15..=player_level).filter(|l| l % 5 == 0).count() as u8;
            player_level - 7 - multiples_of_five_from_15 + 1
        }
        _ => player_level - 10,
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

/// `GetExpansionForLevel` (TrinityCore `Formulas.h`).
fn expansion_for_level(level: u8) -> u8 {
    match level {
        0..30 => 0,
        30..35 => 3,
        35..40 => 5,
        40..45 => 7,
        45..50 => 4,
        50..60 => 8,
        60..70 => 9,
        70..80 => 10,
        _ => CURRENT_EXPANSION,
    }
}

/// What a killed creature contributes to kill XP.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct KillVictim {
    pub level: u8,
    pub elite: bool,
    pub expansion: u8,
    pub experience_modifier: f32,
    pub gives_experience: bool,
}

/// `Trinity::XP::Gain` for an open-world kill (no dungeon maps exist, so
/// elites get the 2x world bonus, not 2.75x).
pub(crate) fn kill_gain(player_level: u8, victim: &KillVictim) -> u32 {
    if !victim.gives_experience {
        return 0;
    }
    let mut gain = base_gain(player_level, victim.level);
    if gain == 0 {
        return 0;
    }
    if victim.expansion < expansion_for_level(player_level) {
        gain = (gain as f32 / 10.0).round() as u32;
    }
    let elite = if victim.elite { 2.0 } else { 1.0 };
    (gain as f32 * elite * victim.experience_modifier) as u32
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

/// `KillRewarder::Reward` XP: `(character_id, xp)` for every member with XP.
/// `grouped` is whether the killer is in a group.
pub(crate) fn kill_rewards(
    victim: &KillVictim,
    members: &[KillMember],
    grouped: bool,
) -> Vec<(u64, u32)> {
    if !grouped {
        return members
            .iter()
            .filter(|member| member.is_killer && member.alive)
            .map(|member| (member.character_id, kill_gain(member.level, victim)))
            .filter(|&(_, xp)| xp > 0)
            .collect();
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
        .filter(|member| victim.level > gray_level(member.level))
        .map(|member| member.level)
        .max()
    else {
        return Vec::new();
    };
    let full_xp = max_level == max_not_gray;
    let xp = kill_gain(max_not_gray, victim);
    let group_rate = group_rate(counted.len());
    members
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
        .collect()
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
/// player keeps the `diffFactor` reduction (10/10 down to 1/10).
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
    let mut xp = diff_factor * base / 10;
    if player_level >= PREVIOUS_EXPANSION_MAX_LEVEL && QUEST_CONTENT_EXPANSION < CURRENT_EXPANSION {
        xp = (xp as f32 / 9.0) as u32;
    }
    round_xp_value(xp)
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

/// XP to add this frame: `(character_id, xp)`.
#[derive(Resource, Default)]
pub(crate) struct XpAwards(pub Vec<(u64, u32)>);

/// Set on a level-up; health and mana refill once stats are recomputed.
#[derive(Component, Debug)]
pub(crate) struct RefillAfterLevelUp;

pub struct ExperiencePlugin;

impl Plugin for ExperiencePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<XpAwards>();
        app.add_systems(
            Update,
            refill_after_level_up.after(crate::stats::update_character_stats),
        );
        app.add_systems(PostUpdate, (award_kill_xp, apply_xp_awards).chain());
    }
}

fn distance(a: &Position, b: &Position) -> f32 {
    ((a.x - b.x).powi(2) + (a.y - b.y).powi(2) + (a.z - b.z).powi(2)).sqrt()
}

fn is_alive(death: Option<&PlayerDeathState>) -> bool {
    death.is_none_or(|death| death.state == PlayerLifeState::Alive)
}

fn victim_of(world_data: &WorldData, npc: &Npc, level: &UnitLevel) -> Option<KillVictim> {
    let template = world_data.templates.get(&npc.template_id)?;
    Some(KillVictim {
        level: level.0,
        elite: matches!(template.rank, 1 | 2),
        expansion: template.expansion,
        experience_modifier: template.experience_modifier,
        gives_experience: template.gives_experience,
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
    ),
    With<Player>,
>;

/// Kill XP for NPC deaths credited to a player (`LastDamager`).
fn award_kill_xp(
    killed: Query<(&Npc, &UnitLevel, &LastDamager, &Position), Added<DeathState>>,
    players: KillPlayers,
    groups: Option<Res<GroupRegistry>>,
    world_data: Option<Res<WorldData>>,
    mut awards: ResMut<XpAwards>,
) {
    let Some(world_data) = world_data else {
        return;
    };
    for (npc, level, killer, position) in &killed {
        let Ok((killer_id, ..)) = players.get(killer.0) else {
            continue;
        };
        let Some(victim) = victim_of(&world_data, npc, level) else {
            continue;
        };
        let group = groups
            .as_deref()
            .map_or_else(Vec::new, |groups| groups.group_members(killer_id.0));
        let members: Vec<KillMember> = players
            .iter()
            .filter(|(id, _, member_position, _)| {
                id.0 == killer_id.0
                    || (group.contains(&id.0)
                        && distance(member_position, position) <= GROUP_XP_DISTANCE)
            })
            .map(|(id, level, _, death)| KillMember {
                character_id: id.0,
                level: level.0,
                alive: is_alive(death),
                is_killer: id.0 == killer_id.0,
            })
            .collect();
        awards
            .0
            .extend(kill_rewards(&victim, &members, !group.is_empty()));
    }
}

type XpPlayers<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static ClientId,
        &'static mut Level,
        Option<&'static mut UnitLevel>,
        Option<&'static PlayerDeathState>,
    ),
    With<Player>,
>;

/// `Player::GiveXP` for queued awards: dead players and the cap gain nothing.
fn apply_xp_awards(
    mut awards: ResMut<XpAwards>,
    db: Option<Res<GameDatabase>>,
    mut players: XpPlayers,
    mut commands: Commands,
) {
    let Some(db) = db else {
        awards.0.clear();
        return;
    };
    for (character_id, amount) in std::mem::take(&mut awards.0) {
        let Some((entity, _, mut level, unit_level, death)) =
            players.iter_mut().find(|(_, id, ..)| id.0 == character_id)
        else {
            continue;
        };
        if !is_alive(death) || level.0 >= MAX_LEVEL {
            continue;
        }
        let (new_level, xp) = give_xp(level.0, load_xp(&db.db, character_id), amount);
        save_xp(&db.db, character_id, xp);
        if new_level == level.0 {
            continue;
        }
        if let Some(mut data) = persistence::load_character(&db.db, character_id) {
            data.level = u16::from(new_level);
            persistence::save_character(&db.db, character_id, &data);
        }
        level.0 = new_level;
        if let Some(mut unit_level) = unit_level {
            unit_level.0 = new_level;
        }
        commands.entity(entity).insert(RefillAfterLevelUp);
    }
}

fn refill_after_level_up(
    mut commands: Commands,
    mut players: Query<(Entity, &mut Health, Option<&mut Mana>), With<RefillAfterLevelUp>>,
    replication_dirty: Option<Res<ReplicationDirtyQueue>>,
) {
    for (entity, mut health, mana) in &mut players {
        health.current = health.max;
        if let Some(mut mana) = mana {
            mana.current = mana.max;
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
