//! SimC76f8118 sc_rogue.cpp7457–7475: Deepening Shadows extends the
//! earned Shadow Dance duration using raw haste-rating percentage.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, ActiveAuras};
use combat::spell_cast::scripts::script_effect_value;
use combat::spell_info::SpellInfo;
use combat::stats::CharacterStats;
use server_catalog::character_stats_data::HASTE_RATING;
use shared::components::Level;

const DANCE: u32 = 185313;
const DEEPENING: u32 = 185314;
const DEEPENING_BUFF: u32 = 1271702;

pub(super) fn extend_duration(world: &mut World, caster: Entity, source: &SpellInfo) {
    if source.id != DANCE || !auras::has_aura(world, caster, DEEPENING) {
        return;
    }
    let base = world
        .get::<ActiveAuras>(caster)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == DANCE && aura.caster == caster)
        })
        .expect("successful Shadow Dance applies its native aura")
        .duration_ms;
    let level = world
        .get::<Level>(caster)
        .expect("Shadow Dance caster level")
        .0;
    let per_percent = HASTE_RATING[usize::from(level.clamp(1, 90) - 1)];
    let rating = world
        .get::<CharacterStats>(caster)
        .expect("Shadow Dance caster stats")
        .ratings
        .haste
        .max(0.0);
    let fraction = rating / per_percent / 100.0;
    let coefficient = script_effect_value(world, caster, DEEPENING_BUFF, 2) / 100.0;
    let duration = (base as f32 * (1.0 + coefficient * fraction)).round() as u32;
    assert!(auras::set_owned_duration(
        world, caster, caster, DANCE, duration
    ));
}
