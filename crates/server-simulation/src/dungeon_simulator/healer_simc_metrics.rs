//! Healer-attributed receipts from a native SimC timeline replay.
#[cfg(test)]
#[path = "healer_simc_metrics_tests.rs"]
mod tests;
use bevy::prelude::{Entity, World};
use serde_json::{Value, json};
use shared::components::{Health, Mana};
use shared::protocol::CombatLogKind;
use std::collections::BTreeMap;

pub(super) struct Recorder {
    healer: Entity,
    started_at: f64,
    tank: Entity,
    members: Vec<Entity>,
    tank_health_min: Option<f32>,
    mana_min: Option<f32>,
    event_cursor: usize,
    spells: BTreeMap<u32, (u64, u64)>,
    energizes: BTreeMap<u32, (u64, u64)>,
    initial_absorbs: BTreeMap<u32, f64>,
    party_damage: BTreeMap<u64, u64>,
    party_healing: BTreeMap<u64, (u64, u64)>,
    mana_curve: Vec<Value>,
    first_death_seconds: Option<f64>,
    tank_damage: u64,
    tank_health_curve: Vec<Value>,
    tank_minimum_pct: f32,
    tank_below_35_ticks: u32,
    idle_gcd_ticks: u32,
}

impl Recorder {
    pub(super) fn start(world: &World, healer: Entity, tank: Entity, members: Vec<Entity>) -> Self {
        let mut recorder = Self {
            healer,
            started_at: crate::spell_cast::now(world),
            tank,
            members,
            tank_health_min: None,
            mana_min: None,
            event_cursor: world
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0
                .len(),
            spells: BTreeMap::new(),
            energizes: BTreeMap::new(),
            initial_absorbs: healer_absorbs(world, healer),
            party_damage: BTreeMap::new(),
            party_healing: BTreeMap::new(),
            mana_curve: Vec::new(),
            first_death_seconds: None,
            tank_damage: 0,
            tank_health_curve: Vec::new(),
            tank_minimum_pct: 100.0,
            tank_below_35_ticks: 0,
            idle_gcd_ticks: 0,
        };
        recorder.sample_mana(world, 0.0);
        recorder.observe_tank_health(world, 0);
        recorder.sample_minima(world);
        recorder
    }

    fn sample_mana(&mut self, world: &World, time_seconds: f64) {
        if let Some(mana) = world.get::<Mana>(self.healer) {
            self.mana_curve.push(
                json!({"time_seconds": time_seconds, "current": mana.current, "max": mana.max}),
            );
        }
    }

    fn observe_tank_health(&mut self, world: &World, tick: u32) {
        let Some(health) = world.get::<Health>(self.tank) else {
            return;
        };
        let pct = if health.max > 0.0 {
            100.0 * health.current.max(0.0) / health.max
        } else {
            0.0
        };
        self.tank_minimum_pct = self.tank_minimum_pct.min(pct);
        if tick > 0 && health.current > 0.0 && pct < 35.0 {
            self.tank_below_35_ticks += 1;
        }
        if tick % 20 == 0 {
            self.tank_health_curve.push(json!({"time_seconds":f64::from(tick) / 20.0,"current":health.current,"max":health.max,"pct":pct}));
        }
    }

    pub(super) fn sample_minima(&mut self, world: &World) {
        if let Some(health) = world.get::<Health>(self.tank) {
            self.tank_health_min = Some(
                self.tank_health_min
                    .map_or(health.current, |old| old.min(health.current)),
            );
        }
        if let Some(mana) = world.get::<Mana>(self.healer) {
            self.mana_min = Some(
                self.mana_min
                    .map_or(mana.current, |old| old.min(mana.current)),
            );
        }
    }

    pub(super) fn observe_idle_gcd(&mut self, world: &World) {
        let alive = world
            .get::<Health>(self.healer)
            .is_some_and(|health| health.current > 0.0);
        let casting = world
            .get::<shared::casting::CastState>(self.healer)
            .is_some();
        let gcd_active = world
            .get::<crate::spell_cast::SpellHistory>(self.healer)
            .is_some_and(|history| history.has_active_gcd(crate::spell_cast::history_now(world)));
        if alive && !casting && !gcd_active {
            self.idle_gcd_ticks += 1;
        }
    }

    pub(super) fn observe_frame(&mut self, world: &World, tick: u32) {
        self.observe_tank_health(world, tick);
        self.sample_minima(world);
        let seconds = f64::from(tick) / 20.0;
        if tick % 20 == 0 {
            self.sample_mana(world, seconds);
        }
        let events = &world
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        for event in &events[self.event_cursor..] {
            let target = event.target.and_then(Entity::try_from_bits);
            if !target.is_some_and(|unit| self.members.contains(&unit)) {
                continue;
            }
            if event.kind == CombatLogKind::Energize && target == Some(self.healer) {
                let raw = event.amount.max(0) as u64;
                let effective = raw - (event.overflow.max(0) as u64).min(raw);
                let entry = self
                    .energizes
                    .entry(event.spell_id.unwrap_or(0))
                    .or_default();
                entry.0 += raw;
                entry.1 += effective;
            }
            if event.kind == CombatLogKind::Death {
                self.first_death_seconds.get_or_insert(seconds);
            } else if event.kind == CombatLogKind::Damage {
                let damage = (event.amount - event.overflow.max(0)).max(0) as u64;
                if let Some(target) = target {
                    *self.party_damage.entry(target.to_bits()).or_default() += damage;
                    if target == self.tank {
                        self.tank_damage += damage;
                    }
                }
            } else if event.kind == CombatLogKind::Heal
                && event
                    .source
                    .and_then(Entity::try_from_bits)
                    .is_some_and(|source| {
                        source == self.healer
                            || world
                                .get::<crate::spell_cast::summon::SummonedCreature>(source)
                                .is_some_and(|summon| summon.owner == self.healer)
                    })
            {
                let raw = event.amount.max(0) as u64;
                let effective = raw - (event.overflow.max(0) as u64).min(raw);
                let spell = self.spells.entry(event.spell_id.unwrap_or(0)).or_default();
                spell.0 += raw;
                spell.1 += effective;
                let recipient = self
                    .party_healing
                    .entry(target.expect("party heal target").to_bits())
                    .or_default();
                recipient.0 += raw;
                recipient.1 += effective;
            }
        }
        self.event_cursor = events.len();
    }

