//! MOD_SHAPESHIFT 36 (`AuraEffect::HandleAuraModShapeshift`): one form at a time, whose
//! `SpellShapeshiftForm` row gives the model (`Unit::GetModelForForm`; the display shows
//! through `unit_state`'s transform/form/native priority) and whose form picks the display
//! power (`Unit::CalculateDisplayPowerType`), the first entry of `UnitPowers`.

use std::collections::HashMap;
use std::path::Path;

use bevy::prelude::*;
use rusqlite::Connection;
use shared::components::{Npc, Player, PowerType};

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
    combat_round_time_ms: u32,
    preset_spells: [u32; 8],
}

/// Separate the authored swing clock from TC's AP-multiplier selection.
#[derive(Debug, Clone, Copy)]
pub struct FormAttackTiming {
    pub seconds: f32,
    pub use_form_ap_speed: bool,
}

/// TC Player::InitDataForForm restores the ranged base clock while shifted.
pub(crate) const FORM_RANGED_ATTACK_SECS: f32 = 2.0;

/// Authored display and cast-gating flags by form ID.
#[derive(Resource, Debug, Default)]
pub struct ShapeshiftForms(HashMap<i32, ShapeshiftForm>);

impl ShapeshiftForms {
    pub(crate) fn attack_timing(&self, form: i32, is_player: bool) -> Option<FormAttackTiming> {
        let ms = self.0.get(&form)?.combat_round_time_ms;
        (ms != 0).then_some(FormAttackTiming {
            seconds: ms as f32 / 1000.0,
            // TC Unit::GetAPMultiplier11100 and IsInFeralForm9557.
            use_form_ap_speed: !is_player
                || matches!(
                    form,
                    FORM_CAT | FORM_BEAR | FORM_DIRE_BEAR | FORM_GHOST_WOLF
                ),
        })
    }

    /// `GetModelForForm` fallback: the form's own display (0 keeps the current model).
    /// The race/gender `ChrCustomizationDisplayInfo` choice is not modelled.
    pub fn display_id(&self, form: i32) -> Option<u32> {
        self.0
            .get(&form)
            .map(|row| row.display_id)
            .filter(|&display| display != 0)
    }

    pub fn creature_type(&self, form: i32) -> Option<u32> {
        self.0.get(&form).map(|row| row.creature_type)
    }

    pub fn flags(&self, form: i32) -> Option<u32> {
        self.0.get(&form).map(|row| row.flags)
    }
}

