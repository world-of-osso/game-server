//! Thin Brewmaster binding for the unchanged profile APL and shared literal controller.
use crate::literal_rotation::{
    self, Driver,
    equipment::Equipment,
    execution::{NativeAction, Prepared},
};
use crate::simc_apl_native::{Bindings, Encounter, NativeFacts};
use bevy::prelude::*;
use server::simc_apl::{Action, ActionFacts, Facts, Program};
use shared::components::PowerType;

// SimC 76f8118 Protection action, aura, and talent aliases.
// SimC76f8118 sc_monk.cpp: Brewmaster action and aura identities.
const ACTIONS: &[(&str, u32, bool)] = &[
    ("black_ox_brew", 115399, true),
    ("celestial_brew", 322507, true),
    ("keg_smash", 121253, false),
    ("blackout_kick", 205523, false),
    ("purifying_brew", 119582, true),
    ("fortifying_brew", 115203, true),
    ("chi_burst", 123986, false),
    // DB2 132578 summons against the selected enemy, as in SimC sc_monk.cpp.
    ("invoke_niuzao", 132578, false),
    ("tiger_palm", 100780, false),
    ("exploding_keg", 325153, false),
    ("empty_the_cellar", 1263438, false),
    ("breath_of_fire", 115181, false),
    ("rushing_jade_wind", 116847, true),
    ("expel_harm", 322101, true),
    ("blood_fury", 20572, true),
    ("berserking", 26297, true),
    ("arcane_torrent", 69179, true),
    ("lights_judgment", 255647, false),
    ("fireblood", 265221, true),
    ("ancestral_call", 274738, true),
    ("bag_of_tricks", 312411, false),
];
const BUFFS: &[(&str, u32)] = &[
    ("aspect_of_harmony_spender", 450711),
    ("aspect_of_harmony_accumulator", 450521),
    ("empty_barrel", 1265307),
    ("blackout_combo", 228563),
    ("invoke_niuzao_the_black_ox", 132578),
    ("empty_the_cellar", 1262768),
];
const TALENTS: &[(&str, u32)] = &[
    ("blackout_combo", 196736),
    ("salsalabims_strength", 383697),
    ("bring_me_another_1", 1265129),
    ("bring_me_another_3", 1265141),
];

pub fn bindings() -> Bindings {
    let names = |rows: &[(&str, u32)]| {
        rows.iter()
            .map(|&(name, id)| (name.to_owned(), id))
            .collect()
    };
    Bindings {
        actions: ACTIONS
            .iter()
            .map(|&(name, id, _)| (name.to_owned(), id))
            .collect(),
        buffs: names(BUFFS),
        talents: names(TALENTS),
        resources: [("energy".into(), PowerType::Energy)].into(),
        dot_refresh: Default::default(),
        global_cooldown_spell: 100780,
    }
}

fn program(profile: &server::followers::parity_profile::Profile) -> Result<Program, String> {
    let actions = profile
        .options
        .get("actions")
        .and_then(serde_json::Value::as_array)
        .ok_or("Brewmaster reference has no literal actions")?;
    let rows = actions
        .iter()
        .map(|row| {
            let row = row.as_str().ok_or("Brewmaster action row is not text")?;
            let (head, action) = row
                .split_once('=')
                .ok_or("Brewmaster action lacks assignment")?;
            let head = head.strip_suffix('+').unwrap_or(head);
            let list = if head == "actions" {
                "default"
            } else {
                head.strip_prefix("actions.")
                    .ok_or("Brewmaster action is not an action list")?
            };
            Ok((list, action.strip_prefix('/').unwrap_or(action)))
        })
        .collect::<Result<Vec<_>, String>>()?;
    Program::parse(&rows)
}

pub struct Rotation {
    driver: Driver,
    bindings: Bindings,
    equipment: Equipment,
    encounter: Encounter,
}

impl Rotation {
    pub fn load(
        world: &mut World,
        actor: Entity,
        encounter: Encounter,
        profile: &server::followers::parity_profile::Profile,
    ) -> Result<Self, String> {
        if profile.spec_id != 268 {
            return Err("literal Brewmaster requires spec268".into());
        }
        let mut bindings = bindings();
        // SimC sc_monk.cpp5349–5352 replaces Celestial Brew when Infusion is selected.
        let infusion_selected = world
            .get::<crate::class_progression::TraitLoadout>(actor)
            .is_some_and(|traits| {
                traits
                    .active_talent_spells()
                    .any(|talent| talent.spell_id == 1241059)
            });
        if infusion_selected {
            bindings.actions.insert("celestial_brew".into(), 1241059);
        }
        let equipment = Equipment::load(world, actor, &mut bindings)?;
        let driver = Driver::new(program(profile)?);
        Ok(Self {
            driver,
            bindings,
            equipment,
            encounter,
        })
    }

    pub fn start_combat_at(&mut self, started_at: f64) {
        self.encounter = literal_rotation::encounter_starting_at(self.encounter, started_at);
    }

