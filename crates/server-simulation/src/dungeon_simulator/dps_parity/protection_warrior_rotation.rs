//! Thin Protection Warrior binding for the unchanged profile APL and shared literal controller.
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
const ACTIONS: &[(&str, u32, bool)] = &[
    ("battle_stance", 386164, true),
    ("charge", 100, false),
    ("avatar", 107574, true),
    ("shield_wall", 871, true),
    ("ignore_pain", 190456, true),
    ("shield_block", 2565, true),
    ("ravager", 228920, false),
    ("thunder_blast", 435222, false),
    ("demoralizing_shout", 1160, true),
    ("champions_leap", 1271985, false),
    ("champions_spear", 376079, false),
    ("demolish", 436358, false),
    ("shield_charge", 385952, false),
    ("thunder_clap", 6343, true),
    ("revenge", 6572, false),
    ("shield_slam", 23922, false),
    ("execute", 163201, false),
    ("wrecking_throw", 384110, false),
    ("shattering_throw", 64382, false),
    ("devastate", 20243, false),
    ("arcane_torrent", 69179, true),
    ("lights_judgment", 255647, false),
    ("bag_of_tricks", 312411, false),
    ("berserking", 26297, true),
    ("blood_fury", 20572, true),
    ("fireblood", 265221, true),
    ("ancestral_call", 274738, true),
];
const BUFFS: &[(&str, u32)] = &[
    ("avatar", 107574),
    ("shield_block", 132404),
    ("ignore_pain", 190456),
    ("thunder_blast", 435615),
    ("violent_outburst", 386478),
    ("seeing_red", 386486),
    ("phalanx", 1278009),
    ("revenge", 5302),
    ("colossal_might", 440989),
    ("sudden_death", 52437),
    ("ravager", 156287),
    ("rend_dot", 388539),
];
const TALENTS: &[(&str, u32)] = &[
    ("booming_voice", 202743),
    ("snap_induction", 456270),
    ("heavy_repercussions", 203177),
    ("practiced_strikes", 429647),
    ("demolish", 436358),
    ("lightning_strikes", 434969),
    ("heavy_handed", 1235088),
    ("javelineer", 1271948),
    ("massacre", 281001),
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
        resources: [("rage".into(), PowerType::Rage)].into(),
        dot_refresh: Default::default(),
        global_cooldown_spell: 23922,
    }
}