    pub(super) fn report(&self, world: &World, duration_seconds: u32) -> Value {
        let (raw, effective) = self.spells.values().fold(
            (0_u64, 0_u64),
            |(raw, effective), &(spell_raw, spell_effective)| {
                (raw + spell_raw, effective + spell_effective)
            },
        );
        let duration = f64::from(duration_seconds);
        let spells: BTreeMap<_, _> = self.spells.iter().map(|(&spell, &(raw, effective))| (spell, json!({"raw":raw,"effective":effective,"raw_hps":raw as f64 / duration,"effective_hps":effective as f64 / duration}))).collect();
        let absorbs: BTreeMap<_, _> = healer_absorbs(world, self.healer)
            .into_iter()
            .filter_map(|(spell, amount)| {
                let consumed = amount - self.initial_absorbs.get(&spell).copied().unwrap_or(0.0);
                (consumed > 0.0).then_some((spell, consumed))
            })
            .collect();
        let fiend = world
            .get_resource::<server_spell_scripts::test_support::FiendAtonementReceipts>()
            .and_then(|receipts| receipts.0.get(&self.healer))
            .copied()
            .unwrap_or_default();
        let fiend_summons: Vec<_> = world.get_resource::<server_spell_scripts::test_support::FiendAtonementReceipts>().into_iter().flat_map(|receipts| &receipts.1).filter(|receipt| receipt.owner == self.healer).map(|receipt| json!({"spell": receipt.spell, "spawn_seconds": receipt.started - self.started_at, "scheduled_lifetime_seconds": receipt.expires.map(|expires| expires - receipt.started), "owner_spell_power": receipt.owner_spell_power, "landed_pre_mitigation_hits": receipt.landed_raw})).collect();
        let fiend_rolls: Vec<_> = world.get_resource::<server_spell_scripts::test_support::FiendAtonementReceipts>().into_iter().flat_map(|receipts| &receipts.2).filter(|(owner, _, _, _)| *owner == self.healer).map(|(_, time, roll, chance)| json!({"time_seconds":time - self.started_at,"roll_pct":roll,"chance_pct":chance})).collect();
        let mut mana_spent = BTreeMap::<u32, f64>::new();
        if let Some(payments) =
            world.get::<crate::spell_cast::ObservedResourcePayments>(self.healer)
        {
            for &(spell, pool, amount) in &payments.payments {
                if pool == crate::spell_cast::CostPool::Power(shared::components::PowerType::Mana) {
                    *mana_spent.entry(spell).or_default() += f64::from(amount);
                }
            }
        }
        let energizes: BTreeMap<_, _> = self
            .energizes
            .iter()
            .map(|(&spell, &(raw, effective))| (spell, json!({"raw":raw,"effective":effective})))
            .collect();
        let mana_regenerated = world
            .get::<crate::spell_cast::ObservedResourcePayments>(self.healer)
            .map_or(0.0, |observed| observed.mana_regenerated);
        json!({"mana_spent_by_spell":mana_spent,"passive_mana_regenerated":mana_regenerated,"resource_energizes_by_spell":energizes,"fiend_rolls":fiend_rolls,"fiend_summons":fiend_summons,"fiend_pre_mitigation_damage":fiend.0,"fiend_atonement_raw":fiend.1,"fiend_atonement_effective":fiend.2,
            "raw_hps":raw as f64 / duration,"effective_hps":effective as f64 / duration,
            "consumed_absorbs_by_spell":absorbs,
            "overheal_pct":(raw > 0).then(|| 100.0 * (raw - effective) as f64 / raw as f64),
            "idle_gcd_pct":100.0 * f64::from(self.idle_gcd_ticks) / (duration * 20.0),
            "spell_healing":spells,"mana_curve":self.mana_curve,"first_death_seconds":self.first_death_seconds,
            "tank_dtps":self.tank_damage as f64 / duration,"tank_health_curve":self.tank_health_curve,
            "tank_minimum_health_pct":self.tank_minimum_pct,"tank_seconds_below_35_alive":f64::from(self.tank_below_35_ticks) / 20.0,
            "mana_min":self.mana_min,"tank_health_min":self.tank_health_min,
            "party_damage":self.party_damage,"party_healing":self.party_healing})
    }
}

fn healer_absorbs(world: &World, healer: Entity) -> BTreeMap<u32, f64> {
    world
        .get_resource::<super::healer_intake::Capture>()
        .map(|capture| {
            capture
                .consumed_absorbs_by_spell
                .iter()
                .filter_map(|(&(caster, spell), &amount)| {
                    (caster == healer).then_some((spell, amount))
                })
                .collect()
        })
        .unwrap_or_default()
}
