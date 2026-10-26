//! Thin spec252 bindings; the shared Program and native spell engine own execution.
use super::{
    inputs::Profile,
    literal_rotation::{
        self, Driver, PrecombatPoll,
        equipment::Equipment,
        execution::{NativeAction, Prepared},
    },
    preparation,
};
use crate::simc_apl_native::{Bindings, Encounter, FightStyle, NativeFacts};
use bevy::prelude::*;
use server::simc_apl::{Action, ActionFacts, Facts, Program};
use shared::components::PowerType;

const ACTIONS: &[(&str, u32, bool)] = &[
    // sc_death_knight.cpp12256–12301: permanent specialization Raise Dead.
    ("raise_dead", 46584, true),
    ("festering_strike", 85948, false),
    ("scourge_strike", 55090, false),
    ("death_coil", 47541, false),
    ("epidemic", 207317, false),
    ("outbreak", 77575, false),
    ("putrefy", 1247378, false),
    ("soul_reaper", 343294, false),
    ("army_of_the_dead", 42650, true),
    ("dark_transformation", 1233448, true),
    ("death_and_decay", 43265, false),
    ("blood_fury", 20572, true),
    ("berserking", 26297, true),
    ("fireblood", 265221, true),
    ("ancestral_call", 274738, true),
    ("arcane_torrent", 50613, true),
    ("arcane_pulse", 260364, true),
    ("lights_judgment", 255647, false),
    ("bag_of_tricks", 312411, false),
];

fn bindings() -> Bindings {
    let mut bindings = Bindings {
        actions: ACTIONS
            .iter()
            .map(|&(name, id, _)| (name.into(), id))
            .collect(),
        // sc_death_knight.cpp15363–15389/16235–16301: native aura identities.
        buffs: [
            ("dark_transformation", 1235391),
            ("sudden_doom", 81340),
            ("festering_scythe", 458123),
            ("festering_scythe_tt", 1241077),
            ("lesser_ghoul_ready", 1254252),
            ("forbidden_knowledge", 1242223),
            ("blightfall", 1271975),
            ("blighted", 1271199),
            ("essence_of_the_blood_queen", 433925),
            ("vampiric_strike", 433899),
            ("dread_plague", 1240996),
            ("soul_reaper_debuff", 1241521),
            ("bloodlust", 2825),
        ]
        .into_iter()
        .map(|(name, id)| (name.into(), id))
        .collect(),
        talents: [
            ("dark_transformation", 1233448),
            ("festering_scythe", 455397),
            ("blightburst", 1254552),
            ("cycle_of_death", 1290864),
            ("army_of_the_dead", 42650),
            ("reaping", 377514),
            ("soul_reaper", 343294),
        ]
        .into_iter()
        .map(|(name, id)| (name.into(), id))
        .collect(),
        resources: [
            ("rune".into(), PowerType::Runes),
            ("runic_power".into(), PowerType::RunicPower),
        ]
        .into(),
        global_cooldown_spell: 85948,
        ..default()
    };
    // sc_death_knight.cpp8805–8830: the Army spawner produces lesser ghouls.
    bindings.actions.insert("army_ghoul".into(), 42650);
    bindings
}

fn load_program(profile: &Profile) -> Result<Program, String> {
    let source = profile
        .options
        .get("actions")
        .and_then(serde_json::Value::as_array)
        .ok_or("Unholy profile lacks literal actions")?;
    let rows = source
        .iter()
        .map(|row| {
            let row = row.as_str().ok_or("Unholy action row must be text")?;
            let (key, value) = row
                .split_once('=')
                .ok_or("Unholy action lacks assignment")?;
            let key = key.trim_end_matches('+');
            let list = if key == "actions" {
                "default"
            } else {
                key.strip_prefix("actions.")
                    .ok_or("Unholy action lacks list")?
            };
            Ok((list, value.trim_start_matches('/')))
        })
        .collect::<Result<Vec<_>, String>>()?;
    // Retain exact source rows; the shared evaluator owns all APL semantics.
    Program::parse(&rows)
}

