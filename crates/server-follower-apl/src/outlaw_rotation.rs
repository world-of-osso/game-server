//! Thin spec260 bindings; the shared Program and native spell engine own execution.
use crate::simc_apl_native::{Bindings, Encounter};
use crate::{
    assassination_rotation::{RogueFacts, stealthed},
    literal_rotation::{
        Driver, PrecombatPoll,
        equipment::Equipment,
        execution::{NativeAction, Prepared},
    },
};
use bevy::prelude::*;
use server::followers::parity_profile::Profile;
use server::simc_apl::{Action, ActionFacts, Facts, Program};
use shared::components::PowerType;

pub const ACTIONS: &[(&str, u32, bool)] = &[
    // Exact profile requests lethal=instant,nonlethal=none;
    // sc_rogue.cpp9405–9414 identifies native315584 as Instant Poison.
    ("apply_poison", 315584, true),
    ("stealth", 1784, true),
    ("adrenaline_rush", 13750, true),
    ("slice_and_dice", 315496, true),
    // sc_rogue.cpp9980 resolves the specialization grant; native70448 uses1214909.
    ("roll_the_bones", 1214909, true),
    ("kick", 1766, false),
    ("ambush", 8676, false),
    ("sinister_strike", 193315, false),
    ("pistol_shot", 185763, false),
    ("blade_flurry", 13877, true),
    ("preparation", 1277933, true),
    ("keep_it_rolling", 381989, true),
    ("blade_rush", 271877, false),
    ("vanish", 1856, true),
    ("shadowmeld", 58984, true),
    ("between_the_eyes", 315341, false),
    ("killing_spree", 51690, false),
    ("coup_de_grace", 441776, false),
    ("dispatch", 2098, false),
    ("blood_fury", 20572, true),
    ("berserking", 26297, true),
    ("fireblood", 265221, true),
    ("ancestral_call", 274738, true),
    ("arcane_torrent", 25046, true),
    ("arcane_pulse", 260364, true),
    ("lights_judgment", 255647, false),
    ("bag_of_tricks", 312411, false),
];

pub fn bindings() -> Bindings {
    Bindings {
        actions: ACTIONS
            .iter()
            .map(|&(name, id, _)| (name.into(), id))
            .collect(),
        buffs: [
            ("adrenaline_rush", 13750),
            ("blade_flurry", 13877),
            ("roll_the_bones", 1214909),
            ("loaded_dice", 256171),
            ("opportunity", 195627),
            ("audacity", 386270),
            ("between_the_eyes", 315341),
            ("fang_strike", 1301405),
            // sc_rogue.cpp10961–10966 preserves the ordered Supercharger counters.
            ("supercharge_1", 470398),
            ("supercharge_2", 470406),
            ("bloodlust", 2825),
        ]
        .into_iter()
        .map(|(name, id)| (name.into(), id))
        .collect(),
        talents: [
            ("improved_adrenaline_rush", 395422),
            ("hidden_opportunity", 383281),
            ("improved_ambush", 381620),
            ("deft_maneuvers", 381878),
            ("audacity", 381845),
            ("fan_the_hammer", 381846),
            ("quick_draw", 196938),
            ("deal_fate", 454419),
            ("supercharger", 470347),
        ]
        .into_iter()
        .map(|(name, id)| (name.into(), id))
        .collect(),
        resources: [
            ("energy".into(), PowerType::Energy),
            ("combo_points".into(), PowerType::ComboPoints),
        ]
        .into(),
        global_cooldown_spell: 193315,
        ..default()
    }
}

pub fn load_program(profile: &Profile) -> Result<Program, String> {
    let source = profile
        .options
        .get("actions")
        .and_then(serde_json::Value::as_array)
        .ok_or("Outlaw profile lacks literal actions")?;
    let rows = source
        .iter()
        .map(|row| {
            let row = row.as_str().ok_or("Outlaw action row must be text")?;
            let (key, value) = row
                .split_once('=')
                .ok_or("Outlaw action lacks assignment")?;
            let key = key.trim_end_matches('+');
            let list = if key == "actions" {
                "default"
            } else {
                key.strip_prefix("actions.")
                    .ok_or("Outlaw action lacks list")?
            };
            Ok((list, value.trim_start_matches('/')))
        })
        .collect::<Result<Vec<_>, String>>()?;
    // Retain exact source rows; the shared evaluator owns all APL semantics.
    Program::parse(&rows)
}

