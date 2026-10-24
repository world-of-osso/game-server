//! Frozen native player decisions, never a parity-specific priority list.
use bevy::prelude::*;
use game_engine_core::assisted_combat::{AssistedFacts, evaluate_rule, is_spell_usable};
use game_engine_core::cooldown_assist::{CooldownAssistCatalog, Recommendation};
use serde_json::json;
use std::collections::BTreeSet;
use std::path::PathBuf;

pub(super) struct PlayerRotation {
    catalog: CooldownAssistCatalog,
    spec: u32,
    pub decisions: u64,
    started_at: f64,
}

impl PlayerRotation {
    pub fn load(spec: u32, started_at: f64) -> Result<Self, String> {
        let path = std::env::var_os("DPS_PARITY_ASSISTED_DB2")
            .map(PathBuf::from)
            .ok_or("DPS_PARITY_ASSISTED_DB2 must name the frozen70448 CSV directory")?;
        let catalog = CooldownAssistCatalog::load(&path)
            .map_err(|error| format!("load frozen AssistedCombat {}: {error}", path.display()))?;
        if catalog.rotation_spells(spec).is_empty() {
            return Err(format!(
                "native AssistedCombat spec{spec} has no authored SBA rotation"
            ));
        }
        Ok(Self {
            catalog,
            spec,
            decisions: 0,
            started_at,
        })
    }

    pub fn drive(
        &mut self,
        world: &mut World,
        actor: Entity,
        target: Entity,
    ) -> Result<(), String> {
        let needed = self.needed_spells(world, actor);
        let facts = crate::spell_cast::test_harness::read_assisted_parity_facts(
            world, actor, target, &needed,
        )?;
        self.decisions += 1;
        let selection = self.catalog.recommend(
            self.spec,
            |id| facts.known.contains(&id),
            |id| is_spell_usable(id, &facts),
            |rule| evaluate_rule(rule, &facts),
        );
        let recommendation = match selection {
            Ok(value) => value,
            Err(error) => {
                self.write_rule_gap(world, actor, target, &facts, &error)?;
                return Err(format!(
                    "NATIVE_ASSISTED_COMBAT_GAP: spec{}: {error}",
                    self.spec
                ));
            }
        };
        if let Recommendation::Spell(requested) = recommendation {
            let effective = crate::auras::overrides::resolve_spell(world, actor, requested);
            if crate::spell_cast::test_harness::read_assisted_request_window(
                world, actor, effective,
            )? {
                crate::spell_cast::cast_for_player(world, actor, effective).map_err(|error| {
                    format!(
                        "NATIVE_PLAYER_CAST_REJECTED: spec{} spell{effective}: {error:?}",
                        self.spec
                    )
                })?;
            }
        }
        Ok(())
    }

    fn needed_spells(&self, world: &World, actor: Entity) -> BTreeSet<u32> {
        let known = world.get::<crate::class_progression::KnownSpells>(actor);
        let mut needed: BTreeSet<_> = self
            .catalog
            .rotation_spells(self.spec)
            .into_iter()
            .filter(|&id| {
                let effective = crate::auras::overrides::resolve_spell(world, actor, id);
                known.is_some_and(|known| known.contains(id) || known.contains(effective))
            })
            .collect();
        // Same referenced-spell expansion as the frozen engine fact collector.
        for rule in self.catalog.rotation_rules(self.spec) {
            if needed.contains(&rule.step_spell)
                && matches!(rule.condition_type, 1 | 2 | 14 | 56 | 65 | 66)
                && rule.values[0] != 0
            {
                needed.insert(rule.values[0]);
            }
        }
        needed
    }

    fn write_rule_gap(
        &self,
        world: &World,
        actor: Entity,
        target: Entity,
        facts: &AssistedFacts,
        error: &str,
    ) -> Result<(), String> {
        let rules: Vec<_> = self.catalog.rotation_rules(self.spec).iter().map(|rule| json!({
            "id": rule.id, "step_spell": rule.step_spell, "condition_type": rule.condition_type,
            "values": rule.values, "field_11_1_7_60520_002": rule.field_11_1_7_60520_002,
        })).collect();
        let events = &world
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        let trace = json!({"spec_id": self.spec, "driver": "NATIVE_ASSISTED_COMBAT_70448",
            "engine_revision": "9ef2efe2d317b837d352fe8c5227086a4fdcd490",
            "world_time": world.resource::<Time<Virtual>>().elapsed_secs_f64(),
            "fight_time": world.resource::<Time<Virtual>>().elapsed_secs_f64() - self.started_at,
            "actor": actor.to_bits(), "target": target.to_bits(), "decisions": self.decisions,
            "error": error, "facts": facts, "rules": rules, "native_event_count": events.len(),
            "native_events_first16": events.iter().take(16).collect::<Vec<_>>()});
        let path =
            super::inputs::directory()?.join(format!("{}.player.assisted-gap.json", self.spec));
        let bytes = serde_json::to_vec_pretty(&trace)
            .map_err(|error| format!("encode player gap: {error}"))?;
        std::fs::write(&path, bytes).map_err(|error| format!("write {}: {error}", path.display()))
    }
}
