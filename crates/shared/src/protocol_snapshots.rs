use bevy::prelude::*;
use lightyear::prelude::*;
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

#[derive(Serialize, Deserialize, Clone, Debug)]
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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CollectionSnapshot {
    pub mounts: Vec<CollectionMountSnapshot>,
    pub pets: Vec<CollectionPetSnapshot>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CollectionMountSnapshot {
    pub mount_id: u32,
    pub name: String,
    pub known: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CollectionPetSnapshot {
    pub pet_id: u32,
    pub name: String,
    pub known: bool,
}

// -- Profession snapshots --

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProfessionSnapshot {
    pub recipes: Vec<ProfessionRecipeSnapshot>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProfessionRecipeSnapshot {
    pub spell_id: u32,
    pub profession: String,
    pub name: String,
    pub craftable: bool,
    pub cooldown: Option<String>,
}

pub fn register_snapshot_messages(app: &mut App) {
    app.register_message::<QuestLogSnapshot>()
        .add_direction(NetworkDirection::ServerToClient);
    app.register_message::<GroupRosterSnapshot>()
        .add_direction(NetworkDirection::ServerToClient);
    app.register_message::<GroupCommandResponse>()
        .add_direction(NetworkDirection::ServerToClient);
    app.register_message::<CombatLogSnapshot>()
        .add_direction(NetworkDirection::ServerToClient);
    app.register_message::<CollectionSnapshot>()
        .add_direction(NetworkDirection::ServerToClient);
    app.register_message::<ProfessionSnapshot>()
        .add_direction(NetworkDirection::ServerToClient);
}