fn program(profile: &super::inputs::Profile) -> Result<Program, String> {
    let actions = profile
        .options
        .get("actions")
        .and_then(serde_json::Value::as_array)
        .ok_or("Protection Warrior reference has no literal actions")?;
    let rows = actions
        .iter()
        .map(|row| {
            let row = row
                .as_str()
                .ok_or("Protection Warrior action row is not text")?;
            let (head, action) = row
                .split_once('=')
                .ok_or("Protection Warrior action lacks assignment")?;
            let head = head.strip_suffix('+').unwrap_or(head);
            let list = if head == "actions" {
                "default"
            } else {
                head.strip_prefix("actions.")
                    .ok_or("Protection Warrior action is not an action list")?
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
        if profile.spec_id != 73 {
            return Err("literal Protection Warrior requires spec73".into());
        }
        let mut bindings = bindings();
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
        let facts = ProtectionWarriorFacts {
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
                spell,
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
        serde_json::json!({"driver": "LITERAL_SIMC_PROTECTION_WARRIOR_76f8118", "decisions": self.driver.decisions,
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
            Err("Protection Warrior precombat must finish pending native setup".into())
        }
    }
}

struct ProtectionWarriorFacts<'a> {
    native: NativeFacts<'a>,
    equipment: &'a Equipment,
}
impl Facts for ProtectionWarriorFacts<'_> {
    fn value(&self, name: &str) -> Result<f64, String> {
        if name.starts_with("trinket.") || name.starts_with("equipped.") {
            self.equipment.fact(name, &self.native)
        } else {
            self.native.value(name)
        }
    }
}
impl ActionFacts for ProtectionWarriorFacts<'_> {
    fn target_ids(&self, action: &Action) -> Result<Vec<u64>, String> {
        self.native.target_ids(action)
    }
    fn value_for(&self, action: &Action, name: &str) -> Result<f64, String> {
        if name.starts_with("trinket.") || name.starts_with("equipped.") {
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

pub(super) const REPORT_KEY: Option<&str> = Some("protection_warrior_rotation");

impl super::SpecDriver for Rotation {
    fn tank_active_mitigation_auras(&self) -> Option<&'static [u32]> {
        Some(&[132404, 190456])
    }
    fn label(&self) -> &'static str {
        "LITERAL_SIMC_PROTECTION_WARRIOR_76f8118"
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
    #[ignore = "requires canonical world database and SimC tank trace"]
    fn replays_protection_warrior_dummy_pressure() {
        let trace = std::env::var("HEALER_TANK_SIMC_TRACE").expect("SimC trace required");
        let report = super::super::super::tank_replay::replay_simc(
            73,
            std::path::Path::new(&trace),
            300,
            70448,
        )
        .expect("Protection Warrior replay");
        assert_eq!(report["spec"], 73);
        assert_eq!(report["rotation_status"], "registered_literal_simc");
        assert!(report["raw_damage"].as_f64().unwrap() > 0.0);
        assert!(report["active_mitigation_uptime"].as_f64().unwrap() > 0.0);
        println!("HEALER_TANK_REPORT={report}");
    }
    #[test]
    #[ignore = "requires canonical world database and SimC tank trace"]
    fn tank_report_records_raw_self_healing_and_periodic_damage_ticks() {
        let trace = std::env::var("HEALER_TANK_SIMC_TRACE").expect("SimC trace required");
        let report = super::super::super::tank_replay::replay_simc_with_healing(
            73,
            std::path::Path::new(&trace),
            300,
            70448,
            true,
        )
        .expect("Protection Warrior replay");
        let raw = report["raw_self_healing_by_spell"]["383104"]
            .as_f64()
            .unwrap();
        let effective = report["self_healing_by_spell"]["383104"].as_f64().unwrap();
        assert!(
            raw > effective && effective > 0.0,
            "raw={raw} effective={effective}"
        );
        println!("HEALER_TANK_REPORT={report}");
        assert!(
            report["periodic_damage_ticks_by_spell"]
                .as_object()
                .unwrap()
                .values()
                .any(|value| value.as_u64().unwrap() > 0),
            "periodic tick counts must be reported"
        );
    }
    #[test]
    #[ignore = "requires canonical world database and SimC tank trace"]
    fn tank_replay_enters_fixed_time_execute_phase() {
        let trace = std::env::var("HEALER_TANK_SIMC_TRACE").expect("SimC trace required");
        let report = super::super::super::tank_replay::replay_simc_with_healing(
            73,
            std::path::Path::new(&trace),
            300,
            70448,
            true,
        )
        .expect("Protection Warrior replay");
        let executes = report["tank_casts"]["163201"].as_u64().unwrap_or(0);
        assert!(
            executes >= 15,
            "fixed-time execute phase must produce paid executes: {executes}"
        );
        println!("HEALER_TANK_REPORT={report}");
    }
    #[test]
    fn skyfury_repeats_white_swings_without_resetting_the_timer() {
        use crate::spell_cast::test_harness::harness_with_native_enemy_dodge_reduction;
        use server::spell_info::SpellStore;
        let seed = harness_with_native_enemy_dodge_reduction(
            Vec::new(),
            &[462854, 236279, 236282, 23922],
            1,
        )
        .unwrap();
        let mut skyfury = seed
            .app
            .world()
            .resource::<SpellStore>()
            .get(462854)
            .unwrap()
            .clone();
        let options = skyfury.aura_options.as_mut().unwrap();
        assert_eq!(options.proc_chance, 20);
        assert_eq!(options.proc_type_mask[0] & 4, 4);
        assert_eq!(options.proc_category_recovery_ms, 100);
        options.proc_chance = 100;
        let mut h = harness_with_native_enemy_dodge_reduction(
            vec![skyfury],
            &[462854, 236279, 236282, 23922],
            1,
        )
        .unwrap();
        h.advance(3);
        let caster = h.caster;
        for spell_id in [462854, 236279] {
            crate::auras::apply_aura(
                h.app.world_mut(),
                &crate::auras::AuraApplication {
                    caster,
                    target: caster,
                    spell_id,
                    effect_mask: 1,
                    source: crate::auras::AuraSource::Cast,
                },
            );
        }
        h.attack(h.npc);
        h.advance(2);
        let timer = h
            .app
            .world()
            .get::<crate::combat::AutoAttack>(caster)
            .unwrap()
            .timer;
        h.advance(3);
        let after = h
            .app
            .world()
            .get::<crate::combat::AutoAttack>(caster)
            .unwrap()
            .timer;
        assert!(
            after < timer,
            "repeat must not reset the normal swing timer"
        );
        let procs = h
            .inbox
            .log
            .iter()
            .filter(|event| {
                event.spell_id == Some(236282)
                    && event.kind == shared::protocol::CombatLogKind::Damage
            })
            .count();
        assert!(
            procs >= 2,
            "both original and repeat must trigger Devastator: {procs}"
        );
    }

    #[test]
    fn into_the_fray_tracks_actual_nearby_units_and_haste() {
        use crate::spell_cast::test_harness::{harness_with, spawn_player};
        let mut h = harness_with(Vec::new(), &[202603, 202602], 1, true).unwrap();
        let actor = h.caster;
        let allies: Vec<_> = [2.0, 4.0, 8.0]
            .into_iter()
            .map(|x| spawn_player(&mut h.app, 1, x))
            .collect();
        h.advance(30);
        let read = |h: &crate::spell_cast::test_harness::Harness| {
            let world = h.app.world();
            let aura = world
                .get::<crate::auras::ActiveAuras>(actor)
                .unwrap()
                .iter()
                .find(|a| a.spell_id == 202602)
                .unwrap();
            (
                aura.stacks,
                world
                    .get::<crate::stats::CharacterStats>(actor)
                    .unwrap()
                    .haste_pct,
            )
        };
        let (stacks, four_haste) = read(&h);
        assert_eq!(
            stacks, 4,
            "three nearby allies and one enemy reach the native cap"
        );
        for ally in allies {
            h.app
                .world_mut()
                .get_mut::<shared::components::Position>(ally)
                .unwrap()
                .x = 20.0;
        }
        h.advance(30);
        let (stacks, one_haste) = read(&h);
        assert_eq!(stacks, 1, "only the real nearby enemy remains");
        assert!(
            four_haste > one_haste,
            "native haste must decrease with nearby stacks"
        );
        let enemy = h.npc;
        h.app
            .world_mut()
            .get_mut::<shared::components::Position>(enemy)
            .unwrap()
            .x = 20.0;
        h.advance(30);
        assert!(
            !crate::auras::has_aura(h.app.world(), actor, 202602),
            "no units means no haste aura"
        );
    }

    #[test]
    fn sudden_death_accepts_demolish_and_demoralizing_shout() {
        use crate::spell_cast::test_harness::harness_with_native_enemy_dodge_reduction;
        let ids = [
            29725, 52437, 163201, 280735, 1160, 436358, 440884, 440886, 440888,
        ];
        let seed = harness_with_native_enemy_dodge_reduction(Vec::new(), &ids, 1).unwrap();
        let mut sudden = seed
            .app
            .world()
            .resource::<server::spell_info::SpellStore>()
            .get(29725)
            .unwrap()
            .clone();
        sudden
            .aura_options
            .as_mut()
            .unwrap()
            .procs_per_minute
            .as_mut()
            .unwrap()
            .base_proc_rate = 600.0;
        for spell in [1160, 436358] {
            for selected in [false, true] {
                let mut h =
                    harness_with_native_enemy_dodge_reduction(vec![sudden.clone()], &ids, 1)
                        .unwrap();
                h.advance(3);
                let actor = h.caster;
                if !selected {
                    crate::auras::remove_auras(
                        h.app.world_mut(),
                        actor,
                        crate::auras::RemoveReason::Consumed,
                        |a| a.spell_id == 29725,
                    );
                }
                h.advance(200);
                let target = if spell == 1160 { actor } else { h.npc };
                h.cast_and_deliver(spell, Some(target));
                assert!(
                    h.inbox.failed.is_empty(),
                    "spell={spell}, failures={:?}",
                    h.inbox.failed
                );
                assert_eq!(
                    crate::auras::has_aura(h.app.world(), actor, 52437),
                    selected,
                    "actual foreground action {spell} must roll selected Sudden Death"
                );
            }
        }
    }

    #[test]
    fn free_revenge_is_consumed_by_revenge() {
        let mut h = crate::spell_cast::test_harness::harness_with_native_enemy_dodge_reduction(
            Vec::new(),
            &[5302, 6572],
            1,
        )
        .unwrap();
        h.advance(3);
        let actor = h.caster;
        crate::auras::apply_aura(
            h.app.world_mut(),
            &crate::auras::AuraApplication {
                caster: actor,
                target: actor,
                spell_id: 5302,
                effect_mask: u32::MAX,
                source: crate::auras::AuraSource::Cast,
            },
        );
        h.set_power(shared::components::PowerType::Rage, 500.0);
        let before = h.power(shared::components::PowerType::Rage);
        h.cast_and_deliver(6572, Some(h.npc));
        h.advance(5);
        assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
        assert!(
            h.power(shared::components::PowerType::Rage) >= before,
            "Revenge is free even if the real autoattack generates Rage"
        );
        assert!(
            !crate::auras::has_aura(h.app.world(), actor, 5302),
            "one free Revenge consumes its proc"
        );
    }

    #[test]
    fn revenge_trigger_accepts_only_white_impacts_and_incoming_avoids() {
        use crate::dungeon_simulator::healer_intake::{Hit, HitKind};
        let seed = crate::spell_cast::test_harness::harness_with_native_enemy_dodge_reduction(
            Vec::new(),
            &[5301, 5302],
            1,
        )
        .unwrap();
        let mut trigger = seed
            .app
            .world()
            .resource::<server::spell_info::SpellStore>()
            .get(5301)
            .unwrap()
            .clone();
        trigger
            .aura_options
            .as_mut()
            .unwrap()
            .procs_per_minute
            .as_mut()
            .unwrap()
            .base_proc_rate = 600.0;
        let mut h = crate::spell_cast::test_harness::harness_with_native_enemy_dodge_reduction(
            vec![trigger],
            &[5301, 5302],
            1,
        )
        .unwrap();
        h.advance(3);
        let actor = h.caster;
        let source = h.npc;
        assert!(crate::auras::has_aura(h.app.world(), actor, 5301));
        for (outcome, expected) in [
            ("Hit", false),
            ("Miss", false),
            ("Dodge", true),
            ("Parry", true),
        ] {
            h.advance(200);
            crate::auras::remove_auras(
                h.app.world_mut(),
                actor,
                crate::auras::RemoveReason::Consumed,
                |a| a.spell_id == 5302,
            );
            crate::combat::replay_healer_hit(
                h.app.world_mut(),
                source,
                actor,
                &Hit {
                    tick: 1,
                    source_guid: 0,
                    pack: 0,
                    target_slot: 0,
                    school: 1,
                    raw: 1.0,
                    level_scaling_factor: 1.0,
                    kind: HitKind::Melee {
                        hand: 0,
                        outcome: outcome.into(),
                        level_difference: 0,
                        critical_multiplier: 1.0,
                        weapon_speed: 2.0,
                    },
                },
            )
            .unwrap();
            h.advance(3);
            assert_eq!(
                crate::auras::has_aura(h.app.world(), actor, 5302),
                expected,
                "incoming {outcome}"
            );
        }
        h.advance(200);
        crate::auras::remove_auras(
            h.app.world_mut(),
            actor,
            crate::auras::RemoveReason::Consumed,
            |a| a.spell_id == 5302,
        );
        h.attack(source);
        h.advance(3);
        assert!(
            crate::auras::has_aura(h.app.world(), actor, 5302),
            "actual white impact must trigger Revenge"
        );
    }

    #[test]
    fn best_served_cold_increases_native_revenge_proc_frequency() {
        fn count_procs(selected: bool) -> usize {
            let mut h = crate::spell_cast::test_harness::harness_with_native_enemy_dodge_reduction(
                Vec::new(),
                &[5301, 5302, 202560],
                1,
            )
            .unwrap();
            h.advance(3);
            let caster = h.caster;
            let target = h.npc;
            crate::auras::remove_auras(
                h.app.world_mut(),
                caster,
                crate::auras::RemoveReason::Consumed,
                |aura| aura.spell_id == 202560,
            );
            if selected {
                crate::auras::apply_aura(
                    h.app.world_mut(),
                    &crate::auras::AuraApplication {
                        caster,
                        target: caster,
                        spell_id: 202560,
                        effect_mask: 7,
                        source: crate::auras::AuraSource::Cast,
                    },
                );
            }
            assert!(crate::auras::has_aura(h.app.world(), caster, 5301));
            let mut successes = 0;
            for seed in 1..=60 {
                h.advance(200);
                crate::auras::remove_auras(
                    h.app.world_mut(),
                    caster,
                    crate::auras::RemoveReason::Consumed,
                    |aura| aura.spell_id == 5302,
                );
                let _seed = crate::random::seed_scope(seed);
                crate::combat::replay_healer_hit(
                    h.app.world_mut(),
                    target,
                    caster,
                    &crate::dungeon_simulator::healer_intake::Hit {
                        tick: 1,
                        source_guid: 0,
                        pack: 0,
                        target_slot: 0,
                        school: 1,
                        raw: 1000.0,
                        level_scaling_factor: 1.0,
                        kind: crate::dungeon_simulator::healer_intake::HitKind::Melee {
                            hand: 0,
                            outcome: "Dodge".into(),
                            level_difference: 0,
                            critical_multiplier: 1.0,
                            weapon_speed: 2.0,
                        },
                    },
                )
                .unwrap();
                h.advance(1);
                successes += usize::from(crate::auras::has_aura(h.app.world(), caster, 5302));
            }
            successes
        }
        let without = count_procs(false);
        let with = count_procs(true);
        assert!(
            with > without,
            "selected native modifier must increase real Revenge procs: {without} vs {with}"
        );
    }

    #[test]
    fn protection_devastator_autoattacks_reset_running_shield_slam() {
        for chance in [0.0, 100.0] {
            let seed = crate::spell_cast::test_harness::harness_with_native_enemy_dodge_reduction(
                Vec::new(),
                &[236279, 236282, 23922],
                1,
            )
            .unwrap();
            let mut talent = seed
                .app
                .world()
                .resource::<server::spell_info::SpellStore>()
                .get(236279)
                .unwrap()
                .clone();
            talent.effects[1].base_points = chance;
            let mut h = crate::spell_cast::test_harness::harness_with_native_enemy_dodge_reduction(
                vec![talent],
                &[236279, 236282, 23922],
                1,
            )
            .unwrap();
            h.advance(3);
            let caster = h.caster;
            crate::auras::remove_auras(
                h.app.world_mut(),
                caster,
                crate::auras::RemoveReason::Consumed,
                |aura| aura.spell_id == 236279,
            );
            h.cast_and_deliver(23922, Some(h.npc));
            assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
            h.advance(5);
            let before = h
                .app
                .world()
                .get::<crate::spell_cast::SpellHistory>(h.caster)
                .unwrap()
                .remaining_cooldown_ms(
                    h.app
                        .world()
                        .resource::<server::spell_info::SpellStore>()
                        .get(23922)
                        .unwrap(),
                    crate::spell_cast::history_now(h.app.world()),
                );
            assert!(
                before > 0,
                "go={:?} cooldowns={:?} failed={:?}",
                h.inbox.go,
                h.inbox.cooldowns,
                h.inbox.failed
            );
            crate::auras::apply_aura(
                h.app.world_mut(),
                &crate::auras::AuraApplication {
                    caster,
                    target: caster,
                    spell_id: 236279,
                    effect_mask: 1,
                    source: crate::auras::AuraSource::Cast,
                },
            );
            h.inbox.cooldowns.clear();
            h.attack(h.npc);
            h.advance(45);
            let procs = h
                .inbox
                .log
                .iter()
                .filter(|event| {
                    event.spell_id == Some(236282)
                        && event.kind == shared::protocol::CombatLogKind::Damage
                })
                .count();
            assert!(procs > 0, "no landed Devastator proc");
            let reset = h
                .inbox
                .cooldowns
                .iter()
                .any(|event| event.spell_id == 23922 && event.remaining_ms == 0);
            assert_eq!(
                reset,
                chance == 100.0,
                "chance={chance} initial deadline={before}"
            );
        }
    }
    #[test]
    fn protection_strategist_revenge_resets_shield_slam_only_when_selected() {
        use crate::spell_cast::test_harness::harness_with_native_enemy_dodge_reduction;
        for selected in [false, true] {
            let ids = [236279, 236282, 384041, 23922, 6572];
            let seed = harness_with_native_enemy_dodge_reduction(Vec::new(), &ids, 1).unwrap();
            let mut devastator = seed
                .app
                .world()
                .resource::<server::spell_info::SpellStore>()
                .get(236279)
                .unwrap()
                .clone();
            devastator.effects[1].base_points = 100.0;
            let mut h =
                harness_with_native_enemy_dodge_reduction(vec![devastator], &ids, 1).unwrap();
            h.advance(3);
            let caster = h.caster;
            crate::auras::remove_auras(
                h.app.world_mut(),
                caster,
                crate::auras::RemoveReason::Consumed,
                |aura| aura.spell_id == 236279 || aura.spell_id == 384041,
            );
            h.cast_and_deliver(23922, Some(h.npc));
            h.advance(35);
            if selected {
                crate::auras::apply_aura(
                    h.app.world_mut(),
                    &crate::auras::AuraApplication {
                        caster,
                        target: caster,
                        spell_id: 384041,
                        effect_mask: 1,
                        source: crate::auras::AuraSource::Cast,
                    },
                );
            }
            h.set_power(shared::components::PowerType::Rage, 1000.0);
            h.inbox.cooldowns.clear();
            h.cast_and_deliver(6572, Some(h.npc));
            h.advance(5);
            assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
            assert!(h.inbox.go.iter().any(|event| event.spell_id == 6572));
            let reset = h
                .inbox
                .cooldowns
                .iter()
                .any(|event| event.spell_id == 23922 && event.remaining_ms == 0);
            assert_eq!(reset, selected);
        }
    }
    #[test]
    fn protection_shield_block_and_charge_add_native_aura_duration() {
        let mut h = crate::spell_cast::test_harness::harness_with_native_enemy_dodge_reduction(
            Vec::new(),
            &[2565, 132404, 231847, 385952, 385953, 385954],
            1,
        )
        .unwrap();
        h.advance(3);
        h.set_power(shared::components::PowerType::Rage, 1000.0);
        h.cast_and_deliver(2565, Some(h.caster));
        let expiry = |h: &crate::spell_cast::test_harness::Harness| {
            h.app
                .world()
                .get::<crate::auras::ActiveAuras>(h.caster)
                .unwrap()
                .iter()
                .find(|a| a.spell_id == 132404)
                .unwrap()
                .expires_at
                .unwrap()
        };
        let first = expiry(&h);
        h.advance(35);
        h.cast_and_deliver(385952, Some(h.npc));
        let after_charge = expiry(&h);
        h.advance(35);
        h.cast_and_deliver(2565, Some(h.caster));
        let after_block = expiry(&h);
        assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
        assert!((after_charge - first - 6.0).abs() < 0.001);
        assert!((after_block - after_charge - 6.0).abs() < 0.001);
    }
    #[test]
    #[ignore = "requires canonical MID2 Protection profile"]
    fn protection_anger_management_reduces_defensives_from_paid_ignore_pain() {
        use super::super::literal_rotation::execution::{Execution, execute};
        use shared::components::PowerType;
        let _seed = crate::random::seed_scope(70448);
        let mut run = super::super::prepare_measurement(73, 1).unwrap();
        super::super::engage(&mut run);
        let actor = run.actor;
        let target = run.targets[0];
        for spell in [107574, 871] {
            execute(
                run.app.world_mut(),
                actor,
                Execution::Spell(spell, actor, Some(target)),
            )
            .unwrap();
            for _ in 0..40 {
                run.app.update();
            }
        }
        run.app
            .world_mut()
            .get_mut::<crate::spell_cast::powers::PowerPools>(actor)
            .unwrap()
            .get_mut(PowerType::Rage)
            .unwrap()
            .current = 1000.0;
        let cooldown = |run: &super::super::Run, id| {
            let world = run.app.world();
            let spell = world
                .resource::<crate::spell_info::SpellStore>()
                .get(id)
                .unwrap();
            world
                .get::<crate::spell_cast::SpellHistory>(actor)
                .unwrap()
                .remaining_cooldown_ms(spell, crate::spell_cast::history_now(world))
        };
        let before = [cooldown(&run, 107574), cooldown(&run, 871)];
        assert!(before.iter().all(|value| *value > 0));
        execute(
            run.app.world_mut(),
            actor,
            Execution::Spell(190456, actor, Some(target)),
        )
        .unwrap();
        let remaining = run
            .app
            .world()
            .get::<crate::spell_cast::powers::PowerPools>(actor)
            .unwrap()
            .get(PowerType::Rage)
            .unwrap()
            .current;
        let paid = 1000.0 - remaining;
        assert!(paid > 0.0);
        let divisor =
            crate::spell_cast::scripts::script_effect_value(run.app.world(), actor, 152278, 1);
        let units = run
            .app
            .world()
            .resource::<crate::spell_cast::powers::PowerTypeTable>()
            .get(PowerType::Rage)
            .unwrap()
            .display_modifier;
        let expected_ms = f64::from(paid / units as f32 / divisor) * 1000.0;
        for (spell, previous) in [107574, 871].into_iter().zip(before) {
            let reduction = previous as f64 - cooldown(&run, spell) as f64;
            assert!(
                (reduction - expected_ms).abs() <= 1.0,
                "spell={spell}: reduction={reduction}, expected={expected_ms}"
            );
        }
    }
    #[test]
    #[ignore = "requires canonical MID2 Protection profile"]
    fn protection_shield_block_uses_two_native_charges() {
        let mut run = super::super::prepare_measurement(73, 1).unwrap();
        super::super::engage(&mut run);
        let world = run.app.world();
        let spell = world
            .resource::<crate::spell_info::SpellStore>()
            .get(2565)
            .unwrap();
        let category = spell.categories.as_ref().unwrap();
        let mods = crate::auras::charge_mods(
            world,
            run.actor,
            category.charge_category,
            category.charge_type_mask,
        );
        assert_eq!(category.max_charges as i32 + mods.extra_charges, 2);
        let haste = world
            .get::<crate::stats::CharacterStats>(run.actor)
            .unwrap()
            .haste_pct;
        assert!(haste > 0.0);
        assert!(
            (mods.recovery_mult - 1.0 / (1.0 + haste / 100.0)).abs() < 0.0001,
            "Shield Block recharge must use native haste: haste={haste}, multiplier={}",
            mods.recovery_mult
        );
    }
    #[test]
    #[ignore = "requires canonical MID2 Protection profile"]
    fn protection_shield_charge_extends_modified_shield_block_duration() {
        use super::super::literal_rotation::execution::{Execution, execute};
        let mut run = super::super::prepare_measurement(73, 1).unwrap();
        super::super::engage(&mut run);
        let actor = run.actor;
        let target = run.targets[0];
        run.app
            .world_mut()
            .get_mut::<crate::spell_cast::powers::PowerPools>(actor)
            .unwrap()
            .get_mut(shared::components::PowerType::Rage)
            .unwrap()
            .current = 1000.0;
        execute(
            run.app.world_mut(),
            actor,
            Execution::Spell(2565, actor, Some(target)),
        )
        .unwrap();
        let block = |run: &super::super::Run| {
            let auras = run
                .app
                .world()
                .get::<crate::auras::ActiveAuras>(actor)
                .unwrap();
            let aura = auras.iter().find(|a| a.spell_id == 132404).unwrap();
            (aura.duration_ms, aura.expires_at.unwrap())
        };
        let (duration, expiry) = block(&run);
        assert_eq!(duration, 8000, "rank-2 Enduring Defenses adds two seconds");
        for _ in 0..40 {
            run.app.update();
        }
        execute(
            run.app.world_mut(),
            actor,
            Execution::Spell(385952, target, Some(target)),
        )
        .unwrap();
        let (_, extended) = block(&run);
        assert!((extended - expiry - 8.0).abs() < 0.001);
    }
    #[test]
    #[ignore = "requires canonical MID2 Protection profile"]
    fn protection_execute_applies_selected_deep_wounds() {
        use super::super::literal_rotation::execution::{Execution, execute};
        let mut run = super::super::prepare_measurement(73, 1).unwrap();
        super::super::engage(&mut run);
        let actor = run.actor;
        let target = run.targets[0];
        let mut health = run
            .app
            .world_mut()
            .get_mut::<shared::components::Health>(target)
            .unwrap();
        health.current = health.max * 0.1;
        run.app
            .world_mut()
            .get_mut::<crate::spell_cast::powers::PowerPools>(actor)
            .unwrap()
            .get_mut(shared::components::PowerType::Rage)
            .unwrap()
            .current = 1000.0;
        execute(
            run.app.world_mut(),
            actor,
            Execution::Spell(163201, target, Some(target)),
        )
        .unwrap();
        for _ in 0..20 {
            run.app.update();
        }
        assert!(
            crate::auras::has_aura(run.app.world(), target, 262115),
            "actual Execute must apply selected Deep Wounds"
        );
    }
    #[test]
    #[ignore = "requires canonical MID2 Protection profile"]
    fn protection_deep_wounds_refresh_preserves_pending_tick() {
        let mut run = super::super::prepare_measurement(73, 1).unwrap();
        let actor = run.actor;
        let target = run.targets[0];
        let ticks = |run: &super::super::Run| {
            run.app
                .world()
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0
                .iter()
                .filter(|event| {
                    event.source == Some(actor.to_bits())
                        && event.target == Some(target.to_bits())
                        && event.spell_id == Some(262115)
                        && event.periodic
                        && event.kind == shared::protocol::CombatLogKind::Damage
                })
                .count()
        };
        crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 262115, Some(target), 1);
        for _ in 0..10 {
            run.app.update();
        }
        let before = ticks(&run);
        crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 262115, Some(target), 1);
        for _ in 0..160 {
            run.app.update();
        }
        assert_eq!(
            ticks(&run) - before,
            7,
            "refresh must retain the pending tick and add six fresh ticks"
        );
    }
    #[test]
    fn protection_fueled_by_violence_heals_from_selected_bleed_ticks() {
        use shared::protocol::CombatLogKind;
        for bleed in [388539, 262115] {
            for selected in [false, true] {
                let mut h =
                    crate::spell_cast::test_harness::harness_with_native_enemy_dodge_reduction(
                        Vec::new(),
                        &[383103, 383104, bleed],
                        1,
                    )
                    .unwrap();
                h.app
                    .world_mut()
                    .entity_mut(h.caster)
                    .insert(shared::components::ActiveSpec(73));
                h.advance(3);
                let actor = h.caster;
                if !selected {
                    crate::auras::remove_auras(
                        h.app.world_mut(),
                        actor,
                        crate::auras::RemoveReason::Consumed,
                        |aura| aura.spell_id == 383103,
                    );
                }
                h.app
                    .world_mut()
                    .get_mut::<shared::components::Health>(actor)
                    .unwrap()
                    .current = 1000.0;
                crate::spell_cast::cast_triggered(h.app.world_mut(), actor, bleed, Some(h.npc), 1);
                h.advance(100);
                let damage: f32 = h
                    .inbox
                    .log
                    .iter()
                    .filter(|e| e.spell_id == Some(bleed) && e.kind == CombatLogKind::Damage)
                    .map(|e| e.amount as f32)
                    .sum();
                let heals: Vec<_> = h
                    .inbox
                    .log
                    .iter()
                    .filter(|e| e.spell_id == Some(383104) && e.kind == CombatLogKind::Heal)
                    .collect();
                assert!(damage > 0.0, "bleed {bleed} must tick");
                if selected {
                    let healing: f32 = heals.iter().map(|e| e.amount as f32).sum();
                    assert!(
                        healing > 0.0 && healing <= damage,
                        "damage={damage} heal={healing}"
                    );
                } else {
                    assert!(heals.is_empty(), "unselected talent must not heal");
                }
            }
        }
    }
    #[test]
    fn protection_blood_and_thunder_applies_rend_from_landed_clap_and_blast() {
        use shared::protocol::CombatLogKind;
        for action in [6343, 435222] {
            for selected in [false, true] {
                let mut observed = false;
                for _ in 0..8 {
                    let mut ids = vec![action, 388539, 435203];
                    if selected {
                        ids.push(384277);
                    }
                    let mut h =
                        crate::spell_cast::test_harness::harness_with_native_enemy_dodge_reduction(
                            Vec::new(),
                            &ids,
                            1,
                        )
                        .unwrap();
                    h.app
                        .world_mut()
                        .entity_mut(h.caster)
                        .insert(shared::components::ActiveSpec(73));
                    h.advance(3);
                    h.cast_and_deliver(action, Some(h.npc));
                    h.advance(20);
                    let landed = h.inbox.log.iter().any(|event| {
                        event.spell_id == Some(action)
                            && event.kind == CombatLogKind::Damage
                            && event.amount > 0
                    });
                    if !landed {
                        continue;
                    }
                    assert_eq!(
                        crate::auras::has_aura(h.app.world(), h.npc, 388539),
                        selected,
                        "action={action} selected={selected}"
                    );
                    observed = true;
                    break;
                }
                assert!(observed, "action={action} must land");
            }
        }
    }
    #[test]
    #[ignore = "requires canonical MID2 Protection profile"]
    fn protection_violent_outburst_tracks_paid_rage_and_grants_ignore_pain() {
        use super::super::literal_rotation::execution::{Execution, execute};
        use shared::components::PowerType;
        let mut run = super::super::prepare_measurement(73, 1).unwrap();
        super::super::engage(&mut run);
        let actor = run.actor;
        let target = run.targets[0];
        let threshold =
            crate::spell_cast::scripts::script_effect_value(run.app.world(), actor, 386477, 0);
        let units = run
            .app
            .world()
            .resource::<crate::spell_cast::powers::PowerTypeTable>()
            .get(PowerType::Rage)
            .unwrap()
            .display_modifier as f32;
        assert!(crate::auras::has_aura(run.app.world(), actor, 386477));
        let mut paid = 0.0;
        while paid < threshold {
            assert!(!crate::auras::has_aura(run.app.world(), actor, 386478));
            run.app
                .world_mut()
                .get_mut::<crate::spell_cast::powers::PowerPools>(actor)
                .unwrap()
                .get_mut(PowerType::Rage)
                .unwrap()
                .current = 1000.0;
            execute(
                run.app.world_mut(),
                actor,
                Execution::Spell(190456, actor, Some(target)),
            )
            .unwrap();
            let remaining = run
                .app
                .world()
                .get::<crate::spell_cast::powers::PowerPools>(actor)
                .unwrap()
                .get(PowerType::Rage)
                .unwrap()
                .current;
            let spent = (1000.0 - remaining) / units;
            assert!(spent > 0.0);
            paid += spent;
            for _ in 0..25 {
                run.app.update();
            }
        }
        assert!(crate::auras::has_aura(run.app.world(), actor, 386478));
        crate::auras::remove_auras(
            run.app.world_mut(),
            actor,
            crate::auras::RemoveReason::Consumed,
            |aura| aura.spell_id == 190456,
        );
        execute(
            run.app.world_mut(),
            actor,
            Execution::Spell(23922, target, Some(target)),
        )
        .unwrap();
        assert!(
            crate::auras::has_aura(run.app.world(), actor, 190456),
            "Shield Slam grants Ignore Pain"
        );
        assert!(
            !crate::auras::has_aura(run.app.world(), actor, 386478),
            "Shield Slam consumes outburst"
        );
    }
    #[test]
    #[ignore = "requires canonical MID2 Protection profile"]
    fn protection_demolish_channel_delivers_three_hits_and_selected_decimator() {
        use super::super::literal_rotation::execution::{Execution, execute};
        use shared::protocol::CombatLogKind;
        for selected in [false, true] {
            let mut run = super::super::prepare_measurement(73, 1).unwrap();
            super::super::engage(&mut run);
            let actor = run.actor;
            let target = run.targets[0];
            if !selected {
                crate::auras::remove_auras(
                    run.app.world_mut(),
                    actor,
                    crate::auras::RemoveReason::Consumed,
                    |aura| aura.spell_id == 1270704,
                );
            }
            execute(
                run.app.world_mut(),
                actor,
                Execution::Spell(436358, target, Some(target)),
            )
            .unwrap();
            for _ in 0..30 {
                run.app.update();
            }
            let events = &run
                .app
                .world()
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0;
            for spell in [440884, 440886, 440888] {
                assert!(
                    events.iter().any(|event| event.spell_id == Some(spell)
                        && event.kind == CombatLogKind::Damage
                        && event.amount > 0),
                    "missing Demolish hit {spell}"
                );
            }
            assert_eq!(
                crate::auras::has_aura(run.app.world(), target, 262115),
                selected,
                "only selected Decimator applies Deep Wounds"
            );
        }
    }
}