pub fn load_shapeshift_forms(conn: &Connection) -> rusqlite::Result<ShapeshiftForms> {
    let mut stmt = conn
        .prepare("SELECT ID, CreatureDisplayID, Flags, CreatureType, CombatRoundTime, PresetSpellID_0, PresetSpellID_1, PresetSpellID_2, PresetSpellID_3, PresetSpellID_4, PresetSpellID_5, PresetSpellID_6, PresetSpellID_7 FROM spell_shapeshift_form")?;
    let forms = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)? as i32,
                ShapeshiftForm {
                    display_id: r.get::<_, i64>(1)? as u32,
                    flags: r.get::<_, i64>(2)? as u32,
                    creature_type: r.get(3)?,
                    combat_round_time_ms: r.get(4)?,
                    preset_spells: [
                        r.get(5)?,
                        r.get(6)?,
                        r.get(7)?,
                        r.get(8)?,
                        r.get(9)?,
                        r.get(10)?,
                        r.get(11)?,
                        r.get(12)?,
                    ],
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

/// TC SpellAuraEffects.cpp:2086–2095 grants these actions only while a Player is shifted.
pub(crate) fn grants_preset_spell(world: &World, unit: Entity, spell: u32) -> bool {
    if spell == 0 || world.get::<Player>(unit).is_none() {
        return false;
    }
    let Some((_, form)) = world.get::<ActiveAuras>(unit).and_then(current_form) else {
        return false;
    };
    world
        .resource::<ShapeshiftForms>()
        .0
        .get(&form)
        .is_some_and(|row| row.preset_spells.contains(&spell))
}

pub fn attack_timing(world: &World, unit: Entity) -> Option<FormAttackTiming> {
    let (_, form) = current_form(world.get::<ActiveAuras>(unit)?)?;
    world.get_resource::<ShapeshiftForms>()?.attack_timing(
        form,
        crate::followers::is_player_like_combatant(world, unit),
    )
}

/// "remove other shapeshift before applying a new one":
/// `RemoveAurasByType(SPELL_AURA_MOD_SHAPESHIFT, ObjectGuid::Empty, GetBase())`.
pub(super) fn remove_other_forms(world: &mut World, unit: Entity, keep_instance: u32) {
    super::remove_auras(world, unit, RemoveReason::Cancelled, |aura| {
        aura.instance_id != keep_instance && aura_form(aura).is_some()
    });
}

/// TC a352b1fa SpellAuraEffects.cpp:1508–1574, including glyph-dependent boosts.
fn form_boosts(world: &World, unit: Entity, form: Option<i32>) -> Vec<u32> {
    let spells: &[u32] = match form {
        Some(FORM_CAT) => &[3025, 48629, 106840, 113636],
        Some(2) => &[5420, 81097],
        Some(FORM_TRAVEL) => &[5419],
        Some(4) => &[5421],
        Some(FORM_BEAR | FORM_DIRE_BEAR) => &[1178, 21178, 106829, 106899],
        Some(27) => &[40122, 40121],
        Some(29) => &[33948, 34764],
        Some(32) => &[27792, 27795, 62371],
        _ => &[],
    };
    let mut boosts = spells.to_vec();
    if form == Some(28) {
        boosts.push(if super::has_aura(world, unit, 107906) {
            107904
        } else if super::has_aura(world, unit, 126745) {
            142024
        } else {
            107903
        });
    } else if form == Some(FORM_GHOST_WOLF) && super::has_aura(world, unit, 58135) {
        boosts.push(160942);
    }
    // SimC76f8118 sc_druid.cpp:3035–3044 grants initial Bear Rage on entry.
    if matches!(form, Some(FORM_BEAR | FORM_DIRE_BEAR)) {
        boosts.push(17057);
    }
    boosts
}

pub(super) fn apply_form_boosts(world: &mut World, unit: Entity, aura: &AuraInstance) {
    for spell in form_boosts(world, unit, aura_form(aura)) {
        if !crate::spell_cast::try_cast_triggered(world, unit, spell, Some(unit), 1) {
            warn!(?unit, spell, "Native form boost refused");
        }
    }
}

pub(super) fn remove_form_boosts(world: &mut World, unit: Entity, aura: &AuraInstance) {
    if matches!(aura_form(aura), Some(FORM_BEAR | FORM_DIRE_BEAR)) {
        // SimC76f8118 sc_druid.cpp3052–3062 loses all Rage when Bear Form ends.
        if let Some((rage, _)) =
            crate::spell_cast::powers::power_value(world, unit, PowerType::Rage)
        {
            crate::spell_cast::powers::change_power(world, unit, PowerType::Rage, -rage);
        }
    }
    let boosts = form_boosts(world, unit, aura_form(aura));
    super::remove_auras(world, unit, RemoveReason::Cancelled, |boost| {
        boost.caster == unit && boosts.contains(&boost.spell_id)
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

fn read_unit_class_power(world: &World, unit: Entity) -> Option<PowerType> {
    let class = if let Some(player) = world.get::<Player>(unit) {
        player.class
    } else if let Some(follower) = world.get::<crate::followers::Follower>(unit) {
        follower.profile.class_id
    } else {
        let npc = world.get::<Npc>(unit)?;
        world
            .get_resource::<crate::world_data::WorldData>()?
            .templates
            .get(&npc.template_id)?
            .unit_class
    };
    world.get_resource::<PowerTypeTable>()?.display_power(class)
}

/// TC SpellAuraEffects2049–2056 updates display power for players and NPCs.
/// The selected native power leads `order`; existing pool values are untouched.
pub(super) fn reconcile_display_power(world: &mut World, unit: Entity) {
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
    let class_display = aura_display.or_else(|| read_unit_class_power(world, unit));
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
