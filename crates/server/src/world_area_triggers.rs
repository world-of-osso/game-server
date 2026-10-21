//! TDB world-static AreaTrigger identity, separate from spell-owned aura fields.
//! TC a352b1fa AreaTrigger.cpp363–376 loads an authored spawn with duration -1.
use bevy::prelude::*;

#[derive(Component, Debug, Clone)]
pub(crate) struct WorldAreaTrigger {
    pub spawn_id: u64,
    pub create_properties_id: u32,
    pub create_properties_custom: bool,
    pub template_id: u32,
    pub template_custom: bool,
    pub visual_spell: u32,
    pub shape: u32,
    pub shape_data: [f32; 8],
}
