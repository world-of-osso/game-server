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
    ("lunar_beam", 204066, false),
    ("wild_guardian", 1269619, true),
    // SimC sc_druid.cpp9157–9161 executes the Cat child on the enemy.
    ("heart_of_the_wild", 1261867, false),
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
    ("moonfire", 164812),
    ("red_moon", 1253582),
    ("rake", 155722),
    ("rip", 1079),
];
const TALENTS: &[(&str, u32)] = &[
    ("convoke_the_spirits", 391528),
    ("ravage", 441583),
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
        if name.starts_with("trinket.")
            || name.starts_with("equipped.")
            || name.starts_with("set_bonus.")
        {
            self.equipment.fact(name, &self.native)
        } else {
            self.native.value(name)
        }
    }
}
impl ActionFacts for Context<'_> {
    fn value_for(&self, action: &Action, name: &str) -> Result<f64, String> {
        if name.starts_with("trinket.")
            || name.starts_with("equipped.")
            || name.starts_with("set_bonus.")
        {
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
    fn guardian_answered_calling_requires_paid_rage_and_single_summon() {
        let _seed = crate::random::seed_scope(70448);
        let mut run = super::super::prepare_registered_player(104, 1).unwrap();
        let actor = run.actor;
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 5487, Some(actor), false).unwrap();
        for _ in 0..40 { run.app.update(); }
        let count = |run: &mut super::super::Run| {
            let world = run.app.world_mut();
            world.query::<&crate::spell_cast::summon::SummonedCreature>().iter(world)
                .filter(|summon| summon.owner == actor && summon.spell_id == 1308399).count()
        };
        assert_eq!(count(&mut run), 0, "form casting alone cannot summon the spirit");
        for _ in 0..80 {
            crate::spell_cast::powers::change_power(run.app.world_mut(), actor, shared::components::PowerType::Rage, 1000.0);
            crate::spell_cast::creature_cast(run.app.world_mut(), actor, 192081, Some(actor), false).unwrap();
            for _ in 0..25 { run.app.update(); }
            assert!(count(&mut run) <= 1, "a new proc cannot overlap the living spirit");
            if count(&mut run) == 1 { break; }
        }
        assert_eq!(count(&mut run), 1, "actual paid Rage must eventually summon the spirit");
        assert!(crate::auras::has_aura(run.app.world(), actor, 1308647));
        for _ in 0..3 {
            crate::spell_cast::powers::change_power(run.app.world_mut(), actor, shared::components::PowerType::Rage, 1000.0);
            crate::spell_cast::creature_cast(run.app.world_mut(), actor, 192081, Some(actor), false).unwrap();
            for _ in 0..25 { run.app.update(); }
        }
        assert_eq!(count(&mut run), 1, "active spirit blocks extra summons");
    }
    #[test]
    #[ignore = "requires Guardian MID2 profile and native world data"]
    fn guardian_blood_frenzy_gains_rage_only_with_owned_thrash() {
        let mut run = super::super::prepare_registered_player(104, 1).unwrap();
        let actor = run.actor;
        let target = run.targets[0];
        let rage = |run: &super::super::Run| {
            crate::spell_cast::powers::power_value(
                run.app.world(),
                actor,
                shared::components::PowerType::Rage,
            )
            .unwrap()
            .0
        };
        for _ in 0..120 {
            run.app.update();
        }
        let before = rage(&run);
        for _ in 0..120 {
            run.app.update();
        }
        assert_eq!(
            rage(&run),
            before,
            "no Thrash target means no Blood Frenzy rage"
        );
        crate::auras::apply_aura(
            run.app.world_mut(),
            &crate::auras::AuraApplication {
                caster: actor,
                target,
                spell_id: 192090,
                effect_mask: u32::MAX,
                source: crate::auras::AuraSource::Cast,
            },
        );
        for _ in 0..120 {
            run.app.update();
        }
        assert!(
            rage(&run) > before,
            "an owned Thrash bleed must earn periodic rage"
        );
    }

    #[test]
    #[ignore = "requires Guardian MID2 profile and native world data"]
    fn guardian_ursocs_guidance_reduces_incarnation_for_paid_ironfur() {
        let mut run = super::super::prepare_registered_player(104, 1).unwrap();
        let actor = run.actor;
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 5487, Some(actor), false)
            .unwrap();
        for _ in 0..40 {
            run.app.update();
        }
        crate::spell_cast::scripts::start_hasted_cooldown(
            run.app.world_mut(),
            actor,
            102558,
            180000.0,
        );
        let remaining = |run: &super::super::Run| {
            let world = run.app.world();
            let spell = world
                .resource::<crate::spell_info::SpellStore>()
                .get(102558)
                .unwrap();
            world
                .get::<crate::spell_cast::SpellHistory>(actor)
                .unwrap()
                .remaining_cooldown_ms(spell, crate::spell_cast::now(world))
        };
        crate::spell_cast::powers::change_power(
            run.app.world_mut(),
            actor,
            shared::components::PowerType::Rage,
            1000.0,
        );
        let before = remaining(&run);
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 192081, Some(actor), false)
            .unwrap();
        let after = remaining(&run);
        assert_eq!(
            before - after,
            1600,
            "40 paid rage divided by the native 25 rage/second divisor"
        );
    }

    #[test]
    #[ignore = "requires Guardian MID2 profile and SimC dungeon-dummy trace"]
    fn guardian_matted_fur_absorbs_real_dummy_damage() {
        let trace = std::env::var("HEALER_TANK_SIMC_TRACE").expect("Guardian trace required");
        let report = super::super::super::tank_replay::replay_simc_with_healing(
            104,
            std::path::Path::new(&trace),
            300,
            70448,
            true,
        )
        .expect("Guardian tank replay");
        let absorbed = report["absorbs_by_spell"]["385787"].as_f64().unwrap_or(0.0);
        assert!(
            absorbed > 0.0,
            "defensive casts must earn a shield consumed by dummy hits: {report}"
        );
    }

    #[test]
    #[ignore = "requires Guardian MID2 profile and SimC dungeon-dummy trace"]
    fn guardian_ursocs_fury_absorbs_real_dummy_damage() {
        let trace = std::env::var("HEALER_TANK_SIMC_TRACE").expect("Guardian trace required");
        let report = super::super::super::tank_replay::replay_simc_with_healing(
            104,
            std::path::Path::new(&trace),
            300,
            70448,
            true,
        )
        .expect("Guardian tank replay");
        let absorbed = report["absorbs_by_spell"]["372505"].as_f64().unwrap_or(0.0);
        assert!(
            absorbed > 0.0,
            "actual Guardian attacks must earn a shield consumed by dummy hits: {report}"
        );
    }

    #[test]
    #[ignore = "requires Guardian MID2 profile and SimC dungeon-dummy trace"]
    fn guardian_replays_dungeon_dummy_with_literal_rotation() {
        let trace = std::env::var("HEALER_TANK_SIMC_TRACE").expect("Guardian trace required");
        let report = super::super::super::tank_replay::replay_simc_with_healing(
            104,
            std::path::Path::new(&trace),
            300,
            70448,
            true,
        )
        .expect("Guardian tank replay");
        assert_eq!(report["spec"], 104);
        assert_eq!(report["rotation_status"], "registered_literal_simc");
        assert!(report["raw_damage"].as_f64().unwrap() > 0.0);
        assert!(report["active_mitigation_uptime"].as_f64().unwrap() > 0.0);
        assert!(
            report["periodic_damage_ticks_by_spell"]["192090"]
                .as_u64()
                .unwrap_or(0)
                > 0,
            "landed Bear Thrash must apply its real periodic bleed"
        );
        assert!(
            report["tank_casts"]["204066"].as_u64().unwrap_or(0) > 0,
            "Lunar Beam must cast on the enemy while its native destination remains caster-centered"
        );
    }

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
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        assert!(
            events
                .iter()
                .any(|event| event.source == Some(run.actor.to_bits())
                    && event.spell_id == Some(1261868)
                    && event.kind == shared::protocol::CombatLogKind::CastSuccess),
            "the selected Cat-form Heart of the Wild must cast on the enemy"
        );
        assert_eq!(
            super::super::tank_mitigation_auras(&run).unwrap(),
            &[192081, 22842, 22812, 200851]
        );
    }
}
