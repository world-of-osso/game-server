//! Pinned Guardian default APL over native tank state.
use super::literal_rotation::{
    self, Driver,
    equipment::Equipment,
    execution::{NativeAction, Prepared},
};
use crate::simc_apl_native::{
    Bindings, DotRefresh, DotRefreshPolicy, Encounter, NativeFacts, native_dot_spell,
};
use bevy::prelude::*;
use server::simc_apl::{Action, ActionFacts, Facts, Program};
use shared::components::PowerType;

const ACTIONS: &[(&str, u32, bool)] = &[
    ("bear_form", 5487, true),
    ("cat_form", 768, true),
    ("moonkin_form", 24858, true),
    ("ironfur", 192081, true),
    ("barkskin", 22812, true),
    ("bristling_fur", 155835, true),
    ("mangle", 33917, false),
    ("thrash", 77758, true),
    ("maul", 6807, false),
    ("moonfire", 8921, false),
    ("swipe_bear", 213771, true),
    ("red_moon", 1252871, false),
    ("berserk", 50334, true),
    ("lunar_beam", 204066, true),
    ("wild_guardian", 1269619, true),
    ("heart_of_the_wild", 1261867, true),
    ("convoke_the_spirits", 391528, true),
    ("sundering_roar", 1253799, true),
    ("rake", 1822, false),
    ("rip", 1079, false),
    ("shred", 5221, false),
    ("ferocious_bite", 22568, false),
    ("blood_fury", 20572, true),
    ("berserking", 26297, true),
    ("fireblood", 265221, true),
    ("ancestral_call", 274738, true),
];
const BUFFS: &[(&str, u32)] = &[
    ("bear_form", 5487),
    ("cat_form", 768),
    ("moonkin_form", 24858),
    ("ironfur", 192081),
    ("answered_calling", 1308647),
    ("dream_conduit", 1308176),
    ("gory_fur_ironfur", 201671),
    ("gory_fur_maul", 1307881),
    ("gorestained_claws", 1301286),
    ("galactic_guardian", 213708),
    ("ravage", 441585),
    ("bs_inc", 102558),
    ("feline_potential", 441702),
    ("feline_potential_counter", 441701),
    ("lunar_beam", 204066),
    ("thrash", 192090),
    ("red_moon", 1253582),
    ("rake", 155722),
    ("rip", 1079),
];
const TALENTS: &[(&str, u32)] = &[
    ("heart_of_the_wild", 1261867),
    ("fluid_form", 449193),
    ("killing_blow", 1252994),
    ("flashing_claws", 393427),
    ("lunar_calling", 429523),
    ("fount_of_strength", 441675),
    ("raze", 400254),
    ("red_moon", 1252871),
    ("soul_of_the_forest", 158477),
    ("lunation", 429539),
    ("moonkin_form", 197625),
    ("wildpower_surge", 441691),
    ("ursocs_guidance", 393414),
];

fn bindings() -> Bindings {
    let names = |rows: &[(&str, u32)]| {
        rows.iter()
            .map(|&(name, id)| (name.to_owned(), id))
            .collect()
    };
    let mut bindings = Bindings {
        actions: ACTIONS
            .iter()
            .map(|&(name, id, _)| (name.to_owned(), id))
            .collect(),
        buffs: names(BUFFS),
        talents: names(TALENTS),
        resources: [
            ("rage".into(), PowerType::Rage),
            ("combo_points".into(), PowerType::ComboPoints),
        ]
        .into(),
        global_cooldown_spell: 33917,
        ..Default::default()
    };
    bindings.actions.insert("bs_inc".into(), 50334);
    for name in ["thrash", "moonfire", "rake", "rip", "red_moon"] {
        bindings.dot_refresh.insert(
            name.into(),
            DotRefresh {
                resolve: native_dot_spell,
                policy: DotRefreshPolicy::Pandemic,
            },
        );
    }
    bindings
}

fn program() -> Result<Program, String> {
    let rows = include_str!("guardian_tank_rotation/76f8118.simc")
        .lines()
        .map(|row| {
            let (head, action) = row.split_once('=').ok_or("Guardian APL lacks assignment")?;
            let head = head.strip_suffix('+').unwrap_or(head);
            let list = if head == "actions" {
                "default"
            } else {
                head.strip_prefix("actions.")
                    .ok_or("Guardian APL lacks list")?
            };
            Ok((list, action.strip_prefix('/').unwrap_or(action)))
        })
        .collect::<Result<Vec<_>, String>>()?;
    Program::parse(&rows)
}

