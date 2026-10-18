//! TC a352b1fa EffectJumpCharge / ObjectMgr::LoadJumpChargeParams.
//! Only the source options implemented below may launch; never teleport on failure.
use std::collections::HashMap;

use bevy::prelude::*;
use shared::components::Position;

use super::effects::EffectCtx;
use crate::scripted_ai::jump_movement::{JumpOptions, move_jump_with_options};
use crate::spell_info::SpellEffect;

const EVENT_JUMP: u32 = 1004;
const SPECIAL_FACING_ATTRIBUTES: u32 = 0x0000_1000 | 0x0080_0000;

#[derive(Clone, Copy)]
pub(crate) struct JumpChargeParam {
    speed: f32,
    time_mode: bool,
    unlimited_speed: bool,
    min_height: Option<f32>,
    max_height: Option<f32>,
    visual: Option<u32>,
    progress_curve: Option<u32>,
    parabolic_curve: Option<u32>,
    trigger: Option<u32>,
}

#[derive(Clone, Default)]
pub(crate) struct JumpChargeParams(pub(crate) HashMap<u32, JumpChargeParam>);

pub(crate) fn load_params(conn: &rusqlite::Connection) -> rusqlite::Result<JumpChargeParams> {
    let mut statement = conn.prepare(
        "SELECT id,speed,treatSpeedAsMoveTimeSeconds,unlimitedSpeed,minHeight,maxHeight,\
         spellVisualId,progressCurveId,parabolicCurveId,triggerSpellId FROM tdb_jump_charge_params",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get(0)?,
            JumpChargeParam {
                speed: row.get(1)?,
                time_mode: row.get(2)?,
                unlimited_speed: row.get(3)?,
                min_height: row.get(4)?,
                max_height: row.get(5)?,
                visual: row.get(6)?,
                progress_curve: row.get(7)?,
                parabolic_curve: row.get(8)?,
                trigger: row.get(9)?,
            },
        ))
    })?;
    Ok(JumpChargeParams(rows.collect::<rusqlite::Result<_>>()?))
}

impl JumpChargeParam {
    fn motion_options(self) -> Result<JumpOptions, &'static str> {
        let unsupported_extra = self.visual.is_some()
            || self.progress_curve.is_some()
            || self.parabolic_curve.is_some()
            || self.trigger.is_some();
        if unsupported_extra {
            return Err("unsupported jump visual, curve or arrival-spell options");
        }
        // TC MotionMaster.cpp:883 value_or defaults, not synthesized database values.
        let min_height = self.min_height.unwrap_or(0.5);
        let max_height = self.max_height.unwrap_or(1000.0);
        let valid_heights = min_height.is_finite()
            && max_height.is_finite()
            && min_height >= 0.0
            && max_height >= min_height;
        if !valid_heights || !self.speed.is_finite() || self.speed <= 0.0 {
            return Err("invalid jump speed/time or height bounds");
        }
        Ok(JumpOptions {
            speed: self.speed,
            time_mode: self.time_mode,
            unlimited_speed: self.unlimited_speed,
            min_height,
            max_height,
        })
    }
}

fn prepare_jump(
    world: &World,
    ctx: &EffectCtx,
    effect: &SpellEffect,
) -> Result<(Position, JumpOptions), &'static str> {
    let misc = ctx.spell.misc.as_ref().ok_or("missing SpellMisc")?;
    if misc.attributes[9] & SPECIAL_FACING_ATTRIBUTES != 0 || effect.trigger_spell != 0 {
        return Err("unsupported jump facing or effect arrival-spell options");
    }
    let parameters = &world
        .get_resource::<crate::world_data::WorldData>()
        .ok_or("missing world data for jump-charge catalog")?
        .jump_charge_params;
    let parameter = parameters
        .0
        .get(&(effect.misc_values[0] as u32))
        .ok_or("missing referenced jump-charge parameters")?;
    let options = parameter.motion_options()?;
    let point = ctx.dest.ok_or("missing exact jump destination")?.position;
    if !point.is_finite() {
        return Err("non-finite jump destination");
    }
    Ok((
        Position {
            x: point.x,
            y: point.y,
            z: point.z,
        },
        options,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jump_charge_actual_visual_and_arrival_options_are_explicitly_unsupported() {
        let db = rusqlite::Connection::open_with_flags(
            crate::test_data::world_db_path(),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let parameters = load_params(&db).unwrap();
        // Actual TDB17 has visual50217 and trigger52174; neither is silently dropped.
        assert!(matches!(
            parameters.0.get(&17).unwrap().motion_options(),
            Err("unsupported jump visual, curve or arrival-spell options")
        ));
    }

    #[test]
    fn jump_charge_missing_native_parameters_does_not_teleport() {
        let mut h = super::super::test_harness::harness(Vec::new(), &[387981], 12).unwrap();
        let mut data = crate::test_data::world_data().clone();
        data.jump_charge_params.0.remove(&713);
        h.app.insert_resource(data);
        let before = *h.app.world().get::<Position>(h.caster).unwrap();
        let destination = Vec3::new(-5336.8003, 344.32678, -1066.6493);
        let spell = h
            .app
            .world()
            .resource::<crate::spell_info::SpellStore>()
            .get(387981)
            .unwrap()
            .clone();
        let context = EffectCtx {
            caster: h.caster,
            spell: &spell,
            depth: 1,
            dest: Some(super::super::targets::Dest {
                position: destination,
                orientation: 0.0,
            }),
            chain_multiplier: 1.0,
        };
        assert!(matches!(
            prepare_jump(h.app.world(), &context, &spell.effects[0]),
            Err("missing referenced jump-charge parameters")
        ));
        super::super::cast_triggered_at_destination(
            h.app.world_mut(),
            h.caster,
            387981,
            destination,
            1,
        );
        h.advance(20);
        assert_eq!(*h.app.world().get::<Position>(h.caster).unwrap(), before);
    }
}

pub(super) fn jump_to_destination(world: &mut World, ctx: &EffectCtx, effect: &SpellEffect) {
    match prepare_jump(world, ctx, effect) {
        Ok((destination, options)) => {
            move_jump_with_options(world, ctx.caster, EVENT_JUMP, destination, options);
        }
        Err(reason) => error!("Spell {} jump-charge refused: {reason}", ctx.spell.id),
    }
}
