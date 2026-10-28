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
    fn blood_death_strike_sanguination_scales_with_actual_missing_health() {
        use shared::components::{Health, PowerType};
        use shared::protocol::CombatLogKind;
        fn hit(fraction: f32) -> i32 {
            let _seed = crate::random::seed_scope(70448);
            let mut run = super::super::prepare_registered_player(250, 1).unwrap();
            let actor = run.actor;
            let target = run.targets[0];
            super::super::engage(&mut run);
            assert!(crate::auras::has_aura(run.app.world(), actor, 326801));
            assert!(crate::auras::has_aura(run.app.world(), actor, 374261));
            let mut health = run.app.world_mut().get_mut::<Health>(target).unwrap();
            health.current = health.max * fraction;
            crate::spell_cast::powers::change_power(
                run.app.world_mut(),
                actor,
                PowerType::RunicPower,
                1000.0,
            );
            crate::spell_cast::creature_cast(
                run.app.world_mut(),
                actor,
                49998,
                Some(target),
                false,
            )
            .unwrap();
            run.app
                .world()
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0
                .iter()
                .rev()
                .find(|event| {
                    event.spell_id == Some(49998)
                        && event.kind == CombatLogKind::Damage
                        && event.source == Some(actor.to_bits())
                })
                .expect("Death Strike hit")
                .amount
        }
        let full = hit(1.0) as f32;
        let half = hit(0.5) as f32;
        let low = hit(0.1) as f32;
        assert!(
            (half / full - 1.65).abs() < 0.01,
            "half={half}, full={full}"
        );
        assert!((low / full - 2.04).abs() < 0.01, "low={low}, full={full}");
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn bloody_reflection_requires_surviving_shield_and_respects_cooldown() {
        use shared::protocol::CombatLogKind;
        let mut run = super::super::prepare_registered_player(250, 1).unwrap();
        let actor = run.actor;
        let attacker = run.targets[0];
        assert!(crate::auras::has_aura(run.app.world(), actor, 1279633));
        crate::auras::remove_auras(
            run.app.world_mut(),
            actor,
            crate::auras::RemoveReason::Cancelled,
            |aura| matches!(aura.spell_id, 77535 | 207203),
        );
        crate::spell_cast::cast_triggered_with_effect0_amount(
            run.app.world_mut(),
            actor,
            77535,
            Some(actor),
            1,
            500_000.0,
        );
        let strike = |run: &mut super::super::Run| {
            crate::spell_cast::effects::replay_simc_dummy_strike(
                run.app.world_mut(),
                attacker,
                actor,
                10_000.0,
                false,
                false,
            )
        };
        let count = |run: &super::super::Run| {
            run.app
                .world()
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0
                .iter()
                .filter(|event| {
                    event.spell_id == Some(1279656)
                        && event.kind == CombatLogKind::Damage
                        && event.target == Some(attacker.to_bits())
                })
                .count()
        };
        strike(&mut run);
        assert_eq!(count(&run), 1, "surviving shield reflects the actual hit");
        strike(&mut run);
        assert_eq!(count(&run), 1, "same-time hit is inside the 500ms cooldown");
        for _ in 0..12 {
            run.app.update();
        }
        strike(&mut run);
        assert_eq!(count(&run), 2, "reflection is available after cooldown");
        for _ in 0..12 {
            run.app.update();
        }
        let hit = server::test_support::healer_intake::Hit {
            tick: 0,
            source_guid: 0,
            pack: 0,
            target_slot: 0,
            school: 1,
            raw: 10_000.0,
            level_scaling_factor: 1.0,
            kind: server::test_support::healer_intake::HitKind::Melee {
                hand: 0,
                outcome: "Hit".into(),
                level_difference: 0,
                critical_multiplier: 1.0,
                weapon_speed: 2.0,
            },
        };
        crate::combat::replay_healer_hit(run.app.world_mut(), attacker, actor, &hit).unwrap();
        assert_eq!(count(&run), 3, "a fully absorbed white swing also reflects");
        for _ in 0..12 {
            run.app.update();
        }
        crate::auras::remove_auras(
            run.app.world_mut(),
            actor,
            crate::auras::RemoveReason::Cancelled,
            |aura| matches!(aura.spell_id, 77535 | 207203),
        );
        strike(&mut run);
        assert_eq!(count(&run), 3, "no Blood Shield means no reflection");
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn blood_death_strike_grants_voracious_leech() {
        let mut run = super::super::prepare_registered_player(250, 1).unwrap();
        let actor = run.actor;
        let target = run.targets[0];
        super::super::engage(&mut run);
        assert!(crate::auras::has_aura(run.app.world(), actor, 273953));
        let before = run
            .app
            .world()
            .get::<crate::stats::CharacterStats>(actor)
            .unwrap()
            .leech_pct;
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
        assert!(crate::auras::has_aura(run.app.world(), actor, 274009));
        let after = run
            .app
            .world()
            .get::<crate::stats::CharacterStats>(actor)
            .unwrap()
            .leech_pct;
        assert!(
            (after - before - 15.0).abs() < 0.01,
            "Death Strike grants the native 15% Leech: {before} -> {after}"
        );
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn blood_gift_duration_includes_everlasting_bond() {
        let mut run = super::super::prepare_registered_player(250, 1).unwrap();
        let actor = run.actor;
        super::super::engage(&mut run);
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 49028, Some(actor), false)
            .unwrap();
        for _ in 0..190 {
            run.app.update();
        }
        assert!(
            crate::auras::has_aura(run.app.world(), actor, 434153),
            "Everlasting Bond extends the Gift past eight seconds"
        );
        for _ in 0..70 {
            run.app.update();
        }
        assert!(
            !crate::auras::has_aura(run.app.world(), actor, 434153),
            "Gift expires after the twelve-second weapon duration"
        );
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn vampiric_strike_heals_blood_player_from_native_health_percentage() {
        use shared::components::Health;
        use shared::protocol::CombatLogKind;
        let mut run = super::super::prepare_registered_player(250, 1).unwrap();
        let actor = run.actor;
        let target = run.targets[0];
        super::super::engage(&mut run);
        let max = run.app.world().get::<Health>(actor).unwrap().max;
        run.app
            .world_mut()
            .get_mut::<Health>(actor)
            .unwrap()
            .current = max * 0.5;
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 433895, Some(target), false)
            .unwrap();
        for _ in 0..10 {
            run.app.update();
        }
        let heals = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        assert!(
            heals.iter().any(|event| event.spell_id == Some(434422)
                && event.kind == CombatLogKind::Heal
                && event.amount >= (max * 0.01) as i32
                && event.overflow < event.amount),
            "Vampiric Strike must heal from its native percentage"
        );
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn health_defenses_proc_only_below_threshold_and_obey_cooldowns() {
        use shared::components::Health;
        let mut run = super::super::prepare_registered_player(250, 1).unwrap();
        let target = run.actor;
        let source = run.targets[0];
        let max = run.app.world().get::<Health>(target).unwrap().max;
        let strike = |run: &mut super::super::Run| {
            crate::auras::remove_auras(
                run.app.world_mut(),
                target,
                crate::auras::RemoveReason::Cancelled,
                |aura| matches!(aura.spell_id, 77535 | 207203),
            );
            crate::spell_cast::effects::replay_simc_dummy_strike(
                run.app.world_mut(),
                source,
                target,
                max * 0.05,
                false,
                false,
            );
            for _ in 0..4 {
                run.app.update();
            }
        };
        strike(&mut run);
        assert!(!crate::auras::has_aura(run.app.world(), target, 326808));
        assert!(!crate::auras::has_aura(run.app.world(), target, 313015));
        run.app
            .world_mut()
            .get_mut::<Health>(target)
            .unwrap()
            .current = max * 0.30;
        strike(&mut run);
        assert!(
            crate::auras::has_aura(run.app.world(), target, 326808),
            "damage below 35% grants Sanguination"
        );
        assert!(
            crate::auras::has_aura(run.app.world(), target, 326809),
            "Sanguination applies Satiated"
        );
        assert!(!crate::auras::has_aura(run.app.world(), target, 313015));
        run.app
            .world_mut()
            .get_mut::<Health>(target)
            .unwrap()
            .current = max * 0.15;
        strike(&mut run);
        assert!(
            crate::auras::has_aura(run.app.world(), target, 313015),
            "damage below 20% grants Recently Failed"
        );
        let heals = |run: &super::super::Run| {
            run.app
                .world()
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0
                .iter()
                .filter(|event| {
                    event.spell_id == Some(313010)
                        && event.kind == shared::protocol::CombatLogKind::Heal
                })
                .count()
        };
        assert_eq!(heals(&run), 1);
        run.app
            .world_mut()
            .get_mut::<Health>(target)
            .unwrap()
            .current = max * 0.15;
        strike(&mut run);
        assert_eq!(
            heals(&run),
            1,
            "Recently Failed prevents another Failsafe heal"
        );
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn permafrost_white_hits_create_consumable_frost_shield() {
        let _seed = crate::random::seed_scope(70448);
        let mut run = super::super::prepare_registered_player(250, 1).unwrap();
        let actor = run.actor;
        assert!(crate::auras::has_aura(run.app.world(), actor, 207200));
        super::super::engage(&mut run);
        for _ in 0..200 {
            run.app.update();
            if crate::auras::has_aura(run.app.world(), actor, 207203) {
                break;
            }
        }
        assert!(
            crate::auras::has_aura(run.app.world(), actor, 207203),
            "a real white hit must create Frost Shield"
        );
        let (_, absorbed) = crate::auras::absorb_damage(run.app.world_mut(), actor, 1, 1_000_000.0);
        assert!(
            absorbed > 0.0,
            "incoming physical damage must consume the shield"
        );
        crate::auras::remove_auras(
            run.app.world_mut(),
            actor,
            crate::auras::RemoveReason::Consumed,
            |aura| aura.spell_id == 207200 || aura.spell_id == 207203,
        );
        for _ in 0..200 {
            run.app.update();
        }
        assert!(
            !crate::auras::has_aura(run.app.world(), actor, 207203),
            "white hits without the talent must not create a shield"
        );
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn blood_rune_weapons_reduce_actual_incoming_strike() {
        let mut run = super::super::prepare_registered_player(250, 1).unwrap();
        let actor = run.actor;
        let source = run.targets[0];
        super::super::engage(&mut run);
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 49028, Some(actor), false)
            .unwrap();
        for _ in 0..4 {
            run.app.update();
        }
        assert!(crate::auras::has_aura(run.app.world(), actor, 1264407));
        let max = run
            .app
            .world()
            .get::<shared::components::Health>(actor)
            .unwrap()
            .max;
        crate::spell_cast::effects::replay_simc_dummy_strike(
            run.app.world_mut(),
            source,
            actor,
            100_000.0,
            false,
            false,
        );
        let with = max
            - run
                .app
                .world()
                .get::<shared::components::Health>(actor)
                .unwrap()
                .current;
        crate::auras::remove_auras(
            run.app.world_mut(),
            actor,
            crate::auras::RemoveReason::Cancelled,
            |aura| aura.spell_id == 1264407,
        );
        for _ in 0..3 {
            run.app.update();
        }
        let mut health = run
            .app
            .world_mut()
            .get_mut::<shared::components::Health>(actor)
            .unwrap();
        health.current = health.max;
        let max = health.max;
        crate::spell_cast::effects::replay_simc_dummy_strike(
            run.app.world_mut(),
            source,
            actor,
            100_000.0,
            false,
            false,
        );
        let without = max
            - run
                .app
                .world()
                .get::<shared::components::Health>(actor)
                .unwrap()
                .current;
        assert!(
            with < without * 0.90,
            "two weapons reduce the actual incoming hit: with={with}, without={without}"
        );
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn blood_rune_spend_can_summon_dance_weapon_and_enable_parry_buff() {
        let _seed = crate::random::seed_scope(70448);
        let mut run = super::super::prepare_registered_player(250, 1).unwrap();
        let actor = run.actor;
        let target = run.targets[0];
        super::super::engage(&mut run);
        let mut found = false;
        for _ in 0..30 {
            crate::spell_cast::powers::change_power(
                run.app.world_mut(),
                actor,
                shared::components::PowerType::Runes,
                6.0,
            );
            crate::spell_cast::creature_cast(
                run.app.world_mut(),
                actor,
                206930,
                Some(target),
                false,
            )
            .unwrap();
            for _ in 0..30 {
                run.app.update();
            }
            let world = run.app.world_mut();
            found = world
                .query::<&crate::spell_cast::summon::SummonedCreature>()
                .iter(world)
                .any(|summon| summon.owner == actor && summon.spell_id == 1264353);
            if found {
                break;
            }
        }
        assert!(
            found,
            "rune spending summons a real Dance of Midnight weapon"
        );
        assert!(
            crate::auras::has_aura(run.app.world(), actor, 81256),
            "real weapon enables the DRW parry buff"
        );
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn blood_parry_during_rune_weapon_grants_free_vampiric_strike() {
        use server::test_support::healer_intake::{Hit, HitKind};
        let mut run = super::super::prepare_registered_player(250, 1).unwrap();
        let actor = run.actor;
        let source = run.targets[0];
        super::super::engage(&mut run);
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 49028, Some(actor), false)
            .unwrap();
        for _ in 0..30 {
            run.app.update();
        }
        let hit = Hit {
            tick: 0,
            source_guid: 0,
            pack: 0,
            target_slot: 0,
            school: 1,
            raw: 100.0,
            level_scaling_factor: 1.0,
            kind: HitKind::Melee {
                hand: 0,
                outcome: "Parry".into(),
                level_difference: 0,
                critical_multiplier: 1.0,
                weapon_speed: 2.0,
            },
        };
        crate::combat::replay_healer_hit(run.app.world_mut(), source, actor, &hit).unwrap();
        run.app.update();
        assert!(
            crate::auras::has_aura(run.app.world(), actor, 1264568),
            "actual parry earns Dance of Midnight"
        );
        crate::spell_cast::powers::change_power(
            run.app.world_mut(),
            actor,
            shared::components::PowerType::Runes,
            -6.0,
        );
        let strike = crate::spell_cast::knowledge::resolve_action(run.app.world(), actor, 206930)
            .unwrap()
            .unwrap();
        assert_eq!(strike, 433895);
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, strike, Some(source), false)
            .unwrap();
        for _ in 0..4 {
            run.app.update();
        }
        assert!(
            !crate::auras::has_aura(run.app.world(), actor, 1264568),
            "Vampiric Strike consumes the free-cast proc"
        );
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn blood_transfusion_increases_real_weapon_damage_per_stack() {
        use shared::components::{Npc, UnitSummonedBy};
        use shared::protocol::CombatLogKind;
        let mut run = super::super::prepare_registered_player(250, 1).unwrap();
        let actor = run.actor;
        let target = run.targets[0];
        super::super::engage(&mut run);
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 49028, Some(actor), false)
            .unwrap();
        for _ in 0..40 {
            run.app.update();
        }
        run.app
            .world_mut()
            .get_mut::<crate::stats::CharacterStats>(actor)
            .unwrap()
            .crit_pct = 0.0;
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 433895, Some(target), false)
            .unwrap();
        for _ in 0..4 {
            run.app.update();
        }
        let world = run.app.world_mut();
        let weapon = world
            .query::<(bevy::prelude::Entity, &Npc, &UnitSummonedBy)>()
            .iter(world)
            .find(|(_, npc, owner)| npc.template_id == 27893 && owner.0 == actor.to_bits())
            .unwrap()
            .0;
        let damage = |world: &mut bevy::prelude::World| {
            let _seed = crate::random::seed_scope(70448);
            let before = world
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0
                .len();
            crate::spell_cast::cast_triggered_with_overrides(
                world,
                weapon,
                49998,
                Some(target),
                1,
                crate::spell_cast::TriggeredSpellOverrides {
                    effect_mask: Some(1),
                    ..Default::default()
                },
            );
            world
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0[before..]
                .iter()
                .find(|event| {
                    event.source == Some(weapon.to_bits())
                        && event.spell_id == Some(49998)
                        && event.kind == CombatLogKind::Damage
                })
                .expect("weapon Death Strike damage")
                .amount
        };
        let boosted = damage(world);
        crate::auras::remove_auras(
            world,
            weapon,
            crate::auras::RemoveReason::Cancelled,
            |aura| aura.spell_id == 1265577,
        );
        for _ in 0..4 {
            run.app.update();
        }
        let unboosted = damage(run.app.world_mut());
        assert!(
            (boosted as f32 / unboosted as f32 - 1.1).abs() < 0.01,
            "one stack: {boosted} vs {unboosted}"
        );
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn blood_rune_weapon_damage_uses_owner_blood_spell_modifiers() {
        use shared::components::{Npc, UnitSummonedBy};
        use shared::protocol::CombatLogKind;
        let mut run = super::super::prepare_registered_player(250, 1).unwrap();
        let actor = run.actor;
        let target = run.targets[0];
        super::super::engage(&mut run);
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 49028, Some(actor), false)
            .unwrap();
        for _ in 0..40 {
            run.app.update();
        }
        let world = run.app.world_mut();
        world
            .get_mut::<crate::stats::CharacterStats>(actor)
            .unwrap()
            .crit_pct = 0.0;
        let weapon = world
            .query::<(bevy::prelude::Entity, &Npc, &UnitSummonedBy)>()
            .iter(world)
            .find(|(_, npc, owner)| npc.template_id == 27893 && owner.0 == actor.to_bits())
            .unwrap()
            .0;
        let damage = |world: &mut bevy::prelude::World| {
            let _seed = crate::random::seed_scope(70448);
            let before = world
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0
                .len();
            crate::spell_cast::cast_triggered_with_overrides(
                world,
                weapon,
                49998,
                Some(target),
                1,
                crate::spell_cast::TriggeredSpellOverrides {
                    effect_mask: Some(1),
                    ..Default::default()
                },
            );
            world
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0[before..]
                .iter()
                .find(|event| {
                    event.source == Some(weapon.to_bits())
                        && event.spell_id == Some(49998)
                        && event.kind == CombatLogKind::Damage
                })
                .expect("weapon Death Strike damage")
                .amount
        };
        let boosted = damage(world);
        crate::auras::remove_auras(
            world,
            actor,
            crate::auras::RemoveReason::Cancelled,
            |aura| aura.spell_id == 137008,
        );
        let unboosted = damage(world);
        assert!(
            boosted as f32 > unboosted as f32 * 2.5,
            "owner Blood modifiers: {boosted} vs {unboosted}"
        );
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn blood_dancing_rune_weapons_mirror_damage_with_owner_attack_power() {
        use shared::components::{Npc, UnitSummonedBy};
        use shared::protocol::CombatLogKind;
        for (owner_spell, weapon_spell) in [
            (49998, 49998),
            (206930, 228645),
            (433895, 433895),
            (50842, 50842),
        ] {
            let _seed = crate::random::seed_scope(70448);
            let mut run = super::super::prepare_registered_player(250, 1).unwrap();
            let actor = run.actor;
            let target = run.targets[0];
            super::super::engage(&mut run);
            crate::spell_cast::creature_cast(run.app.world_mut(), actor, 49028, Some(actor), false)
                .unwrap();
            for _ in 0..40 {
                run.app.update();
            }
            let world = run.app.world_mut();
            let weapons: Vec<_> = world
                .query::<(bevy::prelude::Entity, &Npc, &UnitSummonedBy)>()
                .iter(world)
                .filter(|(_, npc, owner)| npc.template_id == 27893 && owner.0 == actor.to_bits())
                .map(|(entity, _, _)| entity)
                .collect();
            assert_eq!(weapons.len(), 2);
            let stats = world.get::<crate::stats::CharacterStats>(actor).unwrap();
            let owner_ap = stats.total_attack_power(
                crate::stats::WeaponAttackType::MainHand,
                world.get::<crate::stats::StatModifiers>(actor),
            );
            for &weapon in &weapons {
                assert_eq!(
                    crate::combat::read_pet_owner_critical_chance(world, weapon),
                    Some(stats.crit_pct),
                    "weapon must inherit owner critical chance"
                );
                let (_, pet_ap) = crate::spell_cast::effects::power_stats(world, weapon, false);
                assert!(
                    (pet_ap - owner_ap / 3.0).abs() < 0.01,
                    "weapon AP {pet_ap}, owner AP {owner_ap}"
                );
            }
            crate::spell_cast::powers::change_power(
                world,
                actor,
                shared::components::PowerType::RunicPower,
                1000.0,
            );
            crate::spell_cast::creature_cast(world, actor, owner_spell, Some(target), false)
                .unwrap();
            for _ in 0..4 {
                run.app.update();
            }
            let events = &run
                .app
                .world()
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0;
            for weapon in weapons {
                assert!(
                    events
                        .iter()
                        .any(|event| event.source == Some(weapon.to_bits())
                            && event.target == Some(target.to_bits())
                            && event.spell_id == Some(weapon_spell)
                            && event.kind == CombatLogKind::Damage
                            && event.amount > 0),
                    "missing weapon damage for {owner_spell}"
                );
                if owner_spell == 433895 {
                    assert!(
                        crate::auras::has_aura(run.app.world(), weapon, 1265577),
                        "Vampiric Strike grants each weapon Transfusion"
                    );
                }
            }
        }
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn blood_dancing_rune_weapon_spawns_owned_actor() {
        use shared::components::{Npc, UnitSummonedBy};
        let mut run = super::super::prepare_registered_player(250, 1).unwrap();
        let actor = run.actor;
        super::super::engage(&mut run);
        assert!(
            run.app
                .world()
                .get::<crate::class_progression::KnownSpells>(actor)
                .unwrap()
                .contains(377637)
        );
        crate::auras::remove_auras(
            run.app.world_mut(),
            actor,
            crate::auras::RemoveReason::Cancelled,
            |aura| aura.spell_id == 195181,
        );
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 49028, Some(actor), false)
            .unwrap();
        for _ in 0..4 {
            run.app.update();
        }
        let world = run.app.world_mut();
        let count = world
            .query::<(&Npc, &UnitSummonedBy)>()
            .iter(world)
            .filter(|(npc, owner)| npc.template_id == 27893 && owner.0 == actor.to_bits())
            .count();
        assert_eq!(count, 2, "Everlasting Bond summons a second owned weapon");
        let dance = world
            .get::<crate::auras::ActiveAuras>(actor)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 1264407)
            .expect("weapons grant Dance of Midnight defense");
        assert_eq!(dance.stacks, 2);
        assert_eq!(
            dance
                .effects
                .iter()
                .find(|effect| effect.index == 3)
                .unwrap()
                .amount,
            -6.0
        );
        let bone = world
            .get::<crate::auras::ActiveAuras>(actor)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 195181)
            .expect("Insatiable Blade grants Bone Shield");
        assert_eq!(bone.stacks, 5);
        let target = run.targets[0];
        for _ in 0..130 {
            run.app.update();
        }
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 195292, Some(target), false)
            .unwrap();
        for _ in 0..30 {
            run.app.update();
        }
        let bone = run
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(actor)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 195181)
            .unwrap();
        assert_eq!(
            bone.stacks, 11,
            "owner and both weapon casts grant two stacks each"
        );
        crate::auras::remove_auras(
            run.app.world_mut(),
            actor,
            crate::auras::RemoveReason::Cancelled,
            |aura| aura.spell_id == 195181,
        );
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 195182, Some(target), false)
            .unwrap();
        for _ in 0..30 {
            run.app.update();
        }
        let world = run.app.world_mut();
        let weapons = world
            .query::<(&Npc, &UnitSummonedBy)>()
            .iter(world)
            .filter(|(npc, owner)| npc.template_id == 27893 && owner.0 == actor.to_bits())
            .count();
        let bone = world
            .get::<crate::auras::ActiveAuras>(actor)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 195181)
            .expect("Marrowrend grants Bone Shield");
        assert_eq!(
            usize::from(bone.stacks),
            (3 * (1 + weapons)).min(12),
            "owner and active weapons grant three stacks each"
        );
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn blood_bone_shield_stats_do_not_scale_with_stacks() {
        let mut run = super::super::prepare_registered_player(250, 1).unwrap();
        let actor = run.actor;
        crate::auras::remove_auras(
            run.app.world_mut(),
            actor,
            crate::auras::RemoveReason::Cancelled,
            |aura| aura.spell_id == 195181,
        );
        let application = crate::auras::AuraApplication {
            caster: actor,
            target: actor,
            spell_id: 195181,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        };
        crate::auras::apply_aura(run.app.world_mut(), &application);
        for _ in 0..4 {
            run.app.update();
        }
        let one = run
            .app
            .world()
            .get::<crate::stats::CharacterStats>(actor)
            .unwrap()
            .clone();
        for _ in 0..4 {
            crate::auras::apply_aura(run.app.world_mut(), &application);
        }
        for _ in 0..4 {
            run.app.update();
        }
        let five = run
            .app
            .world()
            .get::<crate::stats::CharacterStats>(actor)
            .unwrap();
        let auras = run
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(actor)
            .unwrap();
        assert_eq!(
            auras
                .iter()
                .find(|aura| aura.spell_id == 195181)
                .unwrap()
                .stacks,
            5
        );
        assert!((one.haste_pct - five.haste_pct).abs() < 0.01);
        assert!((one.armor - five.armor).abs() < 0.01);
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn flagged_fury_of_the_ages_haste_does_not_scale_with_stacks() {
        let mut run = super::super::prepare_registered_player(250, 1).unwrap();
        let actor = run.actor;
        let application = crate::auras::AuraApplication {
            caster: actor,
            target: actor,
            spell_id: 347490,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        };
        crate::auras::apply_aura(run.app.world_mut(), &application);
        for _ in 0..4 {
            run.app.update();
        }
        let one = run
            .app
            .world()
            .get::<crate::stats::CharacterStats>(actor)
            .unwrap()
            .haste_pct;
        crate::auras::apply_aura(run.app.world_mut(), &application);
        for _ in 0..4 {
            run.app.update();
        }
        let two = run
            .app
            .world()
            .get::<crate::stats::CharacterStats>(actor)
            .unwrap()
            .haste_pct;
        let auras = run
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(actor)
            .unwrap();
        assert_eq!(
            auras
                .iter()
                .find(|aura| aura.spell_id == 347490)
                .unwrap()
                .stacks,
            2
        );
        assert!((one - two).abs() < 0.01);
    }

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
    #[ignore = "requires canonical world database and MID2 Unholy inputs"]
    fn unholy_essence_haste_uses_unholy_gift_multiplier() {
        let mut run = super::super::prepare_registered_player(252, 1).unwrap();
        let actor = run.actor;
        for _ in 0..4 {
            run.app.update();
        }
        let before = run
            .app
            .world()
            .get::<crate::stats::CharacterStats>(actor)
            .unwrap()
            .haste_pct;
        for spell_id in [433925, 434153] {
            crate::auras::apply_aura(
                run.app.world_mut(),
                &crate::auras::AuraApplication {
                    caster: actor,
                    target: actor,
                    spell_id,
                    effect_mask: u32::MAX,
                    source: crate::auras::AuraSource::Cast,
                },
            );
        }
        for _ in 0..4 {
            run.app.update();
        }
        let after = run
            .app
            .world()
            .get::<crate::stats::CharacterStats>(actor)
            .unwrap()
            .haste_pct;
        let expected = ((1.0 + before / 100.0) * 1.018 - 1.0) * 100.0;
        assert!(
            (after - expected).abs() < 0.05,
            "Unholy Gift scales one Essence stack by 1.8: before={before} after={after}"
        );
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn blood_boiling_point_proc_consumes_and_echoes_blood_boil() {
        let _seed = crate::random::seed_scope(70448);
        let mut run = super::super::prepare_registered_player(250, 1).unwrap();
        let actor = run.actor;
        let target = run.targets[0];
        super::super::engage(&mut run);
        for _ in 0..30 {
            crate::spell_cast::creature_cast(
                run.app.world_mut(),
                actor,
                206930,
                Some(target),
                false,
            )
            .unwrap();
            for _ in 0..30 {
                run.app.update();
            }
            if crate::auras::has_aura(run.app.world(), actor, 1265968) {
                break;
            }
        }
        assert!(crate::auras::has_aura(run.app.world(), actor, 1265968));
        let start = run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .len();
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 50842, None, false).unwrap();
        for _ in 0..4 {
            run.app.update();
        }
        assert!(!crate::auras::has_aura(run.app.world(), actor, 1265968));
        assert!(crate::auras::has_aura(run.app.world(), actor, 1265982));
        for _ in 0..70 {
            run.app.update();
        }
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0[start..];
        let hits = events
            .iter()
            .filter(|event| {
                event.source == Some(actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(50842)
                    && event.kind == shared::protocol::CombatLogKind::Damage
            })
            .count();
        assert_eq!(hits, 2, "one Blood Boil hit and one delayed echo");
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn blood_death_strike_can_earn_and_consume_vampiric_strike() {
        let _seed = crate::random::seed_scope(70448);
        let mut run = super::super::prepare_registered_player(250, 1).unwrap();
        let actor = run.actor;
        let target = run.targets[0];
        super::super::engage(&mut run);
        for _ in 0..20 {
            crate::spell_cast::powers::change_power(
                run.app.world_mut(),
                actor,
                shared::components::PowerType::RunicPower,
                1000.0,
            );
            crate::spell_cast::creature_cast(
                run.app.world_mut(),
                actor,
                49998,
                Some(target),
                false,
            )
            .unwrap();
            for _ in 0..30 {
                run.app.update();
            }
            if crate::auras::has_aura(run.app.world(), actor, 433899) {
                break;
            }
        }
        assert!(
            crate::auras::has_aura(run.app.world(), actor, 433899),
            "Death Strike earns the proc"
        );
        let strike = crate::spell_cast::knowledge::resolve_action(run.app.world(), actor, 206930)
            .unwrap()
            .unwrap();
        assert_eq!(strike, 433895);
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, strike, Some(target), false)
            .unwrap();
        for _ in 0..4 {
            run.app.update();
        }
        assert!(!crate::auras::has_aura(run.app.world(), actor, 433899));
        assert!(crate::auras::has_aura(run.app.world(), actor, 433925));
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn blood_sanlayn_gift_vampiric_strikes_raise_and_expire_mastery() {
        let mut run = super::super::prepare_registered_player(250, 1).unwrap();
        let actor = run.actor;
        let target = run.targets[0];
        super::super::engage(&mut run);
        for _ in 0..4 {
            run.app.update();
        }
        let before = run
            .app
            .world()
            .get::<crate::stats::CharacterStats>(actor)
            .unwrap()
            .mastery_points;
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 55233, Some(actor), false)
            .unwrap();
        for _ in 0..30 {
            run.app.update();
        }
        assert!(crate::auras::has_aura(run.app.world(), actor, 434153));
        let before_haste = run
            .app
            .world()
            .get::<crate::stats::CharacterStats>(actor)
            .unwrap()
            .haste_pct;
        let strike = crate::spell_cast::knowledge::resolve_action(run.app.world(), actor, 206930)
            .unwrap()
            .expect("Heart Strike resolves");
        assert_eq!(strike, 433895);
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, strike, Some(target), false)
            .unwrap();
        for _ in 0..4 {
            run.app.update();
        }
        assert!(
            crate::auras::has_aura(run.app.world(), actor, 433925),
            "Vampiric Strike earns Essence"
        );
        let during = run
            .app
            .world()
            .get::<crate::stats::CharacterStats>(actor)
            .unwrap()
            .mastery_points;
        let during_haste = run
            .app
            .world()
            .get::<crate::stats::CharacterStats>(actor)
            .unwrap()
            .haste_pct;
        assert!(
            (during_haste - ((1.0 + before_haste / 100.0) * 1.03 - 1.0) * 100.0).abs() < 0.05,
            "one Essence stack grants 3% haste during Blood Gift: before={before_haste}, during={during_haste}"
        );
        assert!(
            during >= before + 0.59,
            "Gift amplifies one Blood Thrill stack: before={before} during={during}"
        );
        for _ in 0..500 {
            run.app.update();
        }
        assert!(!crate::auras::has_aura(run.app.world(), actor, 433925));
        let after = run
            .app
            .world()
            .get::<crate::stats::CharacterStats>(actor)
            .unwrap()
            .mastery_points;
        crate::auras::apply_aura(
            run.app.world_mut(),
            &crate::auras::AuraApplication {
                caster: actor,
                target: actor,
                spell_id: 433925,
                effect_mask: u32::MAX,
                source: crate::auras::AuraSource::Cast,
            },
        );
        for _ in 0..3 {
            run.app.update();
        }
        let renewed = run
            .app
            .world()
            .get::<crate::stats::CharacterStats>(actor)
            .unwrap()
            .mastery_points;
        assert!(
            (renewed - after - 0.2).abs() < 0.01,
            "without Gift, one Essence stack adds 0.2 mastery points"
        );
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn blood_mid2_death_strikes_build_debt_and_marrowrend_spends_it() {
        use crate::auras::{ActiveAuras, AuraApplication, AuraSource};
        use shared::components::PowerType;
        let mut run = super::super::prepare_registered_player(250, 1).unwrap();
        let actor = run.actor;
        let target = run.targets[0];
        super::super::engage(&mut run);
        for spell_id in [1296650, 1296651] {
            crate::auras::apply_aura(
                run.app.world_mut(),
                &AuraApplication {
                    caster: actor,
                    target: actor,
                    spell_id,
                    effect_mask: u32::MAX,
                    source: AuraSource::ItemSet(2055),
                },
            );
        }
        for expected in 1..=10 {
            crate::spell_cast::powers::change_power(
                run.app.world_mut(),
                actor,
                PowerType::RunicPower,
                1000.0,
            );
            crate::spell_cast::creature_cast(
                run.app.world_mut(),
                actor,
                49998,
                Some(target),
                false,
            )
            .unwrap();
            for _ in 0..30 {
                run.app.update();
            }
            let auras = run.app.world().get::<ActiveAuras>(actor).unwrap();
            let debt = auras
                .iter()
                .find(|aura| aura.spell_id == 1310372)
                .expect("Death Strike builds Blood Debt");
            assert_eq!(debt.stacks, expected);
            assert_eq!(debt.effects[0].amount, 0.5);
        }
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 195182, Some(target), false)
            .unwrap();
        for _ in 0..3 {
            run.app.update();
        }
        let auras = run.app.world().get::<ActiveAuras>(actor).unwrap();
        assert!(!auras.iter().any(|aura| aura.spell_id == 1310372));
        assert!(auras.iter().any(|aura| aura.spell_id == 1300369));
        assert!(
            auras
                .iter()
                .any(|aura| aura.spell_id == 195181 && aura.stacks >= 3)
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
    fn blood_death_and_decay_pulses_only_inside_its_real_field() {
        use shared::components::Position;
        use shared::protocol::CombatLogKind;
        let mut run = super::super::prepare_registered_player(250, 1).unwrap();
        let actor = run.actor;
        let target = run.targets[0];
        super::super::engage(&mut run);
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 43265, Some(actor), false)
            .unwrap();
        let count = |run: &super::super::Run| {
            run.app
                .world()
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0
                .iter()
                .filter(|event| {
                    event.source == Some(actor.to_bits())
                        && event.target == Some(target.to_bits())
                        && event.spell_id == Some(52212)
                        && event.kind == CombatLogKind::Damage
                        && event.amount > 0
                })
                .count()
        };
        for _ in 0..60 {
            run.app.update();
        }
        let inside = count(&run);
        assert!(inside >= 2, "expected periodic ground damage, got {inside}");
        let original = *run.app.world().get::<Position>(target).unwrap();
        run.app.world_mut().get_mut::<Position>(target).unwrap().x += 50.0;
        for _ in 0..40 {
            run.app.update();
        }
        assert_eq!(
            count(&run),
            inside,
            "outside target must not take field damage"
        );
        *run.app.world_mut().get_mut::<Position>(target).unwrap() = original;
        for _ in 0..40 {
            run.app.update();
        }
        assert!(
            count(&run) > inside,
            "returning target takes field damage again"
        );
    }

    #[test]
    #[ignore = "requires canonical world database and MID2 Blood inputs"]
    fn blood_relish_in_blood_heals_and_grants_power_on_free_decay() {
        let mut run = super::super::prepare_registered_player(250, 1).unwrap();
        let actor = run.actor;
        super::super::engage(&mut run);
        assert!(
            run.app
                .world()
                .get::<crate::class_progression::KnownSpells>(actor)
                .unwrap()
                .contains(317610)
        );
        for id in [195181, 195181, 195181, 81141] {
            crate::auras::apply_aura(
                run.app.world_mut(),
                &crate::auras::AuraApplication {
                    caster: actor,
                    target: actor,
                    spell_id: id,
                    effect_mask: u32::MAX,
                    source: crate::auras::AuraSource::Cast,
                },
            );
        }
        let mut health = run
            .app
            .world_mut()
            .get_mut::<shared::components::Health>(actor)
            .unwrap();
        health.current = health.max * 0.5;
        let before = health.current;
        crate::spell_cast::powers::change_power(
            run.app.world_mut(),
            actor,
            shared::components::PowerType::RunicPower,
            -1000.0,
        );
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 43265, Some(actor), false)
            .unwrap();
        for _ in 0..4 {
            run.app.update();
        }
        let health = run
            .app
            .world()
            .get::<shared::components::Health>(actor)
            .unwrap()
            .current;
        let power = crate::spell_cast::powers::power_value(
            run.app.world(),
            actor,
            shared::components::PowerType::RunicPower,
        )
        .unwrap()
        .0;
        assert!(health > before, "Relish in Blood must heal");
        assert_eq!(
            power, 100.0,
            "Relish in Blood grants native ten Runic Power"
        );
        assert!(!crate::auras::has_aura(run.app.world(), actor, 81141));
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
        assert!(
            !crate::auras::has_aura(run.app.world(), actor, 81141),
            "Death and Decay consumes its earned Crimson Scourge"
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
        let expiry = run
            .app
            .world_mut()
            .query::<&crate::spell_cast::area_trigger::SpellAreaTrigger>()
            .iter(run.app.world())
            .find(|field| field.spell_id == 43265 && field.caster == actor)
            .unwrap()
            .expires_at
            .unwrap();
        while crate::spell_cast::now(run.app.world()) < expiry + 3.0 {
            run.app.update();
        }
        assert!(
            crate::auras::has_aura(run.app.world(), actor, 391459),
            "Sanguine Ground persists during the four-second expiration grace"
        );
        while crate::spell_cast::now(run.app.world()) < expiry + 4.2 {
            run.app.update();
        }
        assert!(
            !crate::auras::has_aura(run.app.world(), actor, 391459),
            "Sanguine Ground expires after its grace"
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
