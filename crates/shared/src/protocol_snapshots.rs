use crate::components::{CharacterAppearance, EquipmentVisualSlot};
use bevy::prelude::*;
use lightyear::prelude::*;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

// -- Quest snapshots --

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct QuestLogSnapshot {
    pub entries: Vec<QuestEntrySnapshot>,
    pub watched_quest_ids: Vec<u32>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct QuestEntrySnapshot {
    pub quest_id: u32,
    pub title: String,
    pub zone: String,
    pub completed: bool,
    pub repeatability: QuestRepeatability,
    pub objectives: Vec<QuestObjectiveSnapshot>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct QuestObjectiveSnapshot {
    pub text: String,
    pub current: u32,
    pub required: u32,
    pub completed: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum QuestRepeatability {
    Normal,
    Daily,
    Weekly,
}

// -- Group snapshots --

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GroupRosterSnapshot {
    pub is_raid: bool,
    pub ready_count: u16,
    pub total_count: u16,
    pub members: Vec<GroupMemberSnapshot>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GroupMemberSnapshot {
    pub name: String,
    pub role: GroupRoleSnapshot,
    pub is_leader: bool,
    pub online: bool,
    pub subgroup: u8,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum GroupRoleSnapshot {
    Tank,
    Healer,
    Damage,
    None,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GroupCommandResponse {
    pub message: String,
}

// -- Combat log snapshots --

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CombatLogSnapshot {
    pub entries: Vec<CombatLogEntrySnapshot>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CombatLogEntrySnapshot {
    pub kind: CombatLogEventKindSnapshot,
    pub source: String,
    pub target: String,
    pub spell: Option<String>,
    pub amount: Option<i32>,
    pub aura: Option<String>,
    pub text: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum CombatLogEventKindSnapshot {
    Damage,
    Heal,
    Interrupt,
    AuraApplied,
    Death,
}

// -- Collection snapshots --

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct CollectionSnapshot {
    pub mounts: Vec<CollectionMountSnapshot>,
    pub pets: Vec<CollectionPetSnapshot>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct CollectionMountSnapshot {
    pub mount_id: u32,
    pub name: String,
    pub known: bool,
    pub active: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct CollectionPetSnapshot {
    pub pet_id: u32,
    pub name: String,
    pub known: bool,
    pub active: bool,
}

// -- Profession snapshots --

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ProfessionSnapshot {
    pub skills: Vec<ProfessionSkillSnapshot>,
    pub recipes: Vec<ProfessionRecipeSnapshot>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ProfessionSkillSnapshot {
    pub profession: String,
    pub current: u16,
    pub max: u16,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ProfessionRecipeSnapshot {
    pub spell_id: u32,
    pub profession: String,
    pub name: String,
    pub craftable: bool,
    pub cooldown: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct CurrencySnapshot {
    pub entries: Vec<CurrencyEntrySnapshot>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct CurrencyEntrySnapshot {
    pub id: u32,
    pub name: String,
    pub amount: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ReputationSnapshot {
    pub entries: Vec<ReputationEntrySnapshot>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ReputationEntrySnapshot {
    pub faction_id: u32,
    pub faction_name: String,
    pub standing: String,
    pub value: i32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct AchievementSnapshot {
    pub earned_ids: Vec<u32>,
    pub progress: Vec<AchievementProgressSnapshot>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct AchievementProgressSnapshot {
    pub achievement_id: u32,
    pub current: u32,
    pub required: u32,
    pub completed: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct AchievementToastSnapshot {
    pub achievement_id: u32,
    pub name: String,
    pub points: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct WorldMapSnapshot {
    pub discovered_zone_ids: Vec<u32>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum RestAreaKindSnapshot {
    City,
    Inn,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct RestSnapshot {
    pub in_rest_area: bool,
    pub rest_area_kind: Option<RestAreaKindSnapshot>,
    pub rested_xp: u32,
    pub rested_xp_max: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct FriendCharacterSnapshot {
    pub name: String,
    pub level: u16,
    pub class_name: String,
    pub area: String,
    pub presence: crate::components::PresenceStatus,
    pub note: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct FriendsSnapshot {
    pub entries: Vec<FriendCharacterSnapshot>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct GuildMemberSnapshot {
    pub character_name: String,
    pub level: u16,
    pub class_name: String,
    pub rank_name: String,
    pub is_online: bool,
    pub officer_note: String,
    pub last_online: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct GuildSnapshot {
    pub guild_id: u32,
    pub guild_name: String,
    pub motd: String,
    pub info_text: String,
    pub members: Vec<GuildMemberSnapshot>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct WhoCharacterSnapshot {
    pub name: String,
    pub level: u16,
    pub class_name: String,
    pub area: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct WhoSnapshot {
    pub query: String,
    pub entries: Vec<WhoCharacterSnapshot>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum CalendarSignupStatusSnapshot {
    Confirmed,
    Tentative,
    Declined,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct CalendarSignupSnapshot {
    pub character_name: String,
    pub status: CalendarSignupStatusSnapshot,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct CalendarEventSnapshot {
    pub event_id: u64,
    pub title: String,
    pub organizer_name: String,
    pub starts_at_unix_secs: u64,
    pub max_signups: u8,
    pub is_raid: bool,
    pub signups: Vec<CalendarSignupSnapshot>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct CalendarSnapshot {
    pub events: Vec<CalendarEventSnapshot>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct IgnoreListSnapshot {
    pub names: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct LfgRoleCheckSnapshot {
    pub dungeon_id: u32,
    pub dungeon_name: String,
    pub assigned_role: GroupRoleSnapshot,
    pub accepted_count: u8,
    pub total_count: u8,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct LfgMatchMemberSnapshot {
    pub name: String,
    pub role: GroupRoleSnapshot,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct LfgMatchFoundSnapshot {
    pub dungeon_id: u32,
    pub dungeon_name: String,
    pub assigned_role: GroupRoleSnapshot,
    pub members: Vec<LfgMatchMemberSnapshot>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct LfgSnapshot {
    pub queued: bool,
    pub selected_role: Option<GroupRoleSnapshot>,
    pub dungeon_ids: Vec<u32>,
    pub queue_size: u16,
    pub average_wait_secs: u32,
    pub in_demand_roles: Vec<GroupRoleSnapshot>,
    pub role_check: Option<LfgRoleCheckSnapshot>,
    pub match_found: Option<LfgMatchFoundSnapshot>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub enum PvpBracketSnapshot {
    Arena2v2,
    Arena3v3,
    RatedBattleground,
    SoloShuffle,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct PvpBracketStatsSnapshot {
    pub bracket: PvpBracketSnapshot,
    pub rating: u32,
    pub season_wins: u32,
    pub season_losses: u32,
    pub weekly_wins: u32,
    pub weekly_losses: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum PvpQueueKindSnapshot {
    Battleground { battleground_id: u32, name: String },
    RatedBracket { bracket: PvpBracketSnapshot },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct PvpQueueSnapshot {
    pub kind: PvpQueueKindSnapshot,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct PvpSnapshot {
    pub honor: u32,
    pub honor_max: u32,
    pub conquest: u32,
    pub conquest_max: u32,
    pub brackets: Vec<PvpBracketStatsSnapshot>,
    pub queue: Option<PvpQueueSnapshot>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct BarberShopSnapshot {
    pub appearance: CharacterAppearance,
    pub gold: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum DeathStateSnapshot {
    Alive,
    Dead,
    Ghost,
    Resurrecting,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DeathPositionSnapshot {
    pub map_id: u16,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DeathSnapshot {
    pub state: DeathStateSnapshot,
    pub corpse: Option<DeathPositionSnapshot>,
    pub graveyard: Option<DeathPositionSnapshot>,
    pub can_resurrect_at_corpse: bool,
    pub spirit_healer_available: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct DurabilitySlotSnapshot {
    pub slot: EquipmentVisualSlot,
    pub current: u32,
    pub max: u32,
    pub repair_cost: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct DurabilitySnapshot {
    pub total_repair_cost: u32,
    pub slots: Vec<DurabilitySlotSnapshot>,
}

pub fn register_snapshot_messages(app: &mut App) {
    register_server_snapshot::<QuestLogSnapshot>(app);
    register_server_snapshot::<GroupRosterSnapshot>(app);
    register_server_snapshot::<GroupCommandResponse>(app);
    register_server_snapshot::<CombatLogSnapshot>(app);
    register_server_snapshot::<CollectionSnapshot>(app);
    register_server_snapshot::<ProfessionSnapshot>(app);
    register_server_snapshot::<CurrencySnapshot>(app);
    register_server_snapshot::<ReputationSnapshot>(app);
    register_server_snapshot::<AchievementSnapshot>(app);
    register_server_snapshot::<AchievementToastSnapshot>(app);
    register_server_snapshot::<WorldMapSnapshot>(app);
    register_server_snapshot::<RestSnapshot>(app);
    register_server_snapshot::<FriendsSnapshot>(app);
    register_server_snapshot::<GuildSnapshot>(app);
    register_server_snapshot::<WhoSnapshot>(app);
    register_server_snapshot::<CalendarSnapshot>(app);
    register_server_snapshot::<IgnoreListSnapshot>(app);
    register_server_snapshot::<LfgSnapshot>(app);
    register_server_snapshot::<PvpSnapshot>(app);
    register_server_snapshot::<BarberShopSnapshot>(app);
    register_server_snapshot::<DeathSnapshot>(app);
    register_server_snapshot::<DurabilitySnapshot>(app);
}

fn register_server_snapshot<M>(app: &mut App)
where
    M: lightyear::prelude::Message + Serialize + DeserializeOwned,
{
    app.register_message::<M>()
        .add_direction(NetworkDirection::ServerToClient);
}
