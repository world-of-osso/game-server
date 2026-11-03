//! WCL post-mitigation damage receipts, normalized to native recipient HP.
use bevy::prelude::{Entity, World};
use serde::Deserialize;
use serde_json::{Value, json};
use shared::components::Health;
use std::path::Path;

pub(super) enum Delivery {
    Applied(Value),
    Avoided,
    TargetDead,
}

#[derive(Deserialize)]
pub(super) struct Trace {
    pub provenance: String,
    pub excluded_self: usize,
    pub party_specs: [u32; 5],
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
    amount: u64,
    absorbed: u64,
    max_hp: u64,
    health_after: Option<u64>,
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
        if self.slot >= 5
            || self.school == 0
            || self.spell_id == 0
            || self.max_hp == 0
            || self.source_actor == self.target_actor
        {
            return Err("WCL_INTAKE_INVALID_HIT".into());
        }
        match (self.hit_type, self.amount + self.absorbed) {
            (1, amount) if amount > 0 => Ok(()),
            (0 | 7 | 8 | 9 | 10, 0) => Ok(()),
            _ => Err("WCL_INTAKE_UNSUPPORTED_AMOUNT_OR_HIT_TYPE".into()),
        }
    }

    pub fn deliver(
        &self,
        world: &mut World,
        enemy: Entity,
        slots: &[Entity; 5],
    ) -> Result<Delivery, String> {
        let pressure = self.amount + self.absorbed;
        let target = slots[self.slot];
        let health = world
            .get::<Health>(target)
            .ok_or("WCL_INTAKE_TARGET_HEALTH_MISSING")?;
        if health.current <= 0.0 {
            return Ok(Delivery::TargetDead);
        }
        let before = health.current;
        let native_max = health.max;
        let amount = pressure as f32 / self.max_hp as f32 * native_max;
        let spell_id = if self.spell_id == 1 {
            None
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
            Some(self.spell_id)
        };
        if pressure == 0 {
            let outcome = match self.hit_type {
                0 => shared::protocol::MissKind::Miss,
                7 => shared::protocol::MissKind::Dodge,
                8 => shared::protocol::MissKind::Parry,
                9 => shared::protocol::MissKind::Deflect,
                10 => shared::protocol::MissKind::Immune,
                _ => return Err("WCL_INTAKE_UNSUPPORTED_AVOIDANCE".into()),
            };
            crate::combat::replay_avoided_hit(
                world,
                enemy,
                target,
                spell_id,
                self.school,
                self.periodic,
                outcome,
            )?;
            return Ok(Delivery::Avoided);
        }
        let receipt = crate::combat::replay_post_mitigation_hit(
            world,
            enemy,
            target,
            spell_id,
            self.school,
            self.periodic,
            amount,
        )?;
        let after = world
            .get::<Health>(target)
            .ok_or("WCL_INTAKE_TARGET_HEALTH_MISSING")?
            .current;
        Ok(Delivery::Applied(
            json!({"time_ms":self.time_ms,"slot":self.slot,"spell_id":self.spell_id,
            "wcl_hp_pct":self.health_after.map(|hp| 100.0 * hp as f64 / self.max_hp as f64),
            "native_hp_pct":100.0 * f64::from(after / native_max),
            "wcl_loss_pct":100.0 * self.amount as f64 / self.max_hp as f64,
            "wcl_absorbed_pct":100.0 * self.absorbed as f64 / self.max_hp as f64,
            "native_loss_pct":100.0 * f64::from((before - after) / native_max),
            "native_absorbed_pct":100.0 * f64::from(receipt.absorbed_hp / native_max),
            "native_deferred_pct":100.0 * f64::from(receipt.deferred_hp / native_max),
            "native_transferred_pct":100.0 * f64::from(receipt.transferred_hp / native_max)}),
        ))
    }
}