    pub fn step(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
        precombat: bool,
    ) -> Result<bool, String> {
        let mut prepared = self.read_prepared(world, actor, targets)?;
        // SimC sc_monk.cpp3498–3505 requires a ticking Stagger pool.
        if !precombat && !crate::auras::has_aura(world, actor, 124255) {
            prepared.suppress_action("purifying_brew");
        }
        let facts = BrewmasterFacts {
            world,
            actor,
            native: NativeFacts::new(world, actor, &self.bindings, self.encounter, targets.len())
                .with_targets(targets.first().copied(), targets),
            equipment: &self.equipment,
        };
        let selected = if precombat {
            precombat_selection(
                self.driver
                    .poll_precombat(&facts, &prepared, &self.equipment)?,
            )?
        } else {
            self.driver
                .select("default", &facts, &prepared, &self.equipment)?
        };
        let Some((action, execution)) = selected else {
            return Ok(false);
        };
        self.driver.execute(world, actor, action, execution)?;
        Ok(true)
    }

    fn read_prepared(
        &self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<Prepared, String> {
        let actions = ACTIONS
            .iter()
            .map(|&(name, spell, self_target)| NativeAction {
                name,
                spell: self.bindings.actions.get(name).copied().unwrap_or(spell),
                self_target,
                talent: None,
            });
        Prepared::read(
            world,
            actor,
            targets,
            &self.equipment,
            actions,
            self.driver.snapshot.is_none(),
        )
    }

    pub fn report(&self) -> serde_json::Value {
        serde_json::json!({"driver": "LITERAL_SIMC_BREWMASTER_76f8118", "decisions": self.driver.decisions,
            "native_precombat_snapshot": self.driver.snapshot,
            "gaps": ["NATIVE_POTION_INVENTORY_UNAVAILABLE", "EXTERNAL_BUFF_PROVIDER_UNAVAILABLE:power_infusion"]})
    }
}

fn precombat_selection(
    poll: literal_rotation::PrecombatPoll,
) -> Result<Option<(Action, literal_rotation::execution::Execution)>, String> {
    match poll {
        literal_rotation::PrecombatPoll::Selected { action, execution } => {
            Ok(Some((action, execution)))
        }
        literal_rotation::PrecombatPoll::Finished => Ok(None),
        literal_rotation::PrecombatPoll::Waiting => {
            Err("Brewmaster precombat must finish pending native setup".into())
        }
    }
}

pub struct BrewmasterFacts<'a> {
    pub world: &'a World,
    pub actor: Entity,
    pub native: NativeFacts<'a>,
    pub equipment: &'a Equipment,
}
impl Facts for BrewmasterFacts<'_> {
    fn value(&self, name: &str) -> Result<f64, String> {
        match name {
            "health.max" => self
                .world
                .get::<shared::components::Health>(self.actor)
                .map(|health| f64::from(health.max))
                .ok_or_else(|| "Brewmaster health unavailable".into()),
            // SimC player.cpp12317–12335 and sc_monk.cpp5774–5776.
            "buff.aspect_of_harmony_accumulator.value" => Ok(f64::from(
                self.world
                    .get::<crate::auras::harmony::HarmonyPools>(self.actor)
                    .map_or(0.0, |pools| pools.accumulator),
            )),
            "apex.1" => self.native.value("talent.bring_me_another_1.enabled"),
            "apex.3" => self.native.value("talent.bring_me_another_3.enabled"),
            _ if name.starts_with("trinket.") || name.starts_with("equipped.") => {
                self.equipment.fact(name, &self.native)
            }
            _ => self.native.value(name),
        }
    }
}
impl ActionFacts for BrewmasterFacts<'_> {
    fn target_ids(&self, action: &Action) -> Result<Vec<u64>, String> {
        self.native.target_ids(action)
    }
    fn value_for(&self, action: &Action, name: &str) -> Result<f64, String> {
        if name.starts_with("trinket.")
            || name.starts_with("equipped.")
            || name.starts_with("apex.")
            || name == "buff.aspect_of_harmony_accumulator.value"
            || name == "health.max"
        {
            self.value(name)
        } else {
            self.native.value_for(action, name)
        }
    }
}

impl crate::follower::SpecRotation for Rotation {
    fn poll_precombat(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<literal_rotation::PrecombatPoll, String> {
        Ok(if self.step(world, actor, targets, true)? {
            literal_rotation::PrecombatPoll::Waiting
        } else {
            literal_rotation::PrecombatPoll::Finished
        })
    }

    fn execute(
        &mut self,
        world: &mut World,
        actor: Entity,
        action: Action,
        execution: literal_rotation::execution::Execution,
    ) -> Result<(), String> {
        self.driver.execute(world, actor, action, execution)
    }

    fn start_combat_at(&mut self, started_at: f64) {
        self.start_combat_at(started_at);
    }

    fn drive(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<(), String> {
        self.step(world, actor, targets, false).map(|_| ())
    }
}
