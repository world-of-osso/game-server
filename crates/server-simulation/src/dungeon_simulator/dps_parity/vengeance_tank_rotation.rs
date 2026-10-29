//! Pinned MID2 Vengeance Demon Hunter action identities and native facts.
use super::literal_rotation::equipment::Equipment;
use crate::simc_apl_native::{Bindings, Encounter, NativeFacts};
use bevy::prelude::{Entity, World};
use server::simc_apl::{Action, ActionFacts, Facts};
use shared::components::PowerType;

// SimC 76f8118 sc_death_knight.cpp action constructors and buff lookups.
const ACTIONS: &[(&str, u32)] = &[
    ("demon_spikes", 203720),
    ("disrupt", 183752),
    ("fel_devastation", 212084),
    ("felblade", 232893),
    ("fiery_brand", 204021),
    ("fracture", 263642),
    ("immolation_aura", 258920),
    ("infernal_strike", 189110),
    ("metamorphosis", 187827),
    ("reavers_glaive", 442294),
    ("sigil_of_flame", 204596),
    ("sigil_of_spite", 390163),
    ("soul_carver", 207407),
    ("soul_cleave", 228477),
    ("spirit_bomb", 247454),
    ("throw_glaive", 204157),
];
const BUFFS: &[(&str, u32)] = &[
    ("demon_spikes", 203819),
    ("fiery_brand", 207771),
    ("metamorphosis", 187827),
    ("seething_anger", 1270547),
    ("untethered_rage", 1270476),
    ("voidfall_building", 1256301),
    ("voidfall_spending", 1256302),
    ("reavers_glaive", 444686),
    ("rending_strike", 442442),
    ("glaive_flurry", 442435),
];
const TALENTS: &[(&str, u32)] = &[
    ("fiery_demise", 389220),
    ("soul_sigils", 395446),
    ("fallout", 227174),
    ("charred_flesh", 336639),
    ("sigil_of_spite", 390163),
    ("soul_carver", 207407),
    ("untethered_rage_2", 1270448),
    ("untethered_rage_3", 1270449),
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
        talents: names(TALENTS),
        resources: [("fury".into(), PowerType::Fury)].into(),
        global_cooldown_spell: 263642,
        ..Default::default()
    }
}

pub(crate) struct Context<'a> {
    native: NativeFacts<'a>,
    equipment: &'a Equipment,
    world: &'a World,
    actor: Entity,
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
            world,
            actor,
        }
    }
}

impl Facts for Context<'_> {
    fn value(&self, name: &str) -> Result<f64, String> {
        if name.starts_with("trinket.") || name.starts_with("equipped.") {
            self.equipment.fact(name, &self.native)
        } else if name == "apex.2" {
            self.native.value("talent.untethered_rage_2.enabled")
        } else if name == "apex.3" {
            self.native.value("talent.untethered_rage_3.enabled")
        } else if name == "in_boss_encounter" {
            // SimC76f8118 player.cpp6659–6664: Patchwerk initializes this flag to one.
            match self.native.value("fight_style.dungeonroute")?
                + self.native.value("fight_style.dungeonslice")?
            {
                0.0 => Ok(1.0),
                _ => Err("NATIVE_DUNGEON_BOSS_ENCOUNTER_UNAVAILABLE".into()),
            }
        } else if name == "in_combat" {
            self.world
                .get::<shared::components::CombatStatus>(self.actor)
                .map(|status| f64::from(status.0 as u8))
                .ok_or("NATIVE_COMBAT_STATUS_UNAVAILABLE".into())
        } else if name == "soul_fragments" {
            Ok(server::soul_fragments::active_count(
                self.world,
                self.actor,
                server::soul_fragments::FragmentFilter::Any,
            ) as f64)
        } else if name == "soul_fragments.inactive" {
            Ok(server::soul_fragments::inactive_count(
                self.world,
                self.actor,
                server::soul_fragments::FragmentFilter::Any,
            ) as f64)
        } else if name == "soul_fragments.total" {
            Ok(server::soul_fragments::total_count(
                self.world,
                self.actor,
                server::soul_fragments::FragmentFilter::Any,
            ) as f64)
        } else if name == "raid_event.adds.has_boss" {
            if self.native.value("raid_event.adds.exists")? == 0.0 {
                Ok(0.0)
            } else {
                Err("NATIVE_ADD_EVENT_BOSS_METADATA_UNAVAILABLE".into())
            }
        } else if name == "action.sigil_of_spite.placed" {
            let now = crate::spell_cast::now(self.world);
            Ok(self.world.iter_entities().any(|entity| {
                entity
                    .get::<crate::spell_cast::area_trigger::SpellAreaTrigger>()
                    .is_some_and(|trigger| {
                        trigger.caster == self.actor
                            && trigger.spell_id == 390163
                            && trigger.expires_at.is_some_and(|expiry| now < expiry)
                    })
            }) as u8 as f64)
        } else {
            self.native.value(name)
        }
    }
}

impl ActionFacts for Context<'_> {
    fn value_for(&self, action: &Action, name: &str) -> Result<f64, String> {
        if name.starts_with("trinket.")
            || name.starts_with("equipped.")
            || name == "in_combat"
            || name == "in_boss_encounter"
            || name == "apex.2"
            || name == "apex.3"
            || name.starts_with("soul_fragments")
            || name == "action.sigil_of_spite.placed"
            || name == "raid_event.adds.has_boss"
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
        self.native.value("fury")?;
        Ok("fury".into())
    }
}