pub struct Rotation {
    pub driver: Driver,
    pub bindings: Bindings,
    pub equipment: Equipment,
    pub encounter: Encounter,
}

impl Rotation {
    pub fn load(
        world: &mut World,
        actor: Entity,
        encounter: Encounter,
        profile: &Profile,
    ) -> Result<Self, String> {
        let mut bindings = bindings();
        let equipment = Equipment::load(world, actor, &mut bindings)?;
        Ok(Self {
            driver: Driver::new(load_program(profile)?),
            bindings,
            equipment,
            encounter,
        })
    }

    pub fn poll(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
        precombat: bool,
    ) -> Result<PrecombatPoll, String> {
        let combat = crate::combat_refs::active_opponents(world, actor)
            .next()
            .is_some();
        let stealth = stealthed(world, actor);
        let actions = ACTIONS
            .iter()
            .filter(|&&(name, _, _)| name != "stealth" || (!combat && !stealth))
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
        let facts = OutlawFacts {
            rogue: RogueFacts::new(
                world,
                actor,
                &self.bindings,
                &self.equipment,
                self.encounter,
                targets,
            ),
            world,
            actor,
            prepared: &prepared,
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

pub struct OutlawFacts<'a> {
    rogue: RogueFacts<'a>,
    world: &'a World,
    actor: Entity,
    prepared: &'a Prepared,
    equipment: &'a Equipment,
}
impl Facts for OutlawFacts<'_> {
    fn value(&self, name: &str) -> Result<f64, String> {
        if name != "rtb_buffs" {
            return self.rogue.value(name);
        }
        // sc_rogue.cpp7628–7633/7739–7749/9670–9678: rolled state1..4,
        // represented by exactly one earned native secondary aura, not six old buffs.
        Ok([1214933, 1214934, 1214935, 1214937]
            .into_iter()
            .enumerate()
            .filter(|&(_, spell)| crate::auras::has_aura(self.world, self.actor, spell))
            .map(|(index, _)| (index + 1) as f64)
            .fold(0.0, f64::max))
    }
}
impl ActionFacts for OutlawFacts<'_> {
    fn primary_resource(&self) -> Result<String, String> {
        // SimC76f8118 sc_rogue.cpp1221: Rogue pooling uses Energy.
        self.rogue.value("energy")?;
        Ok("energy".into())
    }

    fn ready_at_resource_max(&self, action: &Action, resource: &str) -> Result<bool, String> {
        if resource != "energy" {
            return Err(format!("unbound native Outlaw pool {resource}"));
        }
        Ok(self.prepared.execution(action, self.equipment)?.is_some()
            || self
                .prepared
                .resource_limited(action, self.equipment, PowerType::Energy)?)
    }

    fn target_ids(&self, action: &Action) -> Result<Vec<u64>, String> {
        self.rogue.target_ids(action)
    }
    fn target_ready(&self, action: &Action) -> Result<bool, String> {
        self.rogue.target_ready(action)
    }
    fn value_for(&self, action: &Action, name: &str) -> Result<f64, String> {
        if name == "rtb_buffs" {
            self.value(name)
        } else {
            self.rogue.value_for(action, name)
        }
    }
}

impl crate::follower::SpecRotation for Rotation {
    fn poll_precombat(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<crate::literal_rotation::PrecombatPoll, String> {
        self.poll(world, actor, targets, true)
    }

    fn execute(
        &mut self,
        world: &mut World,
        actor: Entity,
        action: server::simc_apl::Action,
        execution: crate::literal_rotation::execution::Execution,
    ) -> Result<(), String> {
        self.driver.execute(world, actor, action, execution)
    }

    fn start_combat_at(&mut self, started_at: f64) {
        self.encounter = crate::literal_rotation::encounter_starting_at(self.encounter, started_at);
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
