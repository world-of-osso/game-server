//! Thin spec261 bindings; the shared Program and native spell engine own execution.
use super::{
    assassination_rotation::{RogueFacts, read_coatings, stealthed},
    inputs::Profile,
    literal_rotation::{
        self, Driver, PrecombatPoll,
        equipment::Equipment,
        execution::{NativeAction, NativeComposite, Prepared},
    },
    preparation,
};
use crate::simc_apl_native::{Bindings, Encounter, FightStyle};
use bevy::prelude::*;
use server::simc_apl::{Action, ActionFacts, Facts, Program};
use shared::components::PowerType;

// SimC76f8118 sc_rogue.cpp9494–9538 binds these foreground actions.
const ACTIONS: &[(&str, u32, bool)] = &[
    ("stealth", 1784, true),
    ("shadow_blades", 121471, true),
    ("shadow_dance", 185313, true),
    ("goremaws_bite", 426591, false),
    ("vanish", 1856, true),
    ("shadowmeld", 58984, true),
    ("shadowstrike", 185438, false),
    ("shuriken_storm", 197835, false),
    ("gloomblade", 200758, false),
    ("backstab", 53, false),
    ("eviscerate", 196819, false),
    ("black_powder", 319175, false),
    ("secret_technique", 280719, false),
    ("coup_de_grace", 441776, false),
    ("blood_fury", 20572, true),
    ("berserking", 26297, true),
    ("fireblood", 265221, true),
    ("ancestral_call", 274738, true),
    ("arcane_torrent", 25046, true),
    ("arcane_pulse", 260364, true),
    ("lights_judgment", 255647, false),
    ("bag_of_tricks", 312411, false),
];

fn bindings() -> Bindings {
    Bindings {
        actions: ACTIONS
            .iter()
            .map(|&(name, id, _)| (name.into(), id))
            .collect(),
        // sc_rogue.cpp10370–10388/10919–11218: actual native state identities.
        buffs: [
            ("shadow_blades", 121471),
            ("shadow_dance", 185313),
            ("stealth", 1784),
            ("vanish", 11327),
            ("subterfuge", 115192),
            ("slice_and_dice", 315496),
            ("ancient_arts", 1269163),
            ("shadow_techniques", 196911),
            ("lingering_shadow", 385960),
            ("premeditation", 343173),
            ("darkest_night", 457280),
            ("deathstalkers_mark", 457129),
            ("supercharge_1", 470398),
            ("bloodlust", 2825),
        ]
        .into_iter()
        .map(|(name, id)| (name.into(), id))
        .collect(),
        talents: [
            ("deathstalkers_mark", 457052),
            ("unseen_blade", 441146),
            ("ancient_arts_3", 1268939),
            ("potent_powder", 1265952),
        ]
        .into_iter()
        .map(|(name, id)| (name.into(), id))
        .collect(),
        resources: [
            ("energy".into(), PowerType::Energy),
            ("combo_points".into(), PowerType::ComboPoints),
        ]
        .into(),
        global_cooldown_spell: 53,
        ..default()
    }
}

fn load_program(profile: &Profile) -> Result<Program, String> {
    let source = profile
        .options
        .get("actions")
        .and_then(serde_json::Value::as_array)
        .ok_or("Subtlety profile lacks literal actions")?;
    let rows = source
        .iter()
        .map(|row| {
            let row = row.as_str().ok_or("Subtlety action row must be text")?;
            let (key, value) = row
                .split_once('=')
                .ok_or("Subtlety action lacks assignment")?;
            let key = key.trim_end_matches('+');
            let list = if key == "actions" {
                "default"
            } else {
                key.strip_prefix("actions.")
                    .ok_or("Subtlety action lacks list")?
            };
            Ok((list, value.trim_start_matches('/')))
        })
        .collect::<Result<Vec<_>, String>>()?;
    // SimC76f8118 apl_rogue.cpp259–349: retain every source row and predicate.
    Program::parse(&rows)
}

pub(super) struct Rotation {
    driver: Driver,
    bindings: Bindings,
    equipment: Equipment,
    encounter: Encounter,
    coatings: Vec<NativeAction<'static>>,
}

impl Rotation {
    fn load(
        world: &mut World,
        actor: Entity,
        encounter: Encounter,
        profile: &Profile,
    ) -> Result<Self, String> {
        let mut bindings = bindings();
        let equipment = Equipment::load(world, actor, &mut bindings)?;
        literal_rotation::capture_fixture(world, actor, encounter)?;
        Ok(Self {
            driver: Driver::new(load_program(profile)?),
            bindings,
            equipment,
            encounter,
            coatings: read_coatings(world, actor),
        })
    }

