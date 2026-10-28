//! Healer-attributed receipts from a native SimC timeline replay.
use bevy::prelude::{Entity, World};
use serde_json::{Value, json};
use shared::components::Mana;
use shared::protocol::CombatLogKind;
use std::collections::BTreeMap;

pub(super) struct Recorder {
    healer: Entity,
    members: Vec<Entity>,
    event_cursor: usize,
    spells: BTreeMap<u32, (u64, u64)>,
    mana_curve: Vec<Value>,
    first_death_seconds: Option<f64>,
}

impl Recorder {
    pub(super) fn start(world: &World, healer: Entity, members: Vec<Entity>) -> Self {
        let mut recorder = Self {
            healer,
            members,
            event_cursor: world
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0
                .len(),
            spells: BTreeMap::new(),
            mana_curve: Vec::new(),
            first_death_seconds: None,
        };
        recorder.sample_mana(world, 0.0);
        recorder
    }

    fn sample_mana(&mut self, world: &World, time_seconds: f64) {
        if let Some(mana) = world.get::<Mana>(self.healer) {
            self.mana_curve.push(
                json!({"time_seconds": time_seconds, "current": mana.current, "max": mana.max}),
            );
        }
    }

    pub(super) fn observe_frame(&mut self, world: &World, tick: u32) {
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
            if event.kind == CombatLogKind::Death {
                self.first_death_seconds.get_or_insert(seconds);
            } else if event.kind == CombatLogKind::Heal
                && event.source == Some(self.healer.to_bits())
            {
                let raw = event.amount.max(0) as u64;
                let effective = raw - (event.overflow.max(0) as u64).min(raw);
                let spell = self.spells.entry(event.spell_id.unwrap_or(0)).or_default();
                spell.0 += raw;
                spell.1 += effective;
            }
        }
        self.event_cursor = events.len();
    }

    pub(super) fn report(&self, duration_seconds: u32) -> Value {
        let (raw, effective) = self.spells.values().fold(
            (0_u64, 0_u64),
            |(raw, effective), &(spell_raw, spell_effective)| {
                (raw + spell_raw, effective + spell_effective)
            },
        );
        let duration = f64::from(duration_seconds);
        let spells: BTreeMap<_, _> = self.spells.iter().map(|(&spell, &(raw, effective))| (spell, json!({"raw":raw,"effective":effective,"raw_hps":raw as f64 / duration,"effective_hps":effective as f64 / duration}))).collect();
        json!({"raw_hps":raw as f64 / duration,"effective_hps":effective as f64 / duration,
            "overheal_pct":(raw > 0).then(|| 100.0 * (raw - effective) as f64 / raw as f64),
            "spell_healing":spells,"mana_curve":self.mana_curve,"first_death_seconds":self.first_death_seconds})
    }
}
