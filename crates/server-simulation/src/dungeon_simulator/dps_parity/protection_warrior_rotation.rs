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
}
