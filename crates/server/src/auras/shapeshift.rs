//! MOD_SHAPESHIFT 36 (`AuraEffect::HandleAuraModShapeshift`): one form at a time, whose
//! `SpellShapeshiftForm` row gives the model (`Unit::GetModelForForm`; the display shows
//! through `unit_state`'s transform/form/native priority) and whose form picks the display
//! power (`Unit::CalculateDisplayPowerType`), the first entry of `UnitPowers`.

use std::collections::HashMap;
use std::path::Path;

use bevy::prelude::*;
use rusqlite::Connection;
use shared::components::{Player, PowerType};

use super::handlers::MOD_SHAPESHIFT;
use super::{ActiveAuras, AuraInstance, RemoveReason};
use crate::spell_cast::powers::{PowerPools, PowerTypeTable};

/// `ShapeshiftForm` IDs (`SpellAuraDefines.h`) with a fixed display power.
const FORM_CAT: i32 = 1;
const FORM_TRAVEL: i32 = 3;
const FORM_BEAR: i32 = 5;
const FORM_GHOUL: i32 = 7;
const FORM_DIRE_BEAR: i32 = 8;
const FORM_GHOST_WOLF: i32 = 16;

#[derive(Debug)]
struct ShapeshiftForm {
    display_id: u32,
    flags: u32,
    creature_type: u32,
}

/// Authored display and cast-gating flags by form ID.
#[derive(Resource, Debug, Default)]
pub struct ShapeshiftForms(HashMap<i32, ShapeshiftForm>);

impl ShapeshiftForms {
    /// `GetModelForForm` fallback: the form's own display (0 keeps the current model).
    /// The race/gender `ChrCustomizationDisplayInfo` choice is not modelled.
    pub fn display_id(&self, form: i32) -> Option<u32> {
        self.0
            .get(&form)
            .map(|row| row.display_id)
            .filter(|&display| display != 0)
    }

    pub(crate) fn creature_type(&self, form: i32) -> Option<u32> {
        self.0.get(&form).map(|row| row.creature_type)
    }

    pub(crate) fn flags(&self, form: i32) -> Option<u32> {
        self.0.get(&form).map(|row| row.flags)
    }
}

pub fn load_shapeshift_forms(conn: &Connection) -> rusqlite::Result<ShapeshiftForms> {
    let mut stmt = conn
        .prepare("SELECT ID, CreatureDisplayID, Flags, CreatureType FROM spell_shapeshift_form")?;
    let forms = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)? as i32,
                ShapeshiftForm {
                    display_id: r.get::<_, i64>(1)? as u32,
                    flags: r.get::<_, i64>(2)? as u32,
                    creature_type: r.get(3)?,
                },
            ))
        })?
        .collect::<rusqlite::Result<_>>()?;
    Ok(ShapeshiftForms(forms))
}

pub(super) fn load_shapeshift_forms_startup(mut commands: Commands) {
    let path = Path::new(crate::spell_info::WORLD_DB_PATH);
    let forms = if path.exists() {
        let conn = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .expect("Failed to open world.db for shapeshift forms");
        load_shapeshift_forms(&conn).expect("Failed to load spell_shapeshift_form from world.db")
    } else {
        warn!("world.db not found, shapeshift forms empty");
        ShapeshiftForms::default()
    };
    commands.insert_resource(forms);
}

/// The form of `aura`'s MOD_SHAPESHIFT effect (`EffectMiscValue_0`).
pub(super) fn aura_form(aura: &AuraInstance) -> Option<i32> {
    aura.effects
        .iter()
        .find(|effect| effect.aura_type == MOD_SHAPESHIFT)
        .map(|effect| effect.misc_values[0])
}

/// The unit's shapeshift aura and its form; `remove_other_forms` keeps at most one.
pub(crate) fn current_form(auras: &ActiveAuras) -> Option<(&AuraInstance, i32)> {
    auras
        .auras
        .iter()
        .rev()
        .find_map(|aura| aura_form(aura).map(|form| (aura, form)))
}

/// "remove other shapeshift before applying a new one":
/// `RemoveAurasByType(SPELL_AURA_MOD_SHAPESHIFT, ObjectGuid::Empty, GetBase())`.
pub(super) fn remove_other_forms(world: &mut World, unit: Entity, keep_instance: u32) {
    super::remove_auras(world, unit, RemoveReason::Cancelled, |aura| {
        aura.instance_id != keep_instance && aura_form(aura).is_some()
    });
}

/// TC a352b1fa SpellAuraEffects.cpp1513–1516/1570–1617: native form2 boosts.
const TREE_OF_LIFE_FORM: i32 = 2;
const TREE_BOOSTS: &[u32] = &[5420, 81097];

pub(super) fn apply_form_boosts(world: &mut World, unit: Entity, aura: &AuraInstance) {
    if aura_form(aura) != Some(TREE_OF_LIFE_FORM) {
        return;
    }
    for &spell in TREE_BOOSTS {
        if !crate::spell_cast::try_cast_triggered(world, unit, spell, Some(unit), 1) {
            warn!(?unit, spell, "Native Tree of Life form boost refused");
        }
    }
}

pub(super) fn remove_form_boosts(world: &mut World, unit: Entity, aura: &AuraInstance) {
    if aura_form(aura) != Some(TREE_OF_LIFE_FORM) {
        return;
    }
    super::remove_auras(world, unit, RemoveReason::Cancelled, |boost| {
        boost.caster == unit && TREE_BOOSTS.contains(&boost.spell_id)
    });
}

/// `Unit::CalculateDisplayPowerType`: fixed forms precede aura179 and class power.
fn display_power(form: Option<i32>, class_display: Option<PowerType>) -> Option<PowerType> {
    match form {
        Some(FORM_CAT | FORM_GHOUL) => Some(PowerType::Energy),
        Some(FORM_BEAR | FORM_DIRE_BEAR) => Some(PowerType::Rage),
        Some(FORM_TRAVEL | FORM_GHOST_WOLF) => Some(PowerType::Mana),
        _ => class_display,
    }
}

/// `Player::InitDataForForm` → `UpdateDisplayPower`: the display power leads `order`,
/// the rest follow in ascending DB ID (`PowerTypeTable::class_powers`).
pub(super) fn reconcile_display_power(world: &mut World, unit: Entity) {
    let Some(class) = world.get::<Player>(unit).map(|player| player.class) else {
        return;
    };
    let form = world
        .get::<ActiveAuras>(unit)
        .and_then(current_form)
        .map(|(_, form)| form);
    let aura_display = world.get::<ActiveAuras>(unit).and_then(|auras| {
        auras
            .iter()
            .flat_map(|aura| &aura.effects)
            .filter(|effect| effect.aura_type == super::handlers::MOD_POWER_DISPLAY)
            .find_map(|effect| PowerType::from_db(effect.misc_values[0]))
    });
    let class_display = aura_display.or_else(|| {
        world
            .get_resource::<PowerTypeTable>()
            .and_then(|table| table.display_power(class))
    });
    let Some(display) = display_power(form, class_display) else {
        return;
    };
    let Some(mut pools) = world.get_mut::<PowerPools>(unit) else {
        return;
    };
    if !pools.order.contains(&display) || pools.order.first() == Some(&display) {
        return;
    }
    pools
        .order
        .sort_by_key(|&power| (power != display, power as i32));
}