pub(super) struct Rotation {
    driver: Driver,
    bindings: Bindings,
    equipment: Equipment,
    encounter: Encounter,
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
        })
    }

    fn poll(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
        precombat: bool,
    ) -> Result<PrecombatPoll, String> {
        let actions = ACTIONS
            .iter()
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
        )?;
        let facts = UnholyFacts {
            world,
            actor,
            primary: targets.first().copied(),
            native: NativeFacts::new(world, actor, &self.bindings, self.encounter, targets.len())
                .with_targets(targets.first().copied(), targets),
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
        "LITERAL_SIMC_UNHOLY_76f8118"
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

struct UnholyFacts<'a> {
    world: &'a World,
    actor: Entity,
    primary: Option<Entity>,
    native: NativeFacts<'a>,
    equipment: &'a Equipment,
}

fn equipment_operand(name: &str) -> bool {
    name.starts_with("trinket.") || name.starts_with("equipped.") || name.starts_with("set_bonus.")
}

impl Facts for UnholyFacts<'_> {
    fn value(&self, name: &str) -> Result<f64, String> {
        if equipment_operand(name) {
            return self.equipment.fact(name, &self.native);
        }
        if name == "dot.dread_plague.active_dots" {
            return self.native.value("active_dot.dread_plague");
        }
        // SimC76f8118 action.cpp4276/player.cpp11768–11770,11807–11808,
        // 12574–12600: bare operands bind the actor, even inside target_if.
        if name == "health.pct" {
            let health = self
                .world
                .get::<shared::components::Health>(self.actor)
                .ok_or("native Unholy actor lacks Health")?;
            if health.max <= 0.0 {
                return Err("native Unholy actor has invalid maximum Health".into());
            }
            return Ok(f64::from(health.current) * 100.0 / f64::from(health.max));
        }
        if name == "is_boss" {
            return Ok(self
                .world
                .get::<crate::simc_apl_native::FixtureTarget>(self.actor)
                .map_or(0.0, |role| if role.is_boss { 1.0 } else { 0.0 }));
        }
        self.native.value(name)
    }
}
impl ActionFacts for UnholyFacts<'_> {
    fn target_ids(&self, action: &Action) -> Result<Vec<u64>, String> {
        self.native.target_ids(action)
    }
    fn target_ready(&self, action: &Action) -> Result<bool, String> {
        self.native.target_ready(action)
    }
    fn value_for(&self, action: &Action, name: &str) -> Result<f64, String> {
        if equipment_operand(name)
            || matches!(
                name,
                "dot.dread_plague.active_dots" | "health.pct" | "is_boss"
            )
        {
            return self.value(name);
        }
        if name == "duration" {
            return self.read_action_dot_duration(action);
        }
        self.native.value_for(action, name)
    }
}

impl UnholyFacts<'_> {
    fn read_action_dot_duration(&self, action: &Action) -> Result<f64, String> {
        // SimC76f8118 action.cpp3372/dot.cpp448–456: bare duration is this
        // action's current dot duration; without a captured dot state it is zero.
        let &(_, spell, self_target) = ACTIONS
            .iter()
            .find(|row| row.0 == action.name)
            .ok_or_else(|| format!("unbound native Unholy action {}", action.name))?;
        let target = if self_target {
            Some(self.actor)
        } else {
            action
                .target
                .and_then(Entity::try_from_bits)
                .or(self.primary)
        }
        .ok_or("native Unholy action duration lacks a target")?;
        let now = crate::spell_cast::now(self.world);
        Ok(self
            .world
            .get::<crate::auras::ActiveAuras>(target)
            .into_iter()
            .flat_map(|auras| auras.iter())
            .find(|aura| {
                aura.spell_id == spell
                    && aura.caster == self.actor
                    && aura.expires_at.is_none_or(|end| end > now)
                    && aura.effects.iter().any(|effect| effect.period_ms > 0)
            })
            .map_or(0.0, |aura| f64::from(aura.duration_ms) / 1000.0))
    }
}
