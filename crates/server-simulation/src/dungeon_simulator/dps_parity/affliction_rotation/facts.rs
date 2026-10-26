//! Affliction-only operands read actual owned native state.
use super::*;
use crate::simc_apl_native::NativeFacts;
use server::simc_apl::{Action, ActionFacts, Facts};

pub(super) struct AfflictionFacts<'a> {
    native: NativeFacts<'a>,
    equipment: &'a Equipment,
    darkglare_remains: f64,
    previous_action: Option<&'a str>,
}

impl<'a> AfflictionFacts<'a> {
    pub fn read(
        world: &'a World,
        actor: Entity,
        targets: &'a [Entity],
        bindings: &'a Bindings,
        equipment: &'a Equipment,
        encounter: Encounter,
        previous_action: Option<&'a str>,
    ) -> Self {
        let now = crate::spell_cast::now(world);
        // SimC76f8118 player.cpp12462–12469: remains reads the actual pet expiration.
        let darkglare_remains = world
            .iter_entities()
            .filter_map(|entity| {
                let summon = entity.get::<crate::spell_cast::summon::SummonedCreature>()?;
                let owned_darkglare = summon.owner == actor && summon.spell_id == 205180;
                if !owned_darkglare || !crate::spell_cast::targets::is_alive(world, entity.id()) {
                    return None;
                }
                summon.expires_at.map(|expires| (expires - now).max(0.0))
            })
            .fold(0.0, f64::max);
        Self {
            native: NativeFacts::new(world, actor, bindings, encounter, targets.len())
                .with_targets(targets.first().copied(), targets),
            equipment,
            darkglare_remains,
            previous_action,
        }
    }
}

impl Facts for AfflictionFacts<'_> {
    fn value(&self, name: &str) -> Result<f64, String> {
        // SimC76f8118 action.cpp3714–3733: prev compares the last foreground
        // action, including off-GCD actions, rather than only the previous GCD.
        if let Some(alias) = name.strip_prefix("prev.") {
            return Ok(f64::from(self.previous_action == Some(alias)));
        }
        match name {
            "pet.darkglare.remains" => Ok(self.darkglare_remains),
            "pet.darkglare.active" => Ok(f64::from(self.darkglare_remains > 0.0)),
            _ if name.starts_with("trinket.") => self.equipment.fact(name, &self.native),
            _ => self.native.value(name),
        }
    }
}

impl ActionFacts for AfflictionFacts<'_> {
    fn target_ready(&self, action: &Action) -> Result<bool, String> {
        // SimC76f8118 sc_warlock_actions.cpp2138–2143: real Darkglare enables Grasp.
        match action.name.as_str() {
            "malefic_grasp" => Ok(self.darkglare_remains > 0.0),
            "corruption" => Ok(self.native.value("talent.wither")? == 0.0),
            _ => Ok(true),
        }
    }
    fn target_ids(&self, action: &Action) -> Result<Vec<u64>, String> {
        self.native.target_ids(action)
    }
    fn value_for(&self, action: &Action, name: &str) -> Result<f64, String> {
        // SimC76f8118 dot.cpp448–485: these operands read the candidate's own dot.
        if matches!(name, "remains" | "duration" | "refreshable") {
            return self
                .native
                .value_for(action, &format!("dot.{}.{name}", action.name));
        }
        if name.starts_with("pet.darkglare.")
            || name.starts_with("trinket.")
            || name.starts_with("prev.")
        {
            self.value(name)
        } else {
            self.native.value_for(action, name)
        }
    }
}
