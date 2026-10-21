//! Source-backed conversation runtime/payload. Protocol and TalkingHead presentation
//! follow the server slice; no replacement chat event is emitted here.
use bevy::prelude::*;
use shared::components::Position;

#[derive(Resource, Clone)]
pub(crate) struct Catalog;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ActorObject {
    WorldObject(u64),
    NoObject { creature_id: u32, display_id: u32 },
    ActivePlayer,
    TalkingHead { creature_id: u32, display_id: u32 },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ActorPayload {
    pub id: i32,
    pub index: u8,
    pub name: Option<String>,
    pub object: ActorObject,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BroadcastTextPayload {
    pub male_text: String,
    pub female_text: String,
    pub sound_kit_id: u32,
    pub language_id: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LinePayload {
    pub id: u32,
    pub broadcast_text_id: u32,
    pub ui_camera_id: u32,
    pub actor_index: u8,
    pub flags: u8,
    pub chat_type: u8,
    pub start_ms: i64,
    pub source_duration_ms: Option<i32>,
    pub additional_duration_ms: i32,
    pub visual_kit_id: u32,
    pub animation_kit_id: u32,
    pub start_animation: u16,
    pub end_animation: u16,
    pub text: Option<BroadcastTextPayload>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ConversationPayload {
    pub id: u64,
    pub entry: u32,
    pub creator: u64,
    pub private_owner: u64,
    pub texture_kit_id: u32,
    pub flags: u8,
    pub actors: Vec<ActorPayload>,
    pub lines: Vec<LinePayload>,
    pub last_line_end_ms: i64,
    pub duration_ms: u64,
    pub elapsed_ms: u64,
}

pub(crate) fn load_catalog(_connection: &rusqlite::Connection) -> rusqlite::Result<Catalog> {
    Ok(Catalog)
}

pub(crate) fn visible_for(_world: &mut World, _viewer: Entity) -> Vec<ConversationPayload> {
    Vec::new()
}

pub(crate) fn create_private(
    _world: &mut World,
    _entry: u32,
    _creator: Entity,
    _position: Position,
    _owner: Entity,
) -> Result<Entity, String> {
    Err("conversation runtime is not implemented".into())
}

pub(crate) struct ConversationPlugin;
impl Plugin for ConversationPlugin {
    fn build(&self, _app: &mut App) {}
}
