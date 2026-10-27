//! Devourer expressions read native fragment state and native spell data.
use super::super::literal_rotation::equipment::Equipment;
use crate::simc_apl_native::NativeFacts;
use bevy::prelude::*;
use server::simc_apl::{Action, ActionFacts, Facts};
use server::soul_fragments::{self, FragmentFilter};

pub(super) struct DevourerFacts<'a> {
    pub native: NativeFacts<'a>,
    pub equipment: &'a Equipment,
    pub world: &'a World,
    pub actor: Entity,
}

impl Facts for DevourerFacts<'_> {
    fn value(&self, name: &str) -> Result<f64, String> {
        if name == "buff.vengeful_retreat_movement.up" {
            return Ok(f64::from(
                retreat_movement_active(self.world, self.actor) as u8
            ));
        }
        if name == "active_dot.soul_immolation" {
            // SimC76f8118 sc_demon_hunter.cpp5930–5958: Soul Immolation is a self heal-over-time.
            return Ok(f64::from(
                crate::auras::has_aura(self.world, self.actor, 1241937) as u8,
            ));
        }
        if name == "soul_fragments" {
            return Ok(
                soul_fragments::active_count(self.world, self.actor, FragmentFilter::Any) as f64,
            );
        }
        if name == "action.reap.souls_consumed" {
            // SimC76f8118 sc_demon_hunter.cpp6010-6018,6068-6076.
            let base =
                crate::spell_cast::scripts::script_effect_value(self.world, self.actor, 1227619, 1);
            let extra = if self.native.value("buff.moment_of_craving.up")? != 0.0 {
                crate::spell_cast::scripts::script_effect_value(self.world, self.actor, 1238495, 0)
            } else {
                0.0
            };
            let active = soul_fragments::active_count(self.world, self.actor, FragmentFilter::Any);
            return Ok((active as f64).min(f64::from(base + extra)));
        }
        if name.starts_with("trinket.") {
            return self.equipment.fact(name, &self.native);
        }
        self.native.value(name)
    }
}

fn retreat_movement_active(world: &World, actor: Entity) -> bool {
    let Some(history) = world.get::<crate::spell_cast::SpellHistory>(actor) else {
        return false;
    };
    let Some(start) = history.last_successful_cast_at(198793) else {
        return false;
    };
    // SimC76f8118 cpp8619,9980–9981 uses the native retreat duration;
    // cpp5548–5552 cancels the movement when The Hunt executes.
    if [1246167, 1259431].into_iter().any(|spell| {
        history
            .last_successful_cast_at(spell)
            .is_some_and(|at| at > start)
    }) {
        return false;
    }
    let duration = world
        .resource::<crate::spell_info::SpellStore>()
        .get(198793)
        .and_then(|spell| spell.misc.as_ref())
        .and_then(|misc| misc.duration.as_ref())
        .expect("native Vengeful Retreat duration")
        .duration_ms;
    crate::spell_cast::now(world) < start + f64::from(duration) / 1000.0
}

impl ActionFacts for DevourerFacts<'_> {
    fn target_ids(&self, action: &Action) -> Result<Vec<u64>, String> {
        self.native.target_ids(action)
    }
    fn target_ready(&self, action: &Action) -> Result<bool, String> {
        self.native.target_ready(action)
    }
    fn value_for(&self, action: &Action, name: &str) -> Result<f64, String> {
        if name.starts_with("trinket.")
            || name == "buff.vengeful_retreat_movement.up"
            || name == "active_dot.soul_immolation"
            || name == "soul_fragments"
            || name == "action.reap.souls_consumed"
        {
            return self.value(name);
        }
        self.native.value_for(action, name)
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::{engage, prepare};
    use super::*;
    use crate::simc_apl_native::{Encounter, FightStyle};

    fn movement_fact(
        run: &super::super::super::Run,
        bindings: &crate::simc_apl_native::Bindings,
        equipment: &Equipment,
    ) -> Result<f64, String> {
        let world = run.app.world();
        let actor = run.actor;
        let native = NativeFacts::new(
            world,
            actor,
            bindings,
            Encounter::Fixture {
                started_at: 0.0,
                duration: 300.0,
                style: FightStyle::Patchwerk,
            },
            1,
        );
        DevourerFacts {
            native,
            equipment,
            world,
            actor,
        }
        .value("buff.vengeful_retreat_movement.up")
    }

    #[test]
    #[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec1480 exact profile/traits"]
    fn real_retreat_exposes_movement_until_native_duration_expires() {
        let mut run = prepare(1480, 1).expect("native Devourer profile");
        let actor = run.actor;
        engage(&mut run);
        let mut bindings = super::super::bindings::build();
        let equipment =
            Equipment::load(run.app.world_mut(), actor, &mut bindings).expect("native equipment");
        assert_eq!(
            movement_fact(&run, &bindings, &equipment).unwrap(),
            0.0,
            "no movement before cast"
        );
        crate::spell_cast::cast_for_player_with_target(
            run.app.world_mut(),
            actor,
            198793,
            Some(actor),
        )
        .expect("real Vengeful Retreat");
        assert_eq!(
            movement_fact(&run, &bindings, &equipment).unwrap(),
            1.0,
            "movement starts with real cast"
        );
        for _ in 0..30 {
            run.app.update();
        }
        assert_eq!(
            movement_fact(&run, &bindings, &equipment).unwrap(),
            0.0,
            "native movement duration expires"
        );
    }
}