struct Context<'a> {
    native: NativeFacts<'a>,
    equipment: &'a Equipment,
}
impl Facts for Context<'_> {
    fn value(&self, name: &str) -> Result<f64, String> {
        if name.starts_with("trinket.") || name.starts_with("equipped.") {
            self.equipment.fact(name, &self.native)
        } else {
            self.native.value(name)
        }
    }
}
impl ActionFacts for Context<'_> {
    fn value_for(&self, action: &Action, name: &str) -> Result<f64, String> {
        if name.starts_with("trinket.") || name.starts_with("equipped.") {
            self.value(name)
        } else {
            self.native.value_for(action, name)
        }
    }
    fn target_ids(&self, action: &Action) -> Result<Vec<u64>, String> {
        self.native.target_ids(action)
    }
    fn target_ready(&self, action: &Action) -> Result<bool, String> {
        self.native.target_ready(action)
    }
    fn primary_resource(&self) -> Result<String, String> {
        Ok("rage".into())
    }
}

pub(super) struct Rotation {
    driver: Driver,
    bindings: Bindings,
    equipment: Equipment,
    encounter: Encounter,
}
impl Rotation {
    fn prepared(
        &self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
        snapshot: bool,
    ) -> Result<Prepared, String> {
        Prepared::read(
            world,
            actor,
            targets,
            &self.equipment,
            ACTIONS
                .iter()
                .map(|&(name, spell, self_target)| NativeAction {
                    name,
                    spell,
                    self_target,
                    talent: None,
                }),
            snapshot,
        )
    }
    fn select(
        &mut self,
        world: &World,
        actor: Entity,
        targets: &[Entity],
        list: &str,
        prepared: &Prepared,
    ) -> Result<Option<(Action, super::literal_rotation::execution::Execution)>, String> {
        let facts = Context {
            native: NativeFacts::new(world, actor, &self.bindings, self.encounter, targets.len())
                .with_targets(targets.first().copied(), targets),
            equipment: &self.equipment,
        };
        self.driver.select(list, &facts, prepared, &self.equipment)
    }
}

pub(super) fn prepare_driver(run: &mut super::Run) -> Result<Box<dyn super::SpecDriver>, String> {
    let encounter = Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    let mut bindings = bindings();
    let equipment = Equipment::load(run.app.world(), run.actor, &mut bindings)?;
    literal_rotation::capture_fixture(run.app.world_mut(), run.actor, encounter)?;
    let mut rotation = Rotation {
        driver: Driver::new(program()?),
        bindings,
        equipment,
        encounter,
    };
    super::preparation::finish_pending_setup(run)?;
    loop {
        let prepared = rotation.prepared(
            run.app.world_mut(),
            run.actor,
            &run.targets,
            rotation.driver.snapshot.is_none(),
        )?;
        let Some((action, execution)) = rotation.select(
            run.app.world(),
            run.actor,
            &run.targets,
            "precombat",
            &prepared,
        )?
        else {
            break;
        };
        super::preparation::execute_precombat_plan(run, &mut rotation.driver, action, execution)?;
    }
    rotation.encounter = Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    Ok(Box::new(rotation))
}

pub(super) const REPORT_KEY: Option<&str> = Some("guardian_tank_rotation");
impl super::SpecDriver for Rotation {
    fn label(&self) -> &'static str {
        "LITERAL_SIMC_GUARDIAN_76f8118"
    }
    fn tank_active_mitigation_auras(&self) -> Option<&'static [u32]> {
        Some(&[192081, 22842, 22812, 200851])
    }
    fn drive(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<(), String> {
        let prepared = self.prepared(world, actor, targets, false)?;
        if let Some((action, execution)) =
            self.select(world, actor, targets, "default", &prepared)?
        {
            self.driver.execute(world, actor, action, execution)?;
        }
        Ok(())
    }
    fn report(&self) -> serde_json::Value {
        serde_json::json!({"decisions": self.driver.decisions, "native_precombat_snapshot": self.driver.snapshot})
    }
}

#[cfg(test)]
mod tests {
    #[test]
    #[ignore = "requires Guardian MID2 profile and native world data"]
    fn guardian_literal_adapter_executes_in_bear_form() {
        let mut run = super::super::prepare_registered_player(104, 1).unwrap();
        super::super::engage(&mut run);
        let start = run.app.world().resource::<bevy::prelude::Time>().elapsed();
        super::super::install_fixture_lifetimes(&mut run, start);
        let mut bear = false;
        let mut ironfur = false;
        for _ in 0..600 {
            super::super::drive_player(&mut run).unwrap();
            run.app.update();
            bear |= crate::auras::has_aura(run.app.world(), run.actor, 5487);
            ironfur |= crate::auras::has_aura(run.app.world(), run.actor, 192081);
        }
        assert!(bear, "native Guardian rotation enters Bear Form");
        assert!(
            ironfur,
            "native Guardian rotation earns and spends Rage on Ironfur"
        );
        assert_eq!(
            super::super::tank_mitigation_auras(&run).unwrap(),
            &[192081, 22842, 22812, 200851]
        );
    }
}
