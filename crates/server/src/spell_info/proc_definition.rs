//! TC a352b1fa SpellMgr.cpp1497–1908: explicit effective entries precede generation.
use std::collections::HashMap;

use rusqlite::{Connection, Row};

use super::{ProcDefinitionSource, SpellAuraOptions, SpellEffect, SpellInfo, SpellProcEntry};
use crate::auras::procs::{
    PROC_KILL, PROC2_CAST_SUCCESSFUL, REQ_SPELL_PHASE_MASK, TAKEN_HIT_PROC_MASK,
};

const CAN_PROC_FROM_PROCS: u32 = 0x0400_0000;
const UNRESTRICTED_DEALT_EVENTS: u32 = 0x0025_5550;
const PHASE_CAST: u32 = 1;
const PHASE_HIT: u32 = 2;
const ATTR_REQUIRES_XP_OR_HONOR: u32 = 1;
const ATTR_TRIGGERED_CAN_PROC: u32 = 2;
// TC SpellMgr.h270–277: unsupported authored attribute bits are cleared.
const ALLOWED_ATTRIBUTES: u32 = 0x019f;
// TC isTriggerAura[]; only these effects contribute generated type/family masks.
const TRIGGER_AURAS: &[u32] = &[
    4, 5, 7, 10, 11, 12, 13, 14, 16, 18, 22, 26, 28, 40, 42, 43, 51, 52, 54, 56, 57, 65, 69, 72,
    73, 74, 77, 79, 87, 96, 99, 110, 111, 112, 117, 127, 138, 139, 166, 184, 221, 226, 231, 262,
    270, 271, 319, 383, 455,
];
const ALWAYS_TRIGGERED_AURAS: &[u32] = &[5, 7, 12, 16, 18, 26, 56, 69, 96, 112, 455];

pub(super) fn load(conn: &Connection, spells: &mut [SpellInfo]) -> rusqlite::Result<()> {
    let explicit = read_explicit_entries(conn)?;
    for spell in spells {
        bind_entry(spell, &explicit);
    }
    Ok(())
}

fn read_explicit_entries(conn: &Connection) -> rusqlite::Result<HashMap<u32, SpellProcEntry>> {
    let mut statement = conn.prepare(
        "SELECT SpellId, SchoolMask, SpellFamilyName, SpellFamilyMask0, SpellFamilyMask1,
                SpellFamilyMask2, SpellFamilyMask3, ProcFlags, ProcFlags2, SpellTypeMask,
                SpellPhaseMask, HitMask, AttributesMask, DisableEffectsMask, ProcsPerMinute,
                Chance, Cooldown, Charges FROM tdb_spell_proc",
    )?;
    statement.query_map([], read_explicit_entry)?.collect()
}

fn read_explicit_entry(row: &Row<'_>) -> rusqlite::Result<(u32, SpellProcEntry)> {
    Ok((
        row.get(0)?,
        SpellProcEntry {
            school_mask: row.get(1)?,
            spell_family_name: row.get(2)?,
            spell_family_mask: [row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?],
            proc_flags: [row.get(7)?, row.get(8)?],
            spell_type_mask: row.get(9)?,
            spell_phase_mask: row.get(10)?,
            hit_mask: row.get(11)?,
            attributes_mask: row.get(12)?,
            disable_effects_mask: row.get(13)?,
            procs_per_minute: row.get::<_, f64>(14)? as f32,
            chance: row.get::<_, f64>(15)? as f32,
            cooldown_ms: row.get(16)?,
            charges: row.get(17)?,
        },
    ))
}

fn bind_entry(spell: &mut SpellInfo, explicit: &HashMap<u32, SpellProcEntry>) {
    match read_definition(spell, explicit) {
        Some((source, entry)) => {
            let options = spell.aura_options.get_or_insert_default();
            options.proc_definition_source = Some(source);
            options.proc_entry = Some(entry);
        }
        None => {
            if let Some(options) = spell.aura_options.as_mut() {
                // Non-proc stacking/charge consumers retain the unmodified DB2 fields.
                options.proc_definition_source = None;
                options.proc_entry = None;
            }
        }
    }
}

fn read_definition(
    spell: &SpellInfo,
    explicit: &HashMap<u32, SpellProcEntry>,
) -> Option<(ProcDefinitionSource, SpellProcEntry)> {
    if let Some(entry) = explicit.get(&spell.id) {
        return Some((
            ProcDefinitionSource::ExplicitTdb,
            resolve_defaults(spell, entry.clone()),
        ));
    }
    let scripted_flags = crate::auras::procs::scripted_proc_mask(spell.id);
    let entry = generate_entry(spell, scripted_flags)?;
    let source = if scripted_flags.is_some() {
        ProcDefinitionSource::Scripted
    } else {
        if refuses_default(spell, &entry) {
            return None;
        }
        ProcDefinitionSource::Generated
    };
    Some((source, entry))
}

fn resolve_defaults(spell: &SpellInfo, mut entry: SpellProcEntry) -> SpellProcEntry {
    let native = spell.aura_options.as_deref();
    if entry.proc_flags == [0; 2] {
        entry.proc_flags = native.map_or([0; 2], |options| options.proc_type_mask);
    }
    if entry.charges == 0 {
        // TC assigns uint32 SpellInfo::ProcCharges to the uint8 entry field.
        entry.charges = native.map_or(0, |options| options.proc_charges as u8);
    }
    if entry.chance == 0.0 && entry.procs_per_minute == 0.0 {
        entry.chance = native.map_or(0.0, |options| options.proc_chance as f32);
    }
    if entry.cooldown_ms == 0 {
        entry.cooldown_ms = native.map_or(0, |options| options.proc_category_recovery_ms as u32);
    }
    if entry.spell_phase_mask == 0 && needs_cast_phase(entry.proc_flags) {
        entry.spell_phase_mask = PHASE_CAST;
    }
    validate_entry(spell.id, &mut entry);
    entry
}

