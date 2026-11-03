//! WCL damage receipts replayed through native recipient combat handling.
use super::healer_intake::{Hit, HitKind};
use bevy::prelude::{Entity, World};
use serde::Deserialize;
use std::path::Path;

pub(super) enum Delivery {
    Applied,
    ZeroOutcome,
    TargetDead,
}

#[derive(Deserialize)]
pub(super) struct Trace {
    pub provenance: String,
    pub hits: Vec<RecordedHit>,
}

#[derive(Deserialize)]
pub(super) struct RecordedHit {
    pub time_ms: u64,
    slot: usize,
    source_actor: i64,
    target_actor: i64,
    spell_id: u32,
    school: u32,
    defense: Option<u32>,
    hit_type: u8,
    periodic: bool,
    unmitigated: Option<f32>,
    amount: u64,
    absorbed: u64,
}

impl Trace {
    pub fn read(path: &Path) -> Result<Self, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
        let trace: Self =
            serde_json::from_slice(&bytes).map_err(|e| format!("{}: {e}", path.display()))?;
        if trace.provenance.is_empty()
            || trace.hits.is_empty()
            || trace.hits.windows(2).any(|p| p[0].time_ms > p[1].time_ms)
        {
            return Err("WCL_INTAKE_INVALID_HEADER_OR_ORDER".into());
        }
        for hit in &trace.hits {
            hit.validate()?;
        }
        Ok(trace)
    }
}

impl RecordedHit {
    fn validate(&self) -> Result<(), String> {
        if self.slot >= 5 || self.school == 0 || self.spell_id == 0 {
            return Err("WCL_INTAKE_INVALID_HIT".into());
        }
        match self.unmitigated {
            Some(raw) if raw.is_finite() && raw >= 0.0 && self.hit_type == 1 => Ok(()),
            None if self.amount == 0
                && self.absorbed == 0
                && matches!(self.hit_type, 0 | 7 | 8) =>
            {
                Ok(())
            }
            _ => Err("WCL_INTAKE_UNSUPPORTED_AMOUNT_OR_HIT_TYPE".into()),
        }
    }

    pub fn deliver(
        &self,
        world: &mut World,
        enemy: Entity,
        slots: &[Entity; 5],
        tick: u32,
    ) -> Result<Delivery, String> {
        let Some(raw) = self.unmitigated else {
            return Ok(Delivery::ZeroOutcome);
        };
        let target = slots[self.slot];
        if !world
            .get::<shared::components::Health>(target)
            .is_some_and(|health| health.current > 0.0)
        {
            return Ok(Delivery::TargetDead);
        }
        let source = if self.source_actor == self.target_actor {
            target
        } else {
            enemy
        };
        let kind = if self.spell_id == 1 {
            // TC ObjectMgr normalizes the source creatures' zero BaseAttackTime to 2 s.
            HitKind::Melee {
                hand: 0,
                outcome: "Hit".into(),
                level_difference: 0,
                critical_multiplier: 1.0,
                weapon_speed: 2.0,
            }
        } else {
            let spell = world
                .resource::<crate::spell_info::SpellStore>()
                .get(self.spell_id)
                .ok_or_else(|| format!("WCL_INTAKE_MISSING_SPELL:{}", self.spell_id))?;
            if spell.school_mask() != self.school
                || spell.categories.as_ref().map(|c| c.defense_type) != self.defense
            {
                return Err(format!(
                    "WCL_INTAKE_SPELL_METADATA_CHANGED:{}",
                    self.spell_id
                ));
            }
            HitKind::Spell {
                spell_id: self.spell_id,
                depth: 0,
                bonus_source_guid: None,
                effect_index: None,
                periodic: self.periodic,
                crit: false,
                resolved_source: true,
                blocked: false,
            }
        };
        let hit = Hit {
            tick,
            source_guid: 0,
            pack: 0,
            target_slot: self.slot,
            school: self.school,
            raw,
            level_scaling_factor: 1.0,
            kind,
        };
        match hit.kind {
            HitKind::Melee { .. } => crate::combat::replay_healer_hit(world, source, target, &hit)?,
            HitKind::Spell { .. } => {
                crate::spell_cast::effects::replay_healer_hit(world, source, target, &hit)?
            }
        }
        Ok(Delivery::Applied)
    }
}
