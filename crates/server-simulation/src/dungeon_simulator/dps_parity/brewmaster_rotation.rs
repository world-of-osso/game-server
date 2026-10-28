//! Thin Brewmaster binding for the unchanged profile APL and shared literal controller.
use super::literal_rotation::{
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
    ("invoke_niuzao", 132578, true),
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

fn bindings() -> Bindings {
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

fn program(profile: &super::inputs::Profile) -> Result<Program, String> {
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

pub(super) struct Rotation {
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
        profile: &super::inputs::Profile,
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
        literal_rotation::capture_fixture(world, actor, encounter)?;
        Ok(Self {
            driver,
            bindings,
            equipment,
            encounter,
        })
    }

    pub fn start_combat_at(&mut self, started_at: f64) {
        let Encounter::Fixture {
            duration, style, ..
        } = self.encounter
        else {
            unreachable!()
        };
        self.encounter = Encounter::Fixture {
            started_at,
            duration,
            style,
        };
    }

    pub fn step(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
        precombat: bool,
    ) -> Result<bool, String> {
        let prepared = self.read_prepared(world, actor, targets)?;
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

struct BrewmasterFacts<'a> {
    world: &'a World,
    actor: Entity,
    native: NativeFacts<'a>,
    equipment: &'a Equipment,
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

pub(super) fn prepare_driver(run: &mut super::Run) -> Result<Box<dyn super::SpecDriver>, String> {
    let encounter = crate::simc_apl_native::Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    let mut rotation = Rotation::load(run.app.world_mut(), run.actor, encounter, &run.profile)?;
    while rotation.step(run.app.world_mut(), run.actor, &run.targets, true)? {
        super::preparation::finish_pending_setup(run)?;
    }
    rotation.start_combat_at(crate::spell_cast::now(run.app.world()));
    Ok(Box::new(rotation))
}

pub(super) const REPORT_KEY: Option<&str> = Some("brewmaster_rotation");

impl super::SpecDriver for Rotation {
    fn tank_active_mitigation_auras(&self) -> Option<&'static [u32]> {
        Some(&[119582, 322507, 1241059, 215479, 124255])
    }
    fn label(&self) -> &'static str {
        "LITERAL_SIMC_BREWMASTER_76f8118"
    }

    fn drive(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<(), String> {
        self.step(world, actor, targets, false).map(|_| ())
    }

    fn report(&self) -> serde_json::Value {
        self.report()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    #[ignore = "requires canonical Brewmaster profile and world database"]
    fn harmony_coalescence_spends_real_keg_damage_from_pool() {
        let mut run = super::super::prepare_literal(268, 1).unwrap();
        super::super::engage(&mut run);
        let actor = run.actor;
        let target = run.targets[0];
        assert!(
            run.app
                .world()
                .get::<crate::class_progression::TraitLoadout>(actor)
                .unwrap()
                .active_talent_spells()
                .any(|talent| talent.spell_id == 450529)
        );
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 100780, Some(target), false)
            .unwrap();
        for _ in 0..40 {
            run.app.update();
        }
        let before = run
            .app
            .world()
            .get::<crate::auras::harmony::HarmonyPools>(actor)
            .unwrap()
            .accumulator;
        assert!(before > 0.0);
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 121253, Some(target), false)
            .unwrap();
        let after = run
            .app
            .world()
            .get::<crate::auras::harmony::HarmonyPools>(actor)
            .unwrap()
            .accumulator;
        assert!(
            after < before,
            "Coalescence spends Keg Smash damage: {before} -> {after}"
        );
    }

    #[test]
    #[ignore = "requires canonical Brewmaster profile and world database"]
    fn harmony_real_damage_banks_and_celestial_infusion_spends() {
        let mut run = super::super::prepare_literal(268, 1).unwrap();
        super::super::engage(&mut run);
        let actor = run.actor;
        let target = run.targets[0];
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 100780, Some(target), false)
            .unwrap();
        let bank = run
            .app
            .world()
            .get::<crate::auras::harmony::HarmonyPools>(actor)
            .expect("damage banks Harmony")
            .accumulator;
        assert!(bank > 0.0, "real Tiger Palm must bank damage");
        for _ in 0..40 {
            run.app.update();
        }
        {
            let mut health = run
                .app
                .world_mut()
                .get_mut::<shared::components::Health>(actor)
                .unwrap();
            health.current = health.max * 0.5;
        }
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 322101, Some(actor), false)
            .unwrap();
        let healed_bank = run
            .app
            .world()
            .get::<crate::auras::harmony::HarmonyPools>(actor)
            .unwrap()
            .accumulator;
        assert!(
            healed_bank > bank,
            "effective Expel Harm must add to Harmony"
        );
        let mut bindings = super::bindings();
        let equipment = super::Equipment::load(run.app.world(), actor, &mut bindings).unwrap();
        let world = run.app.world();
        let facts = super::BrewmasterFacts {
            world,
            actor,
            native: super::NativeFacts::new(
                world,
                actor,
                &bindings,
                super::Encounter::Fixture {
                    started_at: 0.0,
                    duration: 300.0,
                    style: crate::simc_apl_native::FightStyle::Patchwerk,
                },
                1,
            ),
            equipment: &equipment,
        };
        assert_eq!(
            server::simc_apl::Facts::value(&facts, "buff.aspect_of_harmony_accumulator.value")
                .unwrap(),
            f64::from(healed_bank)
        );
        assert_eq!(
            server::simc_apl::Facts::value(&facts, "health.max").unwrap(),
            f64::from(world.get::<shared::components::Health>(actor).unwrap().max)
        );
        for _ in 0..40 {
            run.app.update();
        }
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 1241059, Some(actor), false)
            .unwrap();
        let pools = run
            .app
            .world()
            .get::<crate::auras::harmony::HarmonyPools>(actor)
            .unwrap();
        assert_eq!(pools.accumulator, 0.0);
        let before = pools.spender;
        assert!(before > 0.0);
        assert!(crate::auras::has_aura(run.app.world(), actor, 450711));
        for _ in 0..40 {
            run.app.update();
        }
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 100780, Some(target), false)
            .unwrap();
        let after = run
            .app
            .world()
            .get::<crate::auras::harmony::HarmonyPools>(actor)
            .unwrap()
            .spender;
        assert!(after < before, "real damage spends the Harmony pool");
    }

    #[test]
    #[ignore = "requires canonical Brewmaster profile and world database"]
    fn brew_keg_smash_grants_and_extends_shuffle() {
        let mut run = super::super::prepare_literal(268, 1).unwrap();
        super::super::engage(&mut run);
        let actor = run.actor;
        let target = run.targets[0];
        assert_eq!(
            crate::spell_cast::knowledge::resolve_actor_action(run.app.world(), actor, 100784)
                .unwrap(),
            205523
        );
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 121253, Some(target), false)
            .unwrap();
        let now = crate::spell_cast::now(run.app.world());
        let remaining = |world: &bevy::prelude::World| {
            world
                .get::<crate::auras::ActiveAuras>(actor)
                .unwrap()
                .iter()
                .find(|aura| aura.spell_id == 215479 && aura.caster == actor)
                .and_then(|aura| aura.expires_at)
                .unwrap()
                - now
        };
        let first = remaining(run.app.world());
        assert!(first > 4.9 && first <= 5.0, "first Shuffle: {first}");
        for _ in 0..40 {
            run.app.update();
        }
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 205523, Some(target), false)
            .unwrap();
        let second = remaining(run.app.world());
        assert!(
            second > first + 2.9,
            "Blackout Kick extends Shuffle: {first} -> {second}"
        );
    }

    #[test]
    #[ignore = "requires canonical world database and SimC tank trace"]
    fn replays_brewmaster_dummy_pressure() {
        let trace = std::env::var("HEALER_TANK_SIMC_TRACE").expect("SimC trace required");
        let report = super::super::super::tank_replay::replay_simc(
            268,
            std::path::Path::new(&trace),
            300,
            70448,
        )
        .expect("Brewmaster replay");
        assert_eq!(report["spec"], 268);
        assert_eq!(report["rotation_status"], "registered_literal_simc");
        assert!(report["raw_damage"].as_f64().unwrap() > 0.0);
        assert!(report["active_mitigation_uptime"].as_f64().unwrap() > 0.0);
        println!("HEALER_TANK_REPORT={report}");
    }
}
