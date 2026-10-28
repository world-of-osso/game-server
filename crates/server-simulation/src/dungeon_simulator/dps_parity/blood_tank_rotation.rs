//! Pinned MID2 Blood Death Knight action identities and native facts.
use super::literal_rotation::equipment::Equipment;
use crate::simc_apl_native::{Bindings, Encounter, NativeFacts};
use bevy::prelude::{Entity, World};
use server::simc_apl::{Action, ActionFacts, Facts};
use shared::components::PowerType;

// SimC 76f8118 sc_death_knight.cpp action constructors and buff lookups.
const ACTIONS: &[(&str, u32)] = &[
    ("blood_boil", 50842),
    ("any_dnd", 43265),
    ("consumption", 274156),
    ("arcane_torrent", 50613),
    ("dancing_rune_weapon", 49028),
    ("death_and_decay", 43265),
    ("death_strike", 49998),
    ("deaths_caress", 195292),
    ("heart_strike", 206930),
    ("marrowrend", 195182),
    ("raise_dead", 46585),
    ("reapers_mark", 439843),
    ("vampiric_blood", 55233),
    ("ancestral_call", 274738),
    ("berserking", 26297),
    ("blood_fury", 20572),
    ("fireblood", 265221),
];
const BUFFS: &[(&str, u32)] = &[
    ("blood_debt", 1310372),
    ("boiling_point", 1265968),
    ("bone_shield", 195181),
    ("crimson_scourge", 81141),
    ("dancing_rune_weapon", 81256),
    ("death_and_decay", 188290),
    ("essence_of_the_blood_queen", 433925),
    ("gift_of_the_sanlayn", 434153),
    ("exterminate", 441416),
    ("reapers_mark_debuff", 434765),
    ("vampiric_blood", 55233),
    ("vampiric_strike", 433899),
    ("visceral_strength", 461130),
];

pub(crate) fn bindings() -> Bindings {
    let names = |entries: &[(&str, u32)]| {
        entries
            .iter()
            .map(|&(name, id)| (name.to_owned(), id))
            .collect()
    };
    Bindings {
        actions: names(ACTIONS),
        buffs: names(BUFFS),
        resources: [
            ("rune".into(), PowerType::Runes),
            ("runic_power".into(), PowerType::RunicPower),
        ]
        .into(),
        global_cooldown_spell: 206930,
        ..Default::default()
    }
}

pub(crate) struct Context<'a> {
    native: NativeFacts<'a>,
    equipment: &'a Equipment,
}

impl<'a> Context<'a> {
    pub(crate) fn new(
        world: &'a World,
        actor: Entity,
        targets: &'a [Entity],
        bindings: &'a Bindings,
        equipment: &'a Equipment,
        encounter: Encounter,
    ) -> Self {
        Self {
            native: NativeFacts::new(world, actor, bindings, encounter, targets.len())
                .with_targets(targets.first().copied(), targets),
            equipment,
        }
    }
}

impl Facts for Context<'_> {
    fn value(&self, name: &str) -> Result<f64, String> {
        if name.starts_with("trinket.") || name.starts_with("equipped.") {
            self.equipment.fact(name, &self.native)
        } else if name == "hero_tree.sanlayn" {
            self.native.value("hero_tree.san'layn")
        } else {
            self.native.value(name)
        }
    }
}

impl ActionFacts for Context<'_> {
    fn value_for(&self, action: &Action, name: &str) -> Result<f64, String> {
        if name.starts_with("trinket.")
            || name.starts_with("equipped.")
            || name == "hero_tree.sanlayn"
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
        self.native.value("runic_power")?;
        Ok("runic_power".into())
    }
}

fn program() -> Result<server::simc_apl::Program, String> {
    let rows = include_str!("blood_tank_rotation/76f8118.simc")
        .lines()
        .map(|row| {
            let (head, action) = row.split_once('=').ok_or("Blood APL lacks assignment")?;
            let head = head.strip_suffix('+').unwrap_or(head);
            let list = if head == "actions" {
                "default"
            } else {
                head.strip_prefix("actions.")
                    .ok_or("Blood APL lacks list")?
            };
            Ok((list, action.strip_prefix('/').unwrap_or(action)))
        })
        .collect::<Result<Vec<_>, String>>()?;
    server::simc_apl::Program::parse(&rows)
}