    fn poll(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
        precombat: bool,
    ) -> Result<PrecombatPoll, String> {
        let combat = crate::combat_refs::active_opponents(world, actor)
            .next()
            .is_some();
        let stealth = stealthed(world, actor);
        let actions = ACTIONS
            .iter()
            .filter(|&&(name, _, _)| name != "stealth" || (!combat && !stealth))
            .map(|&(name, spell, self_target)| NativeAction {
                name,
                spell,
                self_target,
                talent: None,
            });
        let prepared = Prepared::read(
            world,
            actor,
            targets,
            &self.equipment,
            actions,
            self.driver.snapshot.is_none(),
        )?
        .with_composites(
            world,
            actor,
            targets,
            [NativeComposite {
                name: "apply_poison",
                steps: &self.coatings,
            }],
        )?;
        let facts = SubtletyFacts {
            rogue: RogueFacts::new(
                world,
                actor,
                &self.bindings,
                &self.equipment,
                self.encounter,
                targets,
            ),
            prepared: &prepared,
            equipment: &self.equipment,
        };
        if precombat {
            return self
                .driver
                .poll_precombat(&facts, &prepared, &self.equipment);
        }
        Ok(
            match self
                .driver
                .select("default", &facts, &prepared, &self.equipment)?
            {
                Some((action, execution)) => PrecombatPoll::Selected { action, execution },
                None => PrecombatPoll::Finished,
            },
        )
    }

    pub fn drive(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<(), String> {
        if let PrecombatPoll::Selected { action, execution } =
            self.poll(world, actor, targets, false)?
        {
            self.driver.execute(world, actor, action, execution)?;
        }
        Ok(())
    }
}

pub(super) fn prepare_driver(run: &mut super::Run) -> Result<Box<dyn super::SpecDriver>, String> {
    let encounter = Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: FightStyle::Patchwerk,
    };
    let mut rotation = Rotation::load(run.app.world_mut(), run.actor, encounter, &run.profile)?;
    loop {
        match rotation.poll(run.app.world_mut(), run.actor, &run.targets, true)? {
            PrecombatPoll::Waiting => preparation::finish_pending_setup(run)?,
            PrecombatPoll::Selected { action, execution } => {
                preparation::execute_precombat_plan(run, &mut rotation.driver, action, execution)?
            }
            PrecombatPoll::Finished => break,
        }
    }
    rotation.encounter = Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: FightStyle::Patchwerk,
    };
    Ok(Box::new(rotation))
}

pub(super) const REPORT_KEY: Option<&str> = None;

impl super::SpecDriver for Rotation {
    fn label(&self) -> &'static str {
        "LITERAL_SIMC_SUBTLETY_76f8118"
    }

    fn drive(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<(), String> {
        self.drive(world, actor, targets)
    }
}

struct SubtletyFacts<'a> {
    rogue: RogueFacts<'a>,
    prepared: &'a Prepared,
    equipment: &'a Equipment,
}
impl Facts for SubtletyFacts<'_> {
    fn value(&self, name: &str) -> Result<f64, String> {
        // sc_rogue.cpp1127/9680–9682: the unchanged profile uses the default false.
        if name == "priority_rotation" {
            return Ok(0.0);
        }
        self.rogue.value(name)
    }
}
impl ActionFacts for SubtletyFacts<'_> {
    fn primary_resource(&self) -> Result<String, String> {
        // SimC76f8118 sc_rogue.cpp1221: Rogue pooling uses Energy.
        self.rogue.value("energy")?;
        Ok("energy".into())
    }

    fn ready_at_resource_max(&self, action: &Action, resource: &str) -> Result<bool, String> {
        if resource != "energy" {
            return Err(format!("unbound native Subtlety pool {resource}"));
        }
        Ok(self.prepared.execution(action, self.equipment)?.is_some()
            || self
                .prepared
                .resource_limited(action, self.equipment, PowerType::Energy)?)
    }

    fn target_ids(&self, action: &Action) -> Result<Vec<u64>, String> {
        self.rogue.target_ids(action)
    }
    fn target_ready(&self, action: &Action) -> Result<bool, String> {
        self.rogue.target_ready(action)
    }
    fn value_for(&self, action: &Action, name: &str) -> Result<f64, String> {
        if name == "priority_rotation" {
            self.value(name)
        } else {
            self.rogue.value_for(action, name)
        }
    }
}
