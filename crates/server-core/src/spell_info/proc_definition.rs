//! TC a352b1fa SpellMgr.cpp1760–1908: explicit definitions precede generation.
use std::collections::HashMap;

use rusqlite::Connection;

use super::{ProcDefinitionSource, SpellAuraOptions, SpellInfo};

pub const DANCE_OF_CHIJI: u32 = 325201;
pub const INUNDATE: u32 = 378776;
/// `ProcTypeMask_1` (TrinityCore `ProcFlags2`).
pub const PROC2_CAST_SUCCESSFUL: u32 = 0x4;

/// Existing scripted definitions: Chi-action Dance rolls (SimC monk383–392)
/// and INUNDATE_NATIVE_SUCCESSFUL_UTILITY_CASTS include successful utility casts.
pub fn scripted_proc_mask(id: u32) -> Option<[u32; 2]> {
    match id {
        DANCE_OF_CHIJI | INUNDATE => Some([0, PROC2_CAST_SUCCESSFUL]),
        _ => None,
    }
}

const CAN_PROC_FROM_PROCS: u32 = 0x0400_0000;
// TC DEAL_MELEE_ABILITY/RANGED_ATTACK/RANGED_ABILITY, helpful/harmful
// ability/spell/periodic flags (SpellMgr.cpp1893–1895).
const UNRESTRICTED_DEALT_EVENTS: u32 = 0x0025_5550;
// TC isTriggerAura[]; only these effects contribute the generated family mask.
const TRIGGER_AURAS: &[u32] = &[
    4, 5, 7, 10, 11, 12, 13, 14, 16, 18, 22, 26, 28, 40, 42, 43, 51, 52, 54, 56, 57, 65, 69, 72,
    73, 74, 77, 79, 87, 96, 99, 110, 111, 112, 117, 127, 138, 139, 166, 184, 221, 226, 231, 262,
    270, 271, 319, 383, 455,
];

pub(super) fn load(conn: &Connection, spells: &mut [SpellInfo]) -> rusqlite::Result<()> {
    let mut statement = conn.prepare(
        "SELECT SpellId, SpellFamilyName, SpellFamilyMask0, SpellFamilyMask1,
                SpellFamilyMask2, SpellFamilyMask3 FROM tdb_spell_proc",
    )?;
    let explicit = statement
        .query_map([], |row| {
            let mask =
                [2, 3, 4, 5].map(|column| row.get::<_, i64>(column).map(|value| value as u32));
            let [a, b, c, d] = mask;
            Ok((row.get(0)?, (row.get(1)?, [a?, b?, c?, d?])))
        })?
        .collect::<rusqlite::Result<HashMap<u32, (u32, [u32; 4])>>>()?;
    for spell in spells {
        let source = definition_source(spell, &explicit);
        if source.is_some() {
            let options = spell.aura_options.get_or_insert_default();
            options.proc_definition_source = source;
            options.proc_family = explicit.get(&spell.id).copied();
        } else if let Some(options) = spell.aura_options.as_mut() {
            // Stacks, charges and raw DB2 fields remain available to non-proc consumers.
            options.proc_definition_source = None;
            options.proc_family = None;
        }
    }
    Ok(())
}

fn definition_source(
    spell: &SpellInfo,
    explicit: &HashMap<u32, (u32, [u32; 4])>,
) -> Option<ProcDefinitionSource> {
    if explicit.contains_key(&spell.id) {
        return Some(ProcDefinitionSource::ExplicitTdb);
    }
    if scripted_proc_mask(spell.id).is_some() {
        return Some(ProcDefinitionSource::Scripted);
    }
    let options = spell.aura_options.as_ref()?;
    if options.proc_type_mask == [0; 2] || refuses_default(spell, options) {
        return None;
    }
    Some(ProcDefinitionSource::Generated)
}

fn refuses_default(spell: &SpellInfo, options: &SpellAuraOptions) -> bool {
    let attributes = spell.misc.as_ref().map_or(0, |misc| misc.attributes[3]);
    if attributes & CAN_PROC_FROM_PROCS == 0 || has_family_restriction(spell) {
        return false;
    }
    let has_ppm = options
        .procs_per_minute
        .as_ref()
        .is_some_and(|ppm| ppm.base_proc_rate > 0.0);
    if options.proc_chance < 100 || has_ppm {
        return false;
    }
    if options.proc_category_recovery_ms > 0 || options.proc_charges > 0 {
        return false;
    }
    if options.proc_type_mask[0] & UNRESTRICTED_DEALT_EVENTS == 0 {
        return false;
    }
    triggers_spell(spell)
}

fn has_family_restriction(spell: &SpellInfo) -> bool {
    spell
        .effects
        .iter()
        .filter(|effect| effect.effect != 0 && TRIGGER_AURAS.contains(&effect.aura))
        .any(|effect| effect.spell_class_mask.iter().any(|&mask| mask != 0))
}

fn triggers_spell(spell: &SpellInfo) -> bool {
    // TC stops at the first recognized hit-mask/trigger aura, in effect-index order.
    spell
        .effects
        .iter()
        .filter(|effect| super::is_aura_effect(effect) && effect.aura != 0)
        .find_map(|effect| match effect.aura {
            28 | 74 | 52 | 51 | 54 => Some(false), // reflect, weapon crit, block, hit chance
            42 | 231 => Some(effect.trigger_spell != 0),
            _ => None,
        })
        .unwrap_or(false)
}
