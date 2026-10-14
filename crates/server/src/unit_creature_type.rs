//! TC a352b1fa Unit.cpp:9476–9500: positive player form type, else native
//! race type; NPC template type. Unknown metadata never becomes Humanoid7.
use crate::auras::{
    ActiveAuras,
    shapeshift::{ShapeshiftForms, current_form},
};
use crate::world_data::WorldData;
use bevy::prelude::*;
use shared::components::{Npc, Player};
use std::collections::HashMap;

#[derive(Resource)]
pub(crate) struct RaceCreatureTypes(HashMap<u8, u32>);

pub(crate) fn load_race_types(conn: &rusqlite::Connection) -> rusqlite::Result<RaceCreatureTypes> {
    let mut rows = conn.prepare("SELECT ID,CreatureType FROM chr_races")?;
    let types = rows
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    Ok(RaceCreatureTypes(types))
}

pub(crate) fn load_startup(mut commands: Commands) {
    let conn = rusqlite::Connection::open_with_flags(
        crate::spell_info::WORLD_DB_PATH,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .expect("open native race CreatureType catalog");
    commands
        .insert_resource(load_race_types(&conn).expect(
            "load chr_races.CreatureType; prepare the source race catalog before starting",
        ));
}

pub(crate) struct CreatureTypes<'a> {
    pub races: Option<&'a RaceCreatureTypes>,
    pub forms: Option<&'a ShapeshiftForms>,
    pub data: Option<&'a WorldData>,
}

impl<'a> CreatureTypes<'a> {
    pub fn from_world(world: &'a World) -> Self {
        Self {
            races: world.get_resource(),
            forms: world.get_resource(),
            data: world.get_resource(),
        }
    }

    pub fn resolve(
        &self,
        player: Option<&Player>,
        npc: Option<&Npc>,
        auras: Option<&ActiveAuras>,
    ) -> Result<u32, String> {
        if let Some(player) = player {
            if let Some((_, form)) = auras.and_then(current_form)
                && let Some(kind) = self
                    .forms
                    .and_then(|forms| forms.creature_type(form))
                    .filter(|kind| *kind > 0)
            {
                return Ok(kind);
            }
            return self
                .races
                .and_then(|types| types.0.get(&player.race))
                .copied()
                .ok_or_else(|| format!("missing native CreatureType for race{}", player.race));
        }
        let npc = npc.ok_or("unit has no player or NPC identity")?;
        self.data
            .and_then(|data| data.templates.get(&npc.template_id))
            .map(|row| u32::from(row.creature_type))
            .ok_or_else(|| format!("missing native CreatureType for NPC{}", npc.template_id))
    }
}

pub(crate) fn get(world: &World, unit: Entity) -> u32 {
    CreatureTypes::from_world(world)
        .resolve(
            world.get::<Player>(unit),
            world.get::<Npc>(unit),
            world.get::<ActiveAuras>(unit),
        )
        .unwrap_or_else(|err| panic!("unit{unit}: {err}"))
}

pub(crate) fn mask(kind: u32) -> u32 {
    if kind == 0 {
        0
    } else {
        1u32.checked_shl(kind - 1)
            .expect("source CreatureType must fit its native mask")
    }
}
