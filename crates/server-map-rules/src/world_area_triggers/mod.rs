#[derive(Component, Debug, Clone)]
pub(crate) struct WorldAreaTrigger {
    pub spawn_id: u64,
    pub create_properties_id: u32,
    pub create_properties_custom: bool,
    pub template_id: u32,
    pub template_custom: bool,
    pub visual_spell: Option<u32>,
    pub shape: u32,
    pub shape_data: [f32; 8],
    pub script_name: String,
    pub phase_use_flags: u8,
    pub phase_id: u32,
    pub phase_group: u32,
}


