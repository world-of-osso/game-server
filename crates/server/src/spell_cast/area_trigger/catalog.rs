//! Source-defined aura395 spheres and the finite Frozen Ground effect179 binding. Unsupported scripts, shapes and curve modes
//! remain explicit errors; they never acquire a guessed radius or curve type.
use std::collections::HashMap;

use bevy::prelude::Resource;
use rusqlite::Connection;

#[derive(Resource)]
#[cfg_attr(test, derive(Clone))]
pub(crate) struct AuraFieldCatalog(HashMap<u32, Result<FieldDefinition, String>>);

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum FieldScript {
    VaultRune,
}

#[cfg_attr(test, derive(Clone))]
pub(super) struct FieldDefinition {
    pub trigger_spell: u32,
    pub relation: super::targets::Relation,
    pub script: Option<FieldScript>,
    radius: f32,
    time_to_scale_ms: u32,
    scale_curve: Option<Vec<(f32, f32)>>,
}

struct SphereProperties {
    template: u32,
    custom: bool,
    radius: f32,
    curve: u32,
    time_to_scale_ms: u32,
    script_name: String,
    flags: u32,
}

impl AuraFieldCatalog {
    pub(super) fn definition(&self, properties: u32) -> Result<&FieldDefinition, String> {
        self.0
            .get(&properties)
            .ok_or_else(|| format!("Missing aura395 create-properties {properties}"))?
            .as_ref()
            .map_err(Clone::clone)
    }
}

impl FieldDefinition {
    pub(super) fn radius_at(&self, elapsed_seconds: f64) -> f32 {
        let Some(points) = &self.scale_curve else {
            // Native ScaleCurveId0 means no scale multiplier.
            return self.radius;
        };
        let progress = if self.time_to_scale_ms == 0 {
            0.0
        } else {
            let elapsed_ms = (elapsed_seconds.max(0.0) * 1000.0) as f32;
            (elapsed_ms / self.time_to_scale_ms as f32).clamp(0.0, 1.0)
        };
        // TC AreaTrigger::CalcCurrentScale clamps its source curve multiplier.
        self.radius * clamped_linear(points, progress).max(0.000001)
    }
}