fn validate_entry(spell: u32, entry: &mut SpellProcEntry) {
    if entry.chance < 0.0 {
        bevy::log::warn!(
            spell,
            chance = entry.chance,
            "Negative TDB proc chance reset to zero"
        );
        entry.chance = 0.0;
    }
    if entry.procs_per_minute < 0.0 {
        bevy::log::warn!(
            spell,
            ppm = entry.procs_per_minute,
            "Negative TDB proc PPM reset to zero"
        );
        entry.procs_per_minute = 0.0;
    }
    let unsupported = entry.attributes_mask & !ALLOWED_ATTRIBUTES;
    if unsupported != 0 {
        bevy::log::warn!(
            spell,
            unsupported,
            "Unsupported TDB proc attributes cleared by TC policy"
        );
        entry.attributes_mask &= ALLOWED_ATTRIBUTES;
    }
}

fn needs_cast_phase(flags: [u32; 2]) -> bool {
    flags[0] & REQ_SPELL_PHASE_MASK == 0 && flags[1] & PROC2_CAST_SUCCESSFUL != 0
}

fn generate_entry(spell: &SpellInfo, scripted_flags: Option<[u32; 2]>) -> Option<SpellProcEntry> {
    let native = spell.aura_options.as_deref()?;
    let flags = scripted_flags.unwrap_or(native.proc_type_mask);
    if flags == [0; 2] {
        return None;
    }
    let mut entry = generated_base(spell, native, flags);
    for effect in spell
        .effects
        .iter()
        .filter(|effect| effect.effect != 0 && effect.aura != 0)
    {
        merge_generated_effect(&mut entry, effect);
    }
    if entry.spell_type_mask == 0 {
        return None;
    }
    finish_generated_scope(spell, &mut entry);
    Some(entry)
}

fn generated_base(spell: &SpellInfo, native: &SpellAuraOptions, flags: [u32; 2]) -> SpellProcEntry {
    SpellProcEntry {
        proc_flags: flags,
        spell_phase_mask: if needs_cast_phase(flags) {
            PHASE_CAST
        } else {
            PHASE_HIT
        },
        hit_mask: generated_hit_mask(spell),
        chance: native.proc_chance as f32,
        cooldown_ms: native.proc_category_recovery_ms as u32,
        charges: native.proc_charges as u8,
        ..Default::default()
    }
}

fn merge_generated_effect(entry: &mut SpellProcEntry, effect: &SpellEffect) {
    if !TRIGGER_AURAS.contains(&effect.aura) {
        entry.disable_effects_mask |= 1u32 << effect.index;
        return;
    }
    entry.spell_type_mask |= match effect.aura {
        16 => 5,                              // stealth: damage or no-damage/heal
        5 | 7 | 12 | 18 | 26 | 56 | 455 => 1, // breakable control: damage
        _ => 7,
    };
    for (combined, mask) in entry
        .spell_family_mask
        .iter_mut()
        .zip(effect.spell_class_mask)
    {
        *combined |= mask;
    }
    let always_triggered = ALWAYS_TRIGGERED_AURAS.contains(&effect.aura);
    let taken_trigger =
        entry.proc_flags[0] & TAKEN_HIT_PROC_MASK != 0 && matches!(effect.aura, 42 | 43);
    if always_triggered || taken_trigger {
        entry.attributes_mask |= ATTR_TRIGGERED_CAN_PROC;
    }
}

fn finish_generated_scope(spell: &SpellInfo, entry: &mut SpellProcEntry) {
    if entry.spell_family_mask != [0; 4] {
        entry.spell_family_name = spell
            .class_options
            .as_ref()
            .map_or(0, |options| options.spell_class_set);
    }
    if entry.proc_flags[0] & PROC_KILL != 0 {
        entry.attributes_mask |= ATTR_REQUIRES_XP_OR_HONOR;
    }
}

fn first_proc_effect(spell: &SpellInfo) -> Option<&SpellEffect> {
    spell
        .effects
        .iter()
        .filter(|effect| crate::auras::is_aura_effect(effect))
        .filter(|effect| matches!(effect.aura, 28 | 74 | 52 | 51 | 54 | 42 | 231))
        .min_by_key(|effect| effect.index)
}

fn generated_hit_mask(spell: &SpellInfo) -> u32 {
    let Some(effect) = first_proc_effect(spell) else {
        return 0;
    };
    match effect.aura {
        28 | 74 => 0x800,                             // reflect
        52 => 2,                                      // critical
        51 => 0x40,                                   // block
        54 if effect.base_points as i32 <= -100 => 4, // miss
        _ => 0,
    }
}

fn refuses_default(spell: &SpellInfo, entry: &SpellProcEntry) -> bool {
    let attributes = spell.misc.as_ref().map_or(0, |misc| misc.attributes[3]);
    if attributes & CAN_PROC_FROM_PROCS == 0 || entry.spell_family_mask != [0; 4] {
        return false;
    }
    let has_ppm = spell
        .aura_options
        .as_ref()
        .and_then(|options| options.procs_per_minute.as_ref())
        .is_some_and(|ppm| ppm.base_proc_rate > 0.0);
    let has_throttle = has_ppm || entry.cooldown_ms > 0 || entry.charges > 0;
    if entry.chance < 100.0 || has_throttle {
        return false;
    }
    let triggers_spell = first_proc_effect(spell)
        .is_some_and(|effect| matches!(effect.aura, 42 | 231) && effect.trigger_spell != 0);
    entry.proc_flags[0] & UNRESTRICTED_DEALT_EVENTS != 0 && triggers_spell
}