fn program() -> Result<server::simc_apl::Program, String> {
    let rows = include_str!("vengeance_tank_rotation/76f8118.simc")
        .lines()
        .map(|row| {
            let (head, action) = row
                .split_once('=')
                .ok_or("Vengeance APL lacks assignment")?;
            let head = head.strip_suffix('+').unwrap_or(head);
            let list = if head == "actions" {
                "default"
            } else {
                head.strip_prefix("actions.")
                    .ok_or("Vengeance APL lacks list")?
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
                    "demon_spikes" | "immolation_aura" | "fel_devastation" | "spirit_bomb"
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

pub(super) const REPORT_KEY: Option<&str> = Some("vengeance_tank_rotation");
impl super::SpecDriver for Rotation {
    fn label(&self) -> &'static str {
        "LITERAL_SIMC_VENGEANCE_76f8118"
    }
    fn tank_active_mitigation_auras(&self) -> Option<&'static [u32]> {
        Some(&[203819])
    }
    fn tank_resource_counters(
        &self,
        world: &World,
        actor: Entity,
    ) -> std::collections::BTreeMap<&'static str, u64> {
        let (generated, consumed) = server::soul_fragments::observation_counts(world, actor);
        [
            ("soul_fragments_generated", generated),
            ("soul_fragments_consumed", consumed),
        ]
        .into()
    }
    fn tank_target_debuff_auras(&self) -> &'static [u32] {
        &[207771, 204598]
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
    #[ignore = "requires canonical world database, MID2 Vengeance inputs and SimC tank trace"]
    fn vengeance_replays_mid2_tank_dummy_with_literal_rotation() {
        let trace =
            std::env::var("HEALER_TANK_SIMC_TRACE").expect("HEALER_TANK_SIMC_TRACE required");
        let report = super::super::super::tank_replay::replay_simc(
            581,
            std::path::Path::new(&trace),
            300,
            70448,
        )
        .unwrap();
        assert_eq!(report["spec"], 581);
        assert_eq!(report["level"], 90);
        assert_eq!(report["precombat_casts"]["204596"], 1, "{report}");
        assert_eq!(report["precombat_casts"]["258920"], 1, "{report}");
        assert!(report["incoming_events"].as_u64().unwrap() > 0, "{report}");
        assert!(report["raw_damage"].as_f64().unwrap() > 0.0, "{report}");
        assert!(
            report["active_mitigation_uptime"].as_f64().unwrap() > 0.0,
            "{report}"
        );
        assert!(
            report["tank_casts"]["203720"].as_u64().unwrap() > 0,
            "{report}"
        );
        assert!(
            report["tank_casts"]["228477"].as_u64().unwrap() > 0,
            "{report}"
        );
        assert!(
            report["outgoing_damage_by_spell"]["225919"]
                .as_i64()
                .unwrap()
                > 0,
            "{report}"
        );
        assert!(
            report["outgoing_damage_by_spell"]["189112"]
                .as_i64()
                .unwrap()
                > 0,
            "{report}"
        );
        assert!(
            report["resource_spent_by_spell"]["228477"]["fury"]
                .as_f64()
                .unwrap()
                >= 30.0,
            "{report}"
        );
        assert!(
            report["resource_counters"]["soul_fragments_generated"]
                .as_u64()
                .unwrap()
                > 0,
            "{report}"
        );
        assert!(
            report["resource_counters"]["soul_fragments_consumed"]
                .as_u64()
                .unwrap()
                > 0,
            "{report}"
        );
        assert!(
            report["target_debuff_uptime"]["207771"].as_f64().is_some(),
            "{report}"
        );
        assert!(
            report["target_debuff_uptime"]["204598"].as_f64().unwrap() > 0.0,
            "{report}"
        );
        println!("VENGEANCE_TANK_REPORT={report}");
    }
    #[test]
    #[ignore = "requires native Vengeance profile"]
    fn fiery_brand_impact_applies_target_and_caster_auras() {
        use super::super::*;
        let _seed = crate::random::seed_scope(70_448);
        let (profile, traits) = read_reference(581).unwrap();
        let identity = inputs::read_actor(&profile).unwrap();
        let mut run = prepare_reference(profile, &traits, 1, identity).unwrap();
        let actor = run.actor;
        let target = run.targets[0];
        crate::spell_cast::cast_for_player_with_target(
            run.app.world_mut(),
            actor,
            204021,
            Some(target),
        )
        .unwrap();
        for _ in 0..20 {
            run.app.update();
        }
        assert!(crate::auras::has_aura(run.app.world(), target, 207771));
        assert!(crate::auras::has_aura(run.app.world(), actor, 207771));
    }

    #[test]
    #[ignore = "requires native Vengeance profile"]
    fn fracture_weapon_impacts_each_create_native_lesser_fragment() {
        use super::super::*;
        use server::soul_fragments::{self, FragmentFilter};
        let _seed = crate::random::seed_scope(70_448);
        let (profile, traits) = read_reference(581).unwrap();
        let identity = inputs::read_actor(&profile).unwrap();
        let mut run = prepare_reference(profile, &traits, 1, identity).unwrap();
        let actor = run.actor;
        let target = run.targets[0];
        crate::spell_cast::cast_for_player_with_target(
            run.app.world_mut(),
            actor,
            263642,
            Some(target),
        )
        .unwrap();
        for _ in 0..20 {
            run.app.update();
        }
        let hits = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        for spell in [225919, 225921] {
            assert!(
                hits.iter()
                    .any(|event| event.spell_id == Some(spell) && event.amount > 0),
                "missing fracture hand {spell}"
            );
        }
        assert_eq!(
            soul_fragments::total_count(run.app.world(), actor, FragmentFilter::Lesser),
            2
        );
        assert_eq!(
            soul_fragments::observation_counts(run.app.world(), actor),
            (2, 0)
        );
        let consumed = soul_fragments::consume_active(
            run.app.world_mut(),
            actor,
            FragmentFilter::Lesser,
            1,
            true,
        )
        .unwrap();
        assert_eq!(consumed, 1);
        assert_eq!(
            soul_fragments::observation_counts(run.app.world(), actor),
            (2, 1)
        );
    }
    #[test]
    #[ignore = "requires native Vengeance profile"]
    fn fracture_metamorphosis_fragment_arrives_after_500ms() {
        use super::super::*;
        use server::soul_fragments::{self, FragmentFilter};
        let _seed = crate::random::seed_scope(70_448);
        let (profile, traits) = read_reference(581).unwrap();
        let identity = inputs::read_actor(&profile).unwrap();
        let mut run = prepare_reference(profile, &traits, 1, identity).unwrap();
        let actor = run.actor;
        let target = run.targets[0];
        crate::auras::apply_aura(
            run.app.world_mut(),
            &crate::auras::AuraApplication {
                caster: actor,
                target: actor,
                spell_id: 187827,
                effect_mask: u32::MAX,
                source: crate::auras::AuraSource::Cast,
            },
        );
        crate::spell_cast::cast_for_player_with_target(
            run.app.world_mut(),
            actor,
            263642,
            Some(target),
        )
        .unwrap();
        for _ in 0..3 {
            run.app.update();
        }
        assert_eq!(
            soul_fragments::total_count(run.app.world(), actor, FragmentFilter::Lesser),
            2
        );
        for _ in 0..10 {
            run.app.update();
        }
        assert_eq!(
            soul_fragments::total_count(run.app.world(), actor, FragmentFilter::Lesser),
            3
        );
    }
    #[test]
    #[ignore = "requires native Vengeance profile"]
    fn untethered_rage_modifies_earned_fragment_spenders() {
        use super::super::*;
        use server::soul_fragments::{self, FragmentFilter};
        fn spend(spell: u32, selected: bool) -> (u64, f64) {
            let _seed = crate::random::seed_scope(70_448);
            let (profile, mut traits) = read_reference(581).unwrap();
            if !selected {
                traits.retain(|trait_| trait_.entry_id != 137042);
            }
            let identity = inputs::read_actor(&profile).unwrap();
            let mut run = prepare_reference(profile, &traits, 1, identity).unwrap();
            let actor = run.actor;
            let target = run.targets[0];
            engage(&mut run);
            for _ in 0..3 {
                crate::spell_cast::cast_for_player_with_target(
                    run.app.world_mut(),
                    actor,
                    263642,
                    Some(target),
                )
                .unwrap();
                for _ in 0..55 {
                    run.app.update();
                }
            }
            assert_eq!(
                soul_fragments::active_count(run.app.world(), actor, FragmentFilter::Any),
                6
            );
            let before = soul_fragments::observation_counts(run.app.world(), actor).1;
            crate::spell_cast::cast_for_player_with_target(
                run.app.world_mut(),
                actor,
                spell,
                Some(target),
            )
            .unwrap();
            let consumed = soul_fragments::observation_counts(run.app.world(), actor).1 - before;
            for _ in 0..30 {
                run.app.update();
            }
            let child = if spell == 247454 { 247455 } else { 228478 };
            let event = run
                .app
                .world()
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0
                .iter()
                .find(|event| {
                    event.spell_id == Some(child)
                        && event.target == Some(target.to_bits())
                        && event.kind == shared::protocol::CombatLogKind::Damage
                })
                .unwrap();
            let amount = f64::from(event.amount) / if event.crit { 2.0 } else { 1.0 };
            (consumed, amount)
        }
        let (base_count, base_damage) = spend(247454, false);
        let (selected_count, selected_damage) = spend(247454, true);
        assert_eq!((base_count, selected_count), (5, 6));
        assert!(
            (selected_damage / base_damage - 1.4).abs() < 0.05,
            "Spirit Bomb: {base_damage}/{selected_damage}"
        );
        assert_eq!((spend(228477, false).0, spend(228477, true).0), (2, 3));
    }

    #[test]
    #[ignore = "requires native Vengeance profile"]
    fn spirit_bomb_consumes_earned_fragments_and_deals_delayed_damage() {
        use super::super::*;
        use server::soul_fragments::{self, FragmentFilter};
        let _seed = crate::random::seed_scope(70_448);
        let (profile, traits) = read_reference(581).unwrap();
        let identity = inputs::read_actor(&profile).unwrap();
        let mut run = prepare_reference(profile, &traits, 1, identity).unwrap();
        let actor = run.actor;
        let target = run.targets[0];
        for _ in 0..2 {
            crate::spell_cast::cast_for_player_with_target(
                run.app.world_mut(),
                actor,
                263642,
                Some(target),
            )
            .unwrap();
            for _ in 0..35 {
                run.app.update();
            }
        }
        let active = soul_fragments::active_count(run.app.world(), actor, FragmentFilter::Any);
        assert!(active >= 2);
        assert!(crate::auras::has_aura(run.app.world(), actor, 203981));
        crate::spell_cast::cast_for_player_with_target(
            run.app.world_mut(),
            actor,
            247454,
            Some(actor),
        )
        .unwrap();
        for _ in 0..5 {
            run.app.update();
        }
        assert_eq!(
            soul_fragments::active_count(run.app.world(), actor, FragmentFilter::Any),
            0
        );
        assert!(!crate::auras::has_aura(run.app.world(), actor, 203981));
        let has_damage = |run: &Run| {
            run.app
                .world()
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0
                .iter()
                .any(|event| {
                    event.spell_id == Some(247455)
                        && event.amount > 0
                        && event.kind == shared::protocol::CombatLogKind::Damage
                })
        };
        assert!(!has_damage(&run));
        for _ in 0..25 {
            run.app.update();
        }
        assert!(
            has_damage(&run),
            "native Spirit Bomb child must damage after its delay"
        );
    }
    #[test]
    #[ignore = "requires native Vengeance profile"]
    fn fracture_weapon_impacts_produce_fragments_through_absorb() {
        use super::super::*;
        use server::soul_fragments::{self, FragmentFilter};
        let _seed = crate::random::seed_scope(70_448);
        let (profile, traits) = read_reference(581).unwrap();
        let identity = inputs::read_actor(&profile).unwrap();
        let mut run = prepare_reference(profile, &traits, 1, identity).unwrap();
        let actor = run.actor;
        let target = run.targets[0];
        crate::auras::apply_aura(
            run.app.world_mut(),
            &crate::auras::AuraApplication {
                caster: actor,
                target,
                spell_id: 69787,
                effect_mask: u32::MAX,
                source: crate::auras::AuraSource::Cast,
            },
        );
        crate::spell_cast::cast_for_player_with_target(
            run.app.world_mut(),
            actor,
            263642,
            Some(target),
        )
        .unwrap();
        for _ in 0..20 {
            run.app.update();
        }
        let hits = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        assert!(
            hits.iter()
                .any(|event| matches!(event.spell_id, Some(225919 | 225921)) && event.absorbed > 0),
            "native shield must absorb a Fracture impact: {:?}",
            hits.iter()
                .filter(|event| matches!(event.spell_id, Some(225919 | 225921)))
                .collect::<Vec<_>>()
        );
        assert_eq!(
            soul_fragments::total_count(run.app.world(), actor, FragmentFilter::Lesser),
            2
        );
    }
    #[test]
    #[ignore = "requires native Vengeance profile"]
    fn soul_cleave_consumes_earned_fragments_before_delayed_damage() {
        use super::super::*;
        use server::soul_fragments::{self, FragmentFilter};
        let _seed = crate::random::seed_scope(70_448);
        let (profile, traits) = read_reference(581).unwrap();
        let identity = inputs::read_actor(&profile).unwrap();
        let mut run = prepare_reference(profile, &traits, 1, identity).unwrap();
        let actor = run.actor;
        let target = run.targets[0];
        crate::spell_cast::cast_for_player_with_target(
            run.app.world_mut(),
            actor,
            263642,
            Some(target),
        )
        .unwrap();
        for _ in 0..35 {
            run.app.update();
        }
        assert_eq!(
            soul_fragments::active_count(run.app.world(), actor, FragmentFilter::Any),
            2
        );
        crate::spell_cast::cast_for_player_with_target(
            run.app.world_mut(),
            actor,
            228477,
            Some(target),
        )
        .unwrap();
        run.app.update();
        assert_eq!(
            soul_fragments::active_count(run.app.world(), actor, FragmentFilter::Any),
            0
        );
        assert!(
            crate::auras::has_aura(run.app.world(), actor, 1270547)
                || crate::auras::has_aura(run.app.world(), actor, 1270476),
            "consumption must earn native Untethered Rage or Seething Anger"
        );
        let count = |run: &Run| {
            run.app
                .world()
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0
                .iter()
                .filter(|event| {
                    event.spell_id == Some(228478)
                        && event.kind == shared::protocol::CombatLogKind::Damage
                })
                .count()
        };
        assert_eq!(count(&run), 0, "damage child must wait for native delay");
        for _ in 0..10 {
            run.app.update();
        }
        assert_eq!(count(&run), 1, "exactly one delayed damage child");
    }
    #[test]
    #[ignore = "requires native Vengeance profile"]
    fn vengeful_beast_extends_hardcast_metamorphosis() {
        use super::super::*;
        fn duration(selected: bool) -> f64 {
            let (profile, mut traits) = read_reference(581).unwrap();
            if !selected {
                traits.retain(|trait_| trait_.entry_id != 112899);
            }
            let identity = inputs::read_actor(&profile).unwrap();
            let mut run = prepare_reference(profile, &traits, 1, identity).unwrap();
            let actor = run.actor;
            let target = run.targets[0];
            engage(&mut run);
            crate::spell_cast::cast_for_player_with_target(
                run.app.world_mut(),
                actor,
                187827,
                Some(target),
            )
            .unwrap();
            let now = crate::spell_cast::now(run.app.world());
            run.app
                .world()
                .get::<crate::auras::ActiveAuras>(actor)
                .unwrap()
                .iter()
                .find(|aura| aura.spell_id == 187827)
                .unwrap()
                .expires_at
                .unwrap()
                - now
        }
        let base = duration(false);
        let extended = duration(true);
        assert!(
            (base - 15.0).abs() < 0.05 && (extended - 20.0).abs() < 0.05,
            "base {base}, selected {extended}"
        );
    }

    #[test]
    #[ignore = "requires native Vengeance profile"]
    fn fiery_demise_increases_fire_damage_to_branded_target() {
        use super::super::*;
        fn meteor(selected: bool) -> i32 {
            let _seed = crate::random::seed_scope(70_448);
            let (profile, mut traits) = read_reference(581).unwrap();
            if !selected {
                traits.retain(|trait_| trait_.entry_id != 112872);
            }
            let identity = inputs::read_actor(&profile).unwrap();
            let mut run = prepare_reference(profile, &traits, 1, identity).unwrap();
            let actor = run.actor;
            let target = run.targets[0];
            engage(&mut run);
            crate::spell_cast::cast_for_player_with_target(
                run.app.world_mut(),
                actor,
                204021,
                Some(target),
            )
            .unwrap();
            for _ in 0..5 {
                run.app.update();
            }
            assert!(crate::auras::has_aura(run.app.world(), target, 207771));
            let position = crate::spell_cast::targets::position(run.app.world(), target).unwrap();
            crate::spell_cast::cast_triggered_at_destination(
                run.app.world_mut(),
                actor,
                1256306,
                position,
                1,
            );
            run.app
                .world()
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0
                .iter()
                .find(|event| {
                    event.spell_id == Some(1256306)
                        && event.target == Some(target.to_bits())
                        && event.kind == shared::protocol::CombatLogKind::Damage
                })
                .unwrap()
                .amount
        }
        let without = meteor(false);
        let with = meteor(true);
        assert!(
            (with as f64 / without as f64 - 1.3).abs() < 0.01,
            "rank 2 Fiery Demise: {without}/{with}"
        );
    }

    #[test]
    #[ignore = "requires native Vengeance profile"]
    fn otherworldly_focus_scales_real_meteor_recipients() {
        use super::super::*;
        fn meteor(selected: bool, count: usize) -> i32 {
            let _seed = crate::random::seed_scope(70_448);
            let (profile, mut traits) = read_reference(581).unwrap();
            if !selected {
                traits.retain(|trait_| trait_.entry_id != 135661);
            }
            let identity = inputs::read_actor(&profile).unwrap();
            let mut run = prepare_reference(profile, &traits, count, identity).unwrap();
            let actor = run.actor;
            let target = run.targets[0];
            engage(&mut run);
            let position = crate::spell_cast::targets::position(run.app.world(), target).unwrap();
            crate::spell_cast::cast_triggered_at_destination(
                run.app.world_mut(),
                actor,
                1256306,
                position,
                1,
            );
            run.app
                .world()
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0
                .iter()
                .find(|event| {
                    event.spell_id == Some(1256306)
                        && event.target == Some(target.to_bits())
                        && event.kind == shared::protocol::CombatLogKind::Damage
                })
                .unwrap()
                .amount
        }
        for (count, expected) in [(1, 1.35), (2, 1.30)] {
            let without = meteor(false, count);
            let with = meteor(true, count);
            assert!(
                (with as f64 / without as f64 - expected).abs() < 0.01,
                "actual {count}-target damage: {without}/{with}"
            );
        }
    }

    #[test]
    #[ignore = "requires native Vengeance profile"]
    fn selected_vulnerability_increases_damage_to_frailty_target() {
        use super::super::*;
        fn second_hit(selected: bool) -> i32 {
            let _seed = crate::random::seed_scope(70_448);
            let (profile, mut traits) = read_reference(581).unwrap();
            if !selected {
                traits.retain(|trait_| trait_.entry_id != 112897);
            }
            let identity = inputs::read_actor(&profile).unwrap();
            let mut run = prepare_reference(profile, &traits, 1, identity).unwrap();
            let actor = run.actor;
            let target = run.targets[0];
            engage(&mut run);
            for _ in 0..2 {
                assert!(crate::spell_cast::try_cast_triggered(
                    run.app.world_mut(),
                    actor,
                    228478,
                    Some(target),
                    1
                ));
            }
            run.app
                .world()
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0
                .iter()
                .filter(|event| {
                    event.spell_id == Some(228478)
                        && event.target == Some(target.to_bits())
                        && event.kind == shared::protocol::CombatLogKind::Damage
                })
                .nth(1)
                .unwrap()
                .amount
        }
        let without = second_hit(false);
        let with = second_hit(true);
        assert!(
            (with as f64 / without as f64 - 1.12).abs() < 0.01,
            "selected Vulnerability and Soulcrush must grant 12% per existing Frailty stack: {without}/{with}"
        );
    }

    #[test]
    #[ignore = "requires native Vengeance profile"]
    fn focused_cleave_only_increases_the_real_primary_target() {
        use super::super::*;
        fn damage(selected: bool) -> [i32; 2] {
            let _seed = crate::random::seed_scope(70_448);
            let (profile, mut traits) = read_reference(581).unwrap();
            if !selected {
                traits.retain(|trait_| trait_.entry_id != 112891);
            }
            let identity = inputs::read_actor(&profile).unwrap();
            let mut run = prepare_reference(profile, &traits, 2, identity).unwrap();
            let actor = run.actor;
            let targets = [run.targets[0], run.targets[1]];
            engage(&mut run);
            assert!(crate::spell_cast::try_cast_triggered(
                run.app.world_mut(),
                actor,
                228478,
                Some(targets[0]),
                1
            ));
            let events = &run
                .app
                .world()
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0;
            targets.map(|target| {
                events
                    .iter()
                    .filter(|event| {
                        event.spell_id == Some(228478)
                            && event.target == Some(target.to_bits())
                            && event.kind == shared::protocol::CombatLogKind::Damage
                    })
                    .map(|event| event.amount)
                    .sum()
            })
        }
        let baseline = damage(false);
        let selected = damage(true);
        assert!(
            baseline[0] > 0 && baseline[1] > 0,
            "both real recipients must be hit"
        );
        assert!(
            (selected[0] as f64 / baseline[0] as f64 - 1.6).abs() < 0.01,
            "primary: {baseline:?} {selected:?}"
        );
        assert_eq!(
            selected[1], baseline[1],
            "secondary: {baseline:?} {selected:?}"
        );
    }

    #[test]
    #[ignore = "requires native Vengeance profile"]
    fn vengeance_mid2_four_piece_doubles_damage_on_real_sigil_target() {
        use super::super::*;
        fn hit(spell: u32, equipped: bool, sigil: bool) -> f64 {
            let _seed = crate::random::seed_scope(70_448);
            let (profile, traits) = read_reference(581).unwrap();
            let identity = inputs::read_actor(&profile).unwrap();
            let mut run = prepare_reference(profile, &traits, 1, identity).unwrap();
            let actor = run.actor;
            let target = run.targets[0];
            engage(&mut run);
            assert!(crate::auras::has_aura(run.app.world(), actor, 1296614));
            if !equipped {
                crate::auras::remove_auras(
                    run.app.world_mut(),
                    actor,
                    crate::auras::RemoveReason::Cancelled,
                    |aura| aura.spell_id == 1296614,
                );
            }
            if sigil {
                assert!(crate::spell_cast::try_cast_triggered(
                    run.app.world_mut(),
                    actor,
                    204598,
                    Some(target),
                    1
                ));
                assert!(crate::auras::has_aura(run.app.world(), target, 204598));
            }
            assert!(crate::spell_cast::try_cast_triggered(
                run.app.world_mut(),
                actor,
                spell,
                Some(target),
                1
            ));
            let event = run
                .app
                .world()
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0
                .iter()
                .find(|event| {
                    event.spell_id == Some(spell)
                        && event.target == Some(target.to_bits())
                        && event.kind == shared::protocol::CombatLogKind::Damage
                })
                .unwrap();
            f64::from(event.amount) / if event.crit { 2.0 } else { 1.0 }
        }
        for spell in [258921, 258922, 389860] {
            let without_tier = hit(spell, false, true);
            let with_tier = hit(spell, true, true);
            assert!(
                (with_tier / without_tier - 2.0).abs() < 0.02,
                "spell {spell}: {without_tier}/{with_tier}"
            );
            let without_sigil = hit(spell, false, false);
            let tier_without_sigil = hit(spell, true, false);
            assert!(
                (tier_without_sigil / without_sigil - 1.0).abs() < 0.02,
                "spell {spell} without Sigil: {without_sigil}/{tier_without_sigil}"
            );
        }
    }

    #[test]
    #[ignore = "requires native Vengeance profile"]
    fn infernal_armor_and_retaliation_require_real_eligible_hits() {
        use super::super::super::healer_intake::{Hit, HitKind};
        use super::super::*;
        let _seed = crate::random::seed_scope(70_448);
        let (profile, traits) = read_reference(581).unwrap();
        let identity = inputs::read_actor(&profile).unwrap();
        let mut run = prepare_reference(profile, &traits, 1, identity).unwrap();
        let actor = run.actor;
        let attacker = run.targets[0];
        engage(&mut run);
        let count = |run: &Run, spell| {
            run.app
                .world()
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0
                .iter()
                .filter(|event| {
                    event.spell_id == Some(spell)
                        && event.source == Some(actor.to_bits())
                        && event.target == Some(attacker.to_bits())
                        && event.kind == shared::protocol::CombatLogKind::Damage
                })
                .count()
        };
        crate::spell_cast::effects::replay_simc_dummy_strike(
            run.app.world_mut(),
            attacker,
            actor,
            1000.0,
            false,
            false,
        );
        assert_eq!((count(&run, 320334), count(&run, 391159)), (0, 0));
        crate::spell_cast::cast_for_player_with_target(
            run.app.world_mut(),
            actor,
            258920,
            Some(actor),
        )
        .unwrap();
        for _ in 0..30 {
            run.app.update();
        }
        crate::spell_cast::cast_for_player_with_target(
            run.app.world_mut(),
            actor,
            203720,
            Some(actor),
        )
        .unwrap();
        for _ in 0..3 {
            run.app.update();
        }
        assert!(crate::auras::has_aura(run.app.world(), actor, 258920));
        assert!(crate::auras::has_aura(run.app.world(), actor, 203819));
        let before = (count(&run, 320334), count(&run, 391159));
        crate::spell_cast::effects::replay_simc_dummy_bleed(
            run.app.world_mut(),
            attacker,
            actor,
            1000.0,
        );
        assert_eq!((count(&run, 320334), count(&run, 391159)), before);
        crate::spell_cast::effects::replay_simc_dummy_strike(
            run.app.world_mut(),
            attacker,
            actor,
            1000.0,
            false,
            false,
        );
        assert_eq!(
            (count(&run, 320334), count(&run, 391159)),
            (before.0 + 1, before.1)
        );
        let hit = Hit {
            tick: 1,
            source_guid: 0,
            pack: 0,
            target_slot: 0,
            school: 1,
            raw: 1000.0,
            level_scaling_factor: 1.0,
            kind: HitKind::Melee {
                hand: 0,
                outcome: "Hit".into(),
                level_difference: 0,
                critical_multiplier: 1.0,
                weapon_speed: 2.0,
            },
        };
        crate::combat::replay_healer_hit(run.app.world_mut(), attacker, actor, &hit).unwrap();
        assert_eq!(
            (count(&run, 320334), count(&run, 391159)),
            (before.0 + 2, before.1 + 1)
        );
    }

    #[test]
    #[ignore = "requires native Vengeance profile"]
    fn vengeance_mid2_soul_cleave_extends_real_sigil() {
        use super::super::*;
        let _seed = crate::random::seed_scope(70_448);
        let (profile, traits) = read_reference(581).unwrap();
        let identity = inputs::read_actor(&profile).unwrap();
        let mut run = prepare_reference(profile, &traits, 1, identity).unwrap();
        let actor = run.actor;
        let target = run.targets[0];
        engage(&mut run);
        assert!(crate::auras::has_aura(run.app.world(), actor, 1296613));
        assert!(crate::spell_cast::try_cast_triggered(
            run.app.world_mut(),
            actor,
            204598,
            Some(target),
            1
        ));
        crate::spell_cast::cast_for_player_with_target(
            run.app.world_mut(),
            actor,
            263642,
            Some(target),
        )
        .unwrap();
        for _ in 0..35 {
            run.app.update();
        }
        let expiry = |run: &Run| {
            run.app
                .world()
                .get::<crate::auras::ActiveAuras>(target)
                .unwrap()
                .iter()
                .find(|aura| aura.spell_id == 204598 && aura.caster == actor)
                .unwrap()
                .expires_at
                .unwrap()
        };
        let before = expiry(&run);
        crate::spell_cast::cast_for_player_with_target(
            run.app.world_mut(),
            actor,
            228477,
            Some(target),
        )
        .unwrap();
        for _ in 0..10 {
            run.app.update();
        }
        assert!(
            (expiry(&run) - before - 2.0).abs() < 0.001,
            "equipped MID2 must extend the actual Sigil by 2s"
        );
    }

    #[test]
    #[ignore = "requires native Vengeance profile"]
    fn charred_flesh_extends_real_brand_and_sigil_on_immolation_hit() {
        use super::super::*;
        let _seed = crate::random::seed_scope(70_448);
        let (profile, traits) = read_reference(581).unwrap();
        let identity = inputs::read_actor(&profile).unwrap();
        let mut run = prepare_reference(profile, &traits, 1, identity).unwrap();
        let actor = run.actor;
        let target = run.targets[0];
        engage(&mut run);
        crate::spell_cast::cast_for_player_with_target(
            run.app.world_mut(),
            actor,
            204021,
            Some(target),
        )
        .unwrap();
        assert!(crate::spell_cast::try_cast_triggered(
            run.app.world_mut(),
            actor,
            204598,
            Some(target),
            1
        ));
        for _ in 0..5 {
            run.app.update();
        }
        let expiry = |run: &Run, id| {
            run.app
                .world()
                .get::<crate::auras::ActiveAuras>(target)
                .unwrap()
                .iter()
                .find(|aura| aura.spell_id == id && aura.caster == actor)
                .unwrap()
                .expires_at
                .unwrap()
        };
        let before = [expiry(&run, 207771), expiry(&run, 204598)];
        assert!(crate::spell_cast::try_cast_triggered(
            run.app.world_mut(),
            actor,
            258921,
            None,
            1
        ));
        for (index, id) in [207771, 204598].into_iter().enumerate() {
            assert!(
                (expiry(&run, id) - before[index] - 0.5).abs() < 0.001,
                "rank 2 Charred Flesh must extend {id} by 500ms"
            );
        }
    }

    #[test]
    #[ignore = "requires native Vengeance profile"]
    fn metamorphosis_and_spirit_bomb_earn_meteor_shower() {
        use super::super::*;
        let _seed = crate::random::seed_scope(70_448);
        let (profile, traits) = read_reference(581).unwrap();
        let identity = inputs::read_actor(&profile).unwrap();
        let mut run = prepare_reference(profile, &traits, 1, identity).unwrap();
        let actor = run.actor;
        let target = run.targets[0];
        engage(&mut run);
        for _ in 0..2 {
            crate::spell_cast::cast_for_player_with_target(
                run.app.world_mut(),
                actor,
                263642,
                Some(target),
            )
            .unwrap();
            for _ in 0..35 {
                run.app.update();
            }
        }
        crate::spell_cast::cast_for_player_with_target(
            run.app.world_mut(),
            actor,
            187827,
            Some(actor),
        )
        .unwrap();
        for _ in 0..25 {
            run.app.update();
        }
        assert!(crate::auras::has_aura(run.app.world(), actor, 1256308));
        literal_rotation::execution::execute(
            run.app.world_mut(),
            actor,
            literal_rotation::execution::Execution::Spell(247454, actor, Some(target)),
        )
        .unwrap();
        for _ in 0..160 {
            run.app.update();
        }
        assert!(!crate::auras::has_aura(run.app.world(), actor, 1256308));
        let hits = run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| {
                event.spell_id == Some(1264130)
                    && event.target == Some(target.to_bits())
                    && event.kind == shared::protocol::CombatLogKind::Damage
            })
            .count();
        assert_eq!(
            hits, 12,
            "earned shower must deal all native pulses to the real target"
        );
    }

    #[test]
    #[ignore = "requires native Vengeance profile"]
    fn catastrophe_rolls_damage_from_real_meteor_impacts() {
        use super::super::*;
        fn damage(meteors: usize) -> i32 {
            let _seed = crate::random::seed_scope(70_448);
            let (profile, traits) = read_reference(581).unwrap();
            let identity = inputs::read_actor(&profile).unwrap();
            let mut run = prepare_reference(profile, &traits, 1, identity).unwrap();
            let actor = run.actor;
            let target = run.targets[0];
            engage(&mut run);
            let position = crate::spell_cast::targets::position(run.app.world(), target).unwrap();
            for _ in 0..meteors {
                crate::spell_cast::cast_triggered_at_destination(
                    run.app.world_mut(),
                    actor,
                    1256306,
                    position,
                    1,
                );
                for _ in 0..5 {
                    run.app.update();
                }
            }
            assert!(crate::auras::has_aura(run.app.world(), target, 1256667));
            for _ in 0..240 {
                run.app.update();
            }
            run.app
                .world()
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0
                .iter()
                .filter(|event| {
                    event.spell_id == Some(1256667)
                        && event.periodic
                        && event.kind == shared::protocol::CombatLogKind::Damage
                })
                .map(|event| event.amount)
                .sum()
        }
        let one = damage(1);
        let two = damage(2);
        assert!(
            one > 0 && two > one,
            "two actual impacts must deposit more rolling damage: {one}/{two}"
        );
    }

    #[test]
    #[ignore = "requires native Vengeance profile"]
    fn fallout_initial_immolation_hit_creates_a_lesser_fragment() {
        use super::super::*;
        use server::soul_fragments::{self, FragmentFilter};
        let _seed = crate::random::seed_scope(70_448);
        let (profile, traits) = read_reference(581).unwrap();
        let identity = inputs::read_actor(&profile).unwrap();
        let mut run = prepare_reference(profile, &traits, 1, identity).unwrap();
        let actor = run.actor;
        engage(&mut run);
        for _ in 0..5 {
            run.app.update();
        }
        let before = soul_fragments::total_count(run.app.world(), actor, FragmentFilter::Lesser);
        crate::spell_cast::cast_for_player_with_target(
            run.app.world_mut(),
            actor,
            258920,
            Some(actor),
        )
        .unwrap();
        for _ in 0..5 {
            run.app.update();
        }
        let after = soul_fragments::total_count(run.app.world(), actor, FragmentFilter::Lesser);
        assert_eq!(
            after,
            before + 1,
            "single-target Fallout must generate one real fragment"
        );
        assert!(
            run.app
                .world()
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0
                .iter()
                .any(|event| event.spell_id == Some(258921)
                    && event.kind == shared::protocol::CombatLogKind::Damage)
        );
    }

    #[test]
    #[ignore = "requires native Vengeance profile"]
    fn partial_meteoric_fall_spends_one_stack_as_normal_meteor() {
        use super::super::*;
        use crate::auras::{AuraApplication, AuraSource};
        let _seed = crate::random::seed_scope(70_448);
        let (profile, traits) = read_reference(581).unwrap();
        let identity = inputs::read_actor(&profile).unwrap();
        let mut run = prepare_reference(profile, &traits, 1, identity).unwrap();
        let actor = run.actor;
        let target = run.targets[0];
        let selected = run
            .app
            .world()
            .get::<crate::class_progression::TraitLoadout>(actor)
            .unwrap();
        for id in [1253391, 1256353] {
            assert!(
                selected
                    .active_talent_spells()
                    .any(|talent| talent.spell_id == id),
                "missing talent {id}"
            );
        }
        engage(&mut run);
        crate::auras::apply_aura(
            run.app.world_mut(),
            &AuraApplication {
                caster: actor,
                target: actor,
                spell_id: 1256302,
                effect_mask: u32::MAX,
                source: AuraSource::Cast,
            },
        );
        assert!(crate::auras::set_self_cast_stacks(
            run.app.world_mut(),
            actor,
            1256302,
            1
        ));
        assert!(crate::spell_cast::try_cast_triggered(
            run.app.world_mut(),
            actor,
            228478,
            Some(target),
            1
        ));
        for _ in 0..30 {
            run.app.update();
        }
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        assert!(
            events
                .iter()
                .any(|event| event.spell_id == Some(1256306) && event.amount > 0),
            "normal meteor must land"
        );
        assert!(
            !events
                .iter()
                .any(|event| event.spell_id == Some(1256617) && event.amount > 0),
            "partial spending must not launch World Killer"
        );
    }

    #[test]
    #[ignore = "requires native Vengeance profile"]
    fn overlapping_spenders_do_not_cancel_scheduled_world_killer() {
        use super::super::*;
        let _seed = crate::random::seed_scope(70_448);
        let (profile, traits) = read_reference(581).unwrap();
        let identity = inputs::read_actor(&profile).unwrap();
        let mut run = prepare_reference(profile, &traits, 1, identity).unwrap();
        let actor = run.actor;
        let target = run.targets[0];
        engage(&mut run);
        crate::spell_cast::cast_for_player_with_target(
            run.app.world_mut(),
            actor,
            263642,
            Some(target),
        )
        .unwrap();
        for _ in 0..35 {
            run.app.update();
        }
        crate::spell_cast::cast_for_player_with_target(
            run.app.world_mut(),
            actor,
            187827,
            Some(actor),
        )
        .unwrap();
        for _ in 0..25 {
            run.app.update();
        }
        assert!(crate::auras::has_aura(run.app.world(), actor, 1256302));
        for _ in 0..5 {
            assert!(crate::spell_cast::try_cast_triggered(
                run.app.world_mut(),
                actor,
                228478,
                Some(target),
                1
            ));
            run.app.update();
        }
        assert!(
            !crate::auras::has_aura(run.app.world(), actor, 1256302),
            "other spenders must exhaust the stacks before the scheduled World Killer"
        );
        let final_hour_stacks = |run: &Run| {
            run.app
                .world()
                .get::<crate::auras::ActiveAuras>(actor)
                .and_then(|auras| {
                    auras
                        .iter()
                        .find(|aura| aura.spell_id == 1256322 && aura.caster == actor)
                })
                .map_or(0, |aura| aura.stacks)
        };
        assert_eq!(final_hour_stacks(&run), 3, "three consumed stacks persist");
        assert!(
            !run.app
                .world()
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0
                .iter()
                .any(|event| event.spell_id == Some(1256617)
                    && event.kind == shared::protocol::CombatLogKind::Damage),
            "World Killer must still be pending"
        );
        for _ in 0..50 {
            run.app.update();
        }
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        assert!(
            events
                .iter()
                .filter(|event| event.spell_id == Some(1256617)
                    && event.kind == shared::protocol::CombatLogKind::Damage
                    && event.amount > 0)
                .count()
                == 1,
            "the scheduled World Killer must land without a duplicate from partial spending"
        );
        assert_eq!(
            final_hour_stacks(&run),
            3,
            "zero-stack scheduled meteors must not grant Final Hour"
        );
    }

    #[test]
    #[ignore = "requires native Vengeance profile"]
    fn mass_acceleration_earns_voidfall_and_soul_cleave_spends_it() {
        use super::super::*;
        let _seed = crate::random::seed_scope(70_448);
        let (profile, traits) = read_reference(581).unwrap();
        let identity = inputs::read_actor(&profile).unwrap();
        let mut run = prepare_reference(profile, &traits, 1, identity).unwrap();
        let actor = run.actor;
        let target = run.targets[0];
        crate::spell_cast::cast_for_player_with_target(
            run.app.world_mut(),
            actor,
            263642,
            Some(target),
        )
        .unwrap();
        for _ in 0..35 {
            run.app.update();
        }
        crate::spell_cast::cast_for_player_with_target(
            run.app.world_mut(),
            actor,
            187827,
            Some(actor),
        )
        .unwrap();
        for _ in 0..25 {
            run.app.update();
        }
        assert!(
            crate::auras::has_aura(run.app.world(), actor, 1256302),
            "Meta must earn Voidfall spending"
        );
        let meta_remaining = |run: &Run| {
            let world = run.app.world();
            let spell = world
                .resource::<crate::spell_info::SpellStore>()
                .get(187827)
                .unwrap();
            let bindings = super::bindings();
            let encounter = crate::simc_apl_native::Encounter::Fixture {
                started_at: 0.0,
                duration: 300.0,
                style: crate::simc_apl_native::FightStyle::Patchwerk,
            };
            crate::simc_apl_native::NativeFacts::new(world, actor, &bindings, encounter, 1)
                .cooldown_for_spell(spell, "full_recharge_time")
                .unwrap()
        };
        let before = meta_remaining(&run);
        crate::spell_cast::cast_for_player_with_target(
            run.app.world_mut(),
            actor,
            228477,
            Some(target),
        )
        .unwrap();
        for _ in 0..50 {
            run.app.update();
        }
        assert!(
            !crate::auras::has_aura(run.app.world(), actor, 1256302),
            "Soul Cleave must spend the earned stacks"
        );
        assert!(
            crate::auras::has_aura(run.app.world(), actor, 1256322),
            "spent meteors earn Final Hour"
        );
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        assert!(
            events.iter().any(|event| event.spell_id == Some(1256306)
                && event.kind == shared::protocol::CombatLogKind::Damage
                && event.amount > 0),
            "earned meteor must damage the target"
        );
        assert!(
            before - meta_remaining(&run) > 10.0,
            "World Killer must reduce the real Meta cooldown on impact: before={before} after={} events={:?}",
            meta_remaining(&run),
            events
                .iter()
                .filter(|event| matches!(event.spell_id, Some(1256306 | 1256617)))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    #[ignore = "requires native Vengeance profile"]
    fn mass_acceleration_resets_spirit_bomb_after_metamorphosis() {
        use super::super::*;
        let (profile, traits) = read_reference(581).unwrap();
        let identity = inputs::read_actor(&profile).unwrap();
        let mut run = prepare_reference(profile, &traits, 1, identity).unwrap();
        let actor = run.actor;
        crate::spell_cast::scripts::start_hasted_cooldown(
            run.app.world_mut(),
            actor,
            247454,
            10_000.0,
        );
        let remaining = |run: &Run| {
            let world = run.app.world();
            let spell = world
                .resource::<crate::spell_info::SpellStore>()
                .get(247454)
                .unwrap();
            world
                .get::<crate::spell_cast::SpellHistory>(actor)
                .unwrap()
                .remaining_cooldown_ms(spell, crate::spell_cast::now(world))
        };
        assert!(remaining(&run) > 0);
        crate::spell_cast::cast_for_player_with_target(
            run.app.world_mut(),
            actor,
            187827,
            Some(actor),
        )
        .unwrap();
        for _ in 0..3 {
            run.app.update();
        }
        assert_eq!(
            remaining(&run),
            0,
            "selected Mass Acceleration must reset the active Spirit Bomb cooldown"
        );
    }

    #[test]
    #[ignore = "requires native Vengeance profile"]
    fn untethered_meta_uses_native_short_duration_and_consumes_proc() {
        use super::super::*;
        let (profile, traits) = read_reference(581).unwrap();
        let identity = inputs::read_actor(&profile).unwrap();
        let mut run = prepare_reference(profile, &traits, 1, identity).unwrap();
        let actor = run.actor;
        crate::auras::apply_aura(
            run.app.world_mut(),
            &crate::auras::AuraApplication {
                caster: actor,
                target: actor,
                spell_id: 1270476,
                effect_mask: u32::MAX,
                source: crate::auras::AuraSource::Cast,
            },
        );
        crate::spell_cast::cast_for_player_with_target(
            run.app.world_mut(),
            actor,
            187827,
            Some(actor),
        )
        .unwrap();
        for _ in 0..3 {
            run.app.update();
        }
        let aura = run
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(actor)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 187827)
            .unwrap();
        assert_eq!(aura.duration_ms, 10_000);
        assert!(!crate::auras::has_aura(run.app.world(), actor, 1270476));
    }
    #[test]
    #[ignore = "requires native Vengeance profile"]
    fn untethered_meta_extends_existing_native_metamorphosis() {
        use super::super::*;
        let (profile, traits) = read_reference(581).unwrap();
        let identity = inputs::read_actor(&profile).unwrap();
        let mut run = prepare_reference(profile, &traits, 1, identity).unwrap();
        let actor = run.actor;
        for spell_id in [187827, 1270476] {
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
        let old = run
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(actor)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 187827)
            .unwrap()
            .expires_at
            .unwrap();
        crate::spell_cast::cast_for_player_with_target(
            run.app.world_mut(),
            actor,
            187827,
            Some(actor),
        )
        .unwrap();
        for _ in 0..3 {
            run.app.update();
        }
        let new = run
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(actor)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 187827)
            .unwrap()
            .expires_at
            .unwrap();
        assert!((new - old - 10.0).abs() < 0.11, "old={old} new={new}");
    }
    #[test]
    #[ignore = "requires native Vengeance profile and canonical DB"]
    fn vengeance_felblade_deals_child_damage() {
        use super::super::*;
        let _seed = crate::random::seed_scope(70_448);
        let (profile, traits) = read_reference(581).unwrap();
        let identity = inputs::read_actor(&profile).unwrap();
        let mut run = prepare_reference(profile, &traits, 1, identity).unwrap();
        let actor = run.actor;
        let target = run.targets[0];
        crate::spell_cast::cast_for_player_with_target(
            run.app.world_mut(),
            actor,
            232893,
            Some(target),
        )
        .unwrap();
        for _ in 0..20 {
            run.app.update();
        }
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        assert!(
            events.iter().any(|event| event.spell_id == Some(213243)
                && event.kind == shared::protocol::CombatLogKind::Damage
                && event.amount > 0),
            "Felblade must execute its damage child"
        );
    }

    #[test]
    #[ignore = "requires native Vengeance profile and canonical DB"]
    fn frailty_heals_after_damage_to_affected_target() {
        use super::super::*;
        let _seed = crate::random::seed_scope(70_448);
        let (profile, traits) = read_reference(581).unwrap();
        let identity = inputs::read_actor(&profile).unwrap();
        let mut run = prepare_reference(profile, &traits, 1, identity).unwrap();
        let actor = run.actor;
        let target = run.targets[0];
        engage(&mut run);
        for _ in 0..5 {
            run.app.update();
        }
        run.app
            .world_mut()
            .get_mut::<shared::components::Health>(actor)
            .unwrap()
            .current = 1.0;
        crate::auras::apply_aura(
            run.app.world_mut(),
            &crate::auras::AuraApplication {
                caster: actor,
                target,
                spell_id: 247456,
                effect_mask: u32::MAX,
                source: crate::auras::AuraSource::Cast,
            },
        );
        crate::combat::deal_direct_damage(run.app.world_mut(), actor, target, 1000.0);
        for _ in 0..40 {
            run.app.update();
        }
        let healing: i32 = run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| {
                event.spell_id == Some(227255)
                    && event.kind == shared::protocol::CombatLogKind::Heal
            })
            .map(|event| event.amount)
            .sum();
        assert!(healing > 0, "Frailty must heal from actual damage");
    }

    #[test]
    #[ignore = "requires native Vengeance profile and canonical DB"]
    fn vengeance_leech_observes_damage_channels_and_gear_rating() {
        use super::super::*;
        let _seed = crate::random::seed_scope(70_448);
        let (profile, traits) = read_reference(581).unwrap();
        let identity = inputs::read_actor(&profile).unwrap();
        let mut run = prepare_reference(profile, &traits, 1, identity).unwrap();
        let actor = run.actor;
        let target = run.targets[0];
        engage(&mut run);
        for _ in 0..5 {
            run.app.update();
        }
        let stats = run
            .app
            .world()
            .get::<crate::stats::CharacterStats>(actor)
            .unwrap();
        assert_eq!(stats.ratings.leech, 55.0, "MID2 leech gear rating");
        assert!(
            (stats.leech_pct - 10.79709).abs() < 0.01,
            "native Soul Rending and rating"
        );
        run.app
            .world_mut()
            .get_mut::<shared::components::Health>(actor)
            .unwrap()
            .current = 1.0;
        for spell in [247455, 228478, 204598] {
            let before = run
                .app
                .world()
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0
                .len();
            assert!(crate::spell_cast::try_cast_triggered(
                run.app.world_mut(),
                actor,
                spell,
                Some(target),
                1
            ));
            for _ in 0..40 {
                run.app.update();
            }
            let events = &run
                .app
                .world()
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0[before..];
            assert!(
                events.iter().any(|event| event.spell_id == Some(spell)
                    && event.kind == shared::protocol::CombatLogKind::Damage
                    && event.amount > 0),
                "{spell} must deal damage"
            );
            assert!(
                events.iter().any(|event| event.spell_id == Some(143924)
                    && event.kind == shared::protocol::CombatLogKind::Heal
                    && event.amount > 0),
                "{spell} must feed pooled leech"
            );
        }
        crate::auras::remove_auras(
            run.app.world_mut(),
            target,
            crate::auras::RemoveReason::Consumed,
            |aura| aura.spell_id == 204598 && aura.caster == actor,
        );
        for _ in 0..40 {
            run.app.update();
        }
        let before = run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .len();
        crate::auras::apply_aura(
            run.app.world_mut(),
            &crate::auras::AuraApplication {
                caster: actor,
                target,
                spell_id: 207771,
                effect_mask: 0b10110,
                source: crate::auras::AuraSource::Cast,
            },
        );
        for _ in 0..60 {
            run.app.update();
        }
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0[before..];
        assert!(events.iter().any(|event| event.spell_id == Some(207771)
            && event.periodic
            && event.kind == shared::protocol::CombatLogKind::Damage
            && event.amount > 0));
        assert!(
            events.iter().any(|event| event.spell_id == Some(143924)
                && event.kind == shared::protocol::CombatLogKind::Heal
                && event.amount > 0),
            "Fiery Brand ticks must feed pooled leech"
        );
        crate::spell_cast::cast_for_player_with_target(
            run.app.world_mut(),
            actor,
            187827,
            Some(actor),
        )
        .unwrap();
        for _ in 0..3 {
            run.app.update();
        }
        let meta_leech = run
            .app
            .world()
            .get::<crate::stats::CharacterStats>(actor)
            .unwrap()
            .leech_pct;
        assert!(
            (meta_leech - 20.79709).abs() < 0.01,
            "Meta must apply Soul Rending's native leech modifier"
        );
    }

    #[test]
    #[ignore = "requires native Vengeance profile and canonical DB"]
    fn generic_leech_uses_native_aura_and_effective_damage() {
        use super::super::*;
        let (profile, traits) = read_reference(581).unwrap();
        let identity = inputs::read_actor(&profile).unwrap();
        let mut run = prepare_reference(profile, &traits, 1, identity).unwrap();
        let actor = run.actor;
        let target = run.targets[0];
        crate::auras::apply_aura(
            run.app.world_mut(),
            &crate::auras::AuraApplication {
                caster: actor,
                target: actor,
                spell_id: 204909,
                effect_mask: u32::MAX,
                source: crate::auras::AuraSource::Cast,
            },
        );
        run.app.update();
        let leech = run
            .app
            .world()
            .get::<crate::stats::CharacterStats>(actor)
            .unwrap()
            .leech_pct;
        assert!(
            leech >= 6.0,
            "native Soul Rending must contribute leech: {leech}"
        );
        {
            let mut health = run
                .app
                .world_mut()
                .get_mut::<shared::components::Health>(actor)
                .unwrap();
            health.current = health.max / 2.0;
        }
        crate::combat::deal_direct_damage(run.app.world_mut(), actor, target, 1000.0);
        let logs = |run: &Run| {
            run.app
                .world()
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0
                .iter()
                .filter(|event| {
                    event.spell_id == Some(143924)
                        && event.kind == shared::protocol::CombatLogKind::Heal
                })
                .count()
        };
        assert_eq!(logs(&run), 0, "leech is pooled, not immediate");
        for _ in 0..40 {
            run.app.update();
        }
        assert_eq!(
            logs(&run),
            1,
            "the pooled native leech heal must be delivered once"
        );
        for _ in 0..40 {
            run.app.update();
        }
        assert_eq!(logs(&run), 1, "self-leech must not feed another leech heal");
    }
}