pub(crate) fn load_catalog(connection: &Connection) -> rusqlite::Result<AuraFieldCatalog> {
    let mut statement = connection.prepare(
        "SELECT DISTINCT EffectMiscValue_0 FROM spell_effect \
         WHERE (EffectAura=395 OR (Effect=179 AND SpellID=387149 AND EffectMiscValue_0=26505)) \
         AND EffectMiscValue_0>0 ORDER BY EffectMiscValue_0",
    )?;
    let properties = statement
        .query_map([], |row| row.get::<_, u32>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let definitions = properties
        .into_iter()
        .map(|id| (id, load_definition(connection, id)))
        .collect();
    Ok(AuraFieldCatalog(definitions))
}

fn load_definition(connection: &Connection, id: u32) -> Result<FieldDefinition, String> {
    let properties = load_sphere_properties(connection, id)?;
    if !properties.radius.is_finite() || properties.radius <= 0.0 {
        return Err(format!(
            "Aura395 properties {id}: invalid source radius {}",
            properties.radius
        ));
    }
    let script = select_script(id, &properties)?;
    let (trigger_spell, relation) = load_action(connection, &properties, script)?;
    let scale_curve = if properties.curve == 0 {
        None
    } else {
        Some(load_curve(connection, properties.curve)?)
    };
    Ok(FieldDefinition {
        trigger_spell,
        relation,
        script,
        radius: properties.radius,
        time_to_scale_ms: properties.time_to_scale_ms,
        scale_curve,
    })
}

fn load_sphere_properties(connection: &Connection, id: u32) -> Result<SphereProperties, String> {
    connection
        .query_row(
            "SELECT AreaTriggerId,IsAreatriggerCustom,ShapeData0,ScaleCurveId,TimeToTargetScale,ScriptName,Flags \
         FROM tdb_areatrigger_create_properties \
         WHERE Id=?1 AND IsCustom=0 AND Shape=0 AND ShapeData0=ShapeData1 \
         AND MoveCurveId=0 AND MorphCurveId=0 AND FacingCurveId=0",
            [id],
            |row| {
                Ok(SphereProperties {
                    template: row.get(0)?,
                    custom: row.get(1)?,
                    radius: row.get(2)?,
                    curve: row.get(3)?,
                    time_to_scale_ms: row.get(4)?,
                    script_name: row.get(5)?,
                    flags: row.get(6)?,
                })
            },
        )
        .map_err(|error| {
            format!(
                "Aura395 properties {id}: missing or unsupported fixed sphere: {error}"
            )
        })
}

fn select_script(id: u32, properties: &SphereProperties) -> Result<Option<FieldScript>, String> {
    if properties.script_name.is_empty() {
        return Ok(None);
    }
    let native_identity = id == super::vault_rune::CREATE_PROPERTIES
        && properties.template == super::vault_rune::TEMPLATE
        && properties.script_name == super::vault_rune::SCRIPT_NAME;
    if !native_identity {
        return Err(format!(
            "Aura395 properties {id}: unsupported script {:?}",
            properties.script_name
        ));
    }
    let native_mode = !properties.custom
        && properties.flags == 4
        && properties.curve == 0
        && properties.time_to_scale_ms == 0;
    if !native_mode {
        return Err(format!(
            "Aura395 properties {id}: unsupported Rune flags/custom/scale mode"
        ));
    }
    Ok(Some(FieldScript::VaultRune))
}

fn load_action(
    connection: &Connection,
    properties: &SphereProperties,
    script: Option<FieldScript>,
) -> Result<(u32, super::targets::Relation), String> {
    use super::targets::Relation;
    validate_template(connection, properties.template, properties.custom)?;
    let actions = load_actions(connection, properties.template, properties.custom)?;
    match (script, actions.as_slice()) {
        (None, [(0, spell, 2)]) if *spell != 0 => Ok((*spell, Relation::Enemy)),
        (Some(FieldScript::VaultRune), [(0, super::vault_rune::PROTECTION, 1)]) => {
            Ok((super::vault_rune::PROTECTION, Relation::Ally))
        }
        _ => Err(format!(
            "Aura395 template {}/{}: unsupported actions {actions:?}",
            properties.template, properties.custom
        )),
    }
}

fn validate_template(connection: &Connection, template: u32, custom: bool) -> Result<(), String> {
    connection.query_row(
        "SELECT 1 FROM tdb_areatrigger_template WHERE Id=?1 AND IsCustom=?2 \
         AND Flags=0 AND ActionSetId=0 AND ActionSetFlags=0",
        (template, custom), |_| Ok(()),
    ).map_err(|error| format!("Aura395 template {template}/{custom}: missing or unsupported flags/action set: {error}"))
}

fn load_actions(
    connection: &Connection,
    template: u32,
    custom: bool,
) -> Result<Vec<(u32, u32, u32)>, String> {
    let mut statement = connection
        .prepare(
            "SELECT ActionType,ActionParam,TargetType FROM tdb_areatrigger_template_actions \
         WHERE AreaTriggerId=?1 AND IsCustom=?2",
        )
        .map_err(|error| error.to_string())?;
    let actions = statement
        .query_map((template, custom), |row| {
            Ok((
                row.get::<_, u32>(0)?,
                row.get::<_, u32>(1)?,
                row.get::<_, u32>(2)?,
            ))
        })
        .map_err(|error| error.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())?;
    Ok(actions)
}

fn load_curve(connection: &Connection, id: u32) -> Result<Vec<(f32, f32)>, String> {
    let (kind, flags) = connection
        .query_row("SELECT Type,Flags FROM curve WHERE ID=?1", [id], |row| {
            Ok((row.get::<_, u32>(0)?, row.get::<_, u32>(1)?))
        })
        .map_err(|error| format!("Missing native Curve {id}: {error}"))?;
    if kind != 0 || flags != 0 {
        return Err(format!("Curve {id}: unsupported Type{kind}/Flags{flags}"));
    }
    let points = load_curve_points(connection, id)?;
    validate_curve_points(id, &points)?;
    Ok(points)
}

fn load_curve_points(connection: &Connection, id: u32) -> Result<Vec<(f32, f32)>, String> {
    let mut statement = connection
        .prepare("SELECT Pos_0,Pos_1 FROM curve_point WHERE CurveID=?1 ORDER BY OrderIndex,ID")
        .map_err(|error| error.to_string())?;
    let points = statement
        .query_map([id], |row| {
            Ok((row.get::<_, f32>(0)?, row.get::<_, f32>(1)?))
        })
        .map_err(|error| error.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())?;
    Ok(points)
}

fn validate_curve_points(id: u32, points: &[(f32, f32)]) -> Result<(), String> {
    if points.is_empty() {
        return Err(format!("Curve {id}: no readable points"));
    }
    if points.iter().any(|(x, y)| !x.is_finite() || !y.is_finite()) {
        return Err(format!("Curve {id}: nonfinite point"));
    }
    if points.windows(2).any(|pair| pair[0].0 >= pair[1].0) {
        return Err(format!(
            "Curve {id}: unsupported non-increasing point order"
        ));
    }
    Ok(())
}

fn clamped_linear(points: &[(f32, f32)], x: f32) -> f32 {
    let first = points.first().expect("validated nonempty Curve points");
    let last = points.last().unwrap();
    if x <= first.0 {
        return first.1;
    }
    if x >= last.0 {
        return last.1;
    }
    let right = points.partition_point(|point| point.0 <= x);
    let (x0, y0) = points[right - 1];
    let (x1, y1) = points[right];
    y0 + (x - x0) / (x1 - x0) * (y1 - y0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aura395_native_curve_33927_interpolates_and_clamps_without_missing_id_defaults() {
        let connection = Connection::open_with_flags(
            crate::test_data::world_db_path(),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let points = load_curve(&connection, 33927).unwrap();
        assert_eq!(clamped_linear(&points, -1.0), 0.25);
        assert!((clamped_linear(&points, 0.02) - 1.0).abs() < 0.000001);
        assert!((clamped_linear(&points, 0.01) - 0.625).abs() < 0.000001);
        assert!((clamped_linear(&points, 2.0) - 0.962566733).abs() < 0.000001);
        for missing in [121405, 121406] {
            assert!(
                load_curve(&connection, missing).is_err(),
                "unreadable Curve {missing} must fail explicitly"
            );
        }
    }
}