pub(super) struct Rotation {
    driver: super::literal_rotation::Driver,
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
    ) -> Result<super::literal_rotation::execution::Prepared, String> {
        use super::literal_rotation::execution::{NativeAction, Prepared};
        Prepared::read(
            world,
            actor,
            targets,
            &self.equipment,
            ACTIONS.iter().map(|&(name, spell)| NativeAction {
                name,
                spell,
                self_target: matches!(
                    name,
                    "blood_boil"
                        | "dancing_rune_weapon"
                        | "raise_dead"
                        | "vampiric_blood"
                        | "ancestral_call"
                        | "berserking"
                        | "blood_fury"
                        | "fireblood"
                        | "arcane_torrent"
                ),
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
        prepared: &super::literal_rotation::execution::Prepared,
    ) -> Result<Option<(Action, super::literal_rotation::execution::Execution)>, String> {
        let facts = Context::new(
            world,
            actor,
            targets,
            &self.bindings,
            &self.equipment,
            self.encounter,
        );
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
    super::literal_rotation::capture_fixture(run.app.world_mut(), run.actor, encounter)?;
    let mut rotation = Rotation {
        driver: super::literal_rotation::Driver::new(program()?),
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

pub(super) const REPORT_KEY: Option<&str> = Some("blood_tank_rotation");
impl super::SpecDriver for Rotation {
    fn label(&self) -> &'static str {
        "LITERAL_SIMC_BLOOD_76f8118"
    }
    fn tank_active_mitigation_auras(&self) -> Option<&'static [u32]> {
        Some(&[195181])
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
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn blood_bone_shield_loses_one_stack_on_landed_swings_after_icd() {
        use crate::auras::{ActiveAuras, AuraApplication, AuraSource};
        use server::test_support::healer_intake::{Hit, HitKind};
        let mut run = super::super::prepare_registered_player(250, 1).unwrap();
        let actor = run.actor;
        let source = run.targets[0];
        crate::auras::remove_auras(
            run.app.world_mut(),
            actor,
            crate::auras::RemoveReason::Cancelled,
            |a| a.spell_id == 195181,
        );
        for _ in 0..3 {
            crate::auras::apply_aura(
                run.app.world_mut(),
                &AuraApplication {
                    caster: actor,
                    target: actor,
                    spell_id: 195181,
                    effect_mask: u32::MAX,
                    source: AuraSource::Cast,
                },
            );
        }
        let stacks = |run: &super::super::Run| {
            run.app
                .world()
                .get::<ActiveAuras>(actor)
                .unwrap()
                .iter()
                .find(|a| a.spell_id == 195181)
                .map_or(0, |a| a.stacks)
        };
        let hit = |outcome: &str| Hit {
            tick: 0,
            source_guid: 0,
            pack: 0,
            target_slot: 0,
            school: 1,
            raw: 100.0,
            level_scaling_factor: 1.0,
            kind: HitKind::Melee {
                hand: 0,
                outcome: outcome.into(),
                level_difference: 0,
                critical_multiplier: 1.0,
                weapon_speed: 2.0,
            },
        };
        assert_eq!(stacks(&run), 3);
        crate::combat::replay_healer_hit(run.app.world_mut(), source, actor, &hit("Miss")).unwrap();
        run.app.update();
        assert_eq!(stacks(&run), 3, "misses must not consume Bone Shield");
        crate::combat::replay_healer_hit(run.app.world_mut(), source, actor, &hit("Hit")).unwrap();
        run.app.update();
        assert_eq!(
            stacks(&run),
            2,
            "a landed swing removes one stack, not the whole aura"
        );
        crate::combat::replay_healer_hit(run.app.world_mut(), source, actor, &hit("Hit")).unwrap();
        run.app.update();
        assert_eq!(
            stacks(&run),
            2,
            "the native ICD blocks a second immediate stack loss"
        );
        for _ in 0..52 {
            run.app.update();
        }
        crate::combat::replay_healer_hit(run.app.world_mut(), source, actor, &hit("Hit")).unwrap();
        run.app.update();
        assert_eq!(
            stacks(&run),
            1,
            "a later landed swing consumes one more stack"
        );
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn blood_death_strike_heals_and_creates_real_mastery_shield() {
        use server::test_support::healer_intake::{Hit, HitKind};
        let _seed = crate::random::seed_scope(70448);
        let mut run = super::super::prepare_registered_player(250, 1).unwrap();
        let actor = run.actor;
        let target = run.targets[0];
        super::super::engage(&mut run);
        for _ in 0..4 {
            run.app.update();
        }
        let max = run
            .app
            .world()
            .get::<shared::components::Health>(actor)
            .unwrap()
            .max;
        let hit = Hit {
            tick: 0,
            source_guid: 0,
            pack: 0,
            target_slot: 0,
            school: 1,
            raw: max * 0.7,
            level_scaling_factor: 1.0,
            kind: HitKind::Melee {
                hand: 0,
                outcome: "Hit".into(),
                level_difference: 0,
                critical_multiplier: 1.0,
                weapon_speed: 2.0,
            },
        };
        crate::combat::replay_healer_hit(run.app.world_mut(), target, actor, &hit).unwrap();
        let before = run
            .app
            .world()
            .get::<shared::components::Health>(actor)
            .unwrap()
            .current;
        let current_max = run
            .app
            .world()
            .get::<shared::components::Health>(actor)
            .unwrap()
            .max;
        assert!(
            before < current_max,
            "incoming hit must hurt: current={before} max={current_max}"
        );
        for _ in 0..3 {
            crate::auras::apply_aura(
                run.app.world_mut(),
                &crate::auras::AuraApplication {
                    caster: actor,
                    target: actor,
                    spell_id: 273947,
                    effect_mask: u32::MAX,
                    source: crate::auras::AuraSource::Cast,
                },
            );
        }
        crate::spell_cast::powers::change_power(
            run.app.world_mut(),
            actor,
            shared::components::PowerType::RunicPower,
            1000.0,
        );
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 49998, Some(target), false)
            .unwrap();
        for _ in 0..4 {
            run.app.update();
        }
        let after = run
            .app
            .world()
            .get::<shared::components::Health>(actor)
            .unwrap()
            .current;
        assert!(after > before, "landed Death Strike must heal the caster");
        assert!(
            !crate::auras::has_aura(run.app.world(), actor, 273947),
            "Death Strike must consume Hemostasis after healing"
        );
        assert!(
            crate::auras::has_aura(run.app.world(), actor, 77535),
            "Blood mastery must create a shield"
        );
        let (_, absorbed) = crate::auras::absorb_damage(run.app.world_mut(), actor, 1, max);
        assert!(
            absorbed > 0.0 && absorbed <= max * 0.5,
            "actual physical damage must consume capped Blood Shield: {absorbed}"
        );
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn blood_shield_cap_follows_bloody_reflection() {
        let mut run = super::super::prepare_registered_player(250, 1).unwrap();
        let actor = run.actor;
        assert!(crate::auras::has_aura(run.app.world(), actor, 1279633));
        let max = run
            .app
            .world()
            .get::<shared::components::Health>(actor)
            .unwrap()
            .max;
        crate::spell_cast::cast_triggered_with_effect0_amount(
            run.app.world_mut(),
            actor,
            77535,
            Some(actor),
            1,
            max,
        );
        let (_, selected) = crate::auras::absorb_damage(run.app.world_mut(), actor, 1, max);
        assert!(
            (selected - max * 0.65).abs() < 2.0,
            "selected Bloody Reflection raises the actual cap: {selected}/{max}"
        );
        crate::auras::remove_auras(
            run.app.world_mut(),
            actor,
            crate::auras::RemoveReason::Cancelled,
            |a| a.spell_id == 1279633,
        );
        crate::spell_cast::cast_triggered_with_effect0_amount(
            run.app.world_mut(),
            actor,
            77535,
            Some(actor),
            1,
            max,
        );
        let (_, unselected) = crate::auras::absorb_damage(run.app.world_mut(), actor, 1, max);
        assert!(
            (unselected - max * 0.50).abs() < 2.0,
            "without Bloody Reflection the native cap is 50%: {unselected}/{max}"
        );
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn blood_crimson_scourge_builds_real_ground_and_sanguine_follows_position() {
        let _seed = crate::random::seed_scope(70448);
        let mut run = super::super::prepare_registered_player(250, 1).unwrap();
        let actor = run.actor;
        super::super::engage(&mut run);
        for _ in 0..2000 {
            super::super::drive_literal_tick(&mut run).unwrap();
            run.app.update();
            if crate::auras::has_aura(run.app.world(), actor, 391459) {
                break;
            }
        }
        assert!(
            crate::auras::has_aura(run.app.world(), actor, 188290),
            "real Death and Decay must include its caster"
        );
        assert!(
            crate::auras::has_aura(run.app.world(), actor, 391459),
            "selected Sanguine Ground must follow actual ground"
        );
        let original = *run
            .app
            .world()
            .get::<shared::components::Position>(actor)
            .unwrap();
        run.app
            .world_mut()
            .get_mut::<shared::components::Position>(actor)
            .unwrap()
            .x += 50.0;
        assert!(!crate::auras::has_aura(run.app.world(), actor, 81141), "Death and Decay consumes its earned Crimson Scourge");
        run.app.update();
        assert!(
            !crate::auras::has_aura(run.app.world(), actor, 391459),
            "leaving the ground removes the healing buff"
        );
        *run.app
            .world_mut()
            .get_mut::<shared::components::Position>(actor)
            .unwrap() = original;
        run.app.update();
        assert!(
            crate::auras::has_aura(run.app.world(), actor, 391459),
            "returning to the active field restores the buff"
        );
    }

    #[test]
    #[ignore = "requires canonical world database, MID2 Blood inputs and SimC tank trace"]
    fn blood_replays_mid2_tank_dummy_with_literal_rotation() {
        let trace =
            std::env::var("HEALER_TANK_SIMC_TRACE").expect("HEALER_TANK_SIMC_TRACE required");
        let report = super::super::super::tank_replay::replay_simc(
            250,
            std::path::Path::new(&trace),
            300,
            70448,
        )
        .unwrap();
        assert_eq!(report["spec"], 250);
        assert_eq!(report["level"], 90);
        assert!(report["incoming_events"].as_u64().unwrap() > 0, "{report}");
        assert!(report["raw_damage"].as_f64().unwrap() > 0.0, "{report}");
        assert!(
            report["active_mitigation_uptime"].as_f64().unwrap() > 0.0,
            "{report}"
        );
        assert!(
            report["tank_casts"]["49998"].as_u64().unwrap() > 0,
            "{report}"
        );
        assert!(
            report["tank_casts"]["195292"].as_u64().unwrap() > 0,
            "{report}"
        );
        println!("BLOOD_TANK_REPORT={report}");
    }
}
