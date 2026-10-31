//! Pinned MID2 Blood Death Knight action identities and native facts.
use crate::literal_rotation::equipment::Equipment;
use crate::simc_apl_native::{Bindings, Encounter, NativeFacts};
use bevy::prelude::{Entity, World};
use server::simc_apl::{Action, ActionFacts, Facts};
use shared::components::PowerType;

// SimC 76f8118 sc_death_knight.cpp action constructors and buff lookups.
pub const ACTIONS: &[(&str, u32)] = &[
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
pub const BUFFS: &[(&str, u32)] = &[
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

pub fn bindings() -> Bindings {
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

pub struct Context<'a> {
    native: NativeFacts<'a>,
    equipment: &'a Equipment,
}

impl<'a> Context<'a> {
    pub fn new(
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
        self.native.value("runic_power")?;
        Ok("runic_power".into())
    }
}

pub fn program() -> Result<server::simc_apl::Program, String> {
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

pub struct Rotation {
    pub driver: crate::literal_rotation::Driver,
    pub bindings: Bindings,
    pub equipment: Equipment,
    pub encounter: Encounter,
}

impl Rotation {
    pub fn prepared(
        &self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
        snapshot: bool,
    ) -> Result<crate::literal_rotation::execution::Prepared, String> {
        use crate::literal_rotation::execution::{NativeAction, Prepared};
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

    pub fn select(
        &mut self,
        world: &World,
        actor: Entity,
        targets: &[Entity],
        list: &str,
        prepared: &crate::literal_rotation::execution::Prepared,
    ) -> Result<Option<(Action, crate::literal_rotation::execution::Execution)>, String> {
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

impl Rotation {
    pub fn load(world: &World, actor: Entity, encounter: Encounter) -> Result<Self, String> {
        let mut bindings = bindings();
        let equipment = Equipment::load(world, actor, &mut bindings)?;
        Ok(Self {
            driver: crate::literal_rotation::Driver::new(program()?),
            bindings,
            equipment,
            encounter,
        })
    }
}

impl crate::follower::SpecRotation for Rotation {
    fn poll_precombat(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<crate::literal_rotation::PrecombatPoll, String> {
        let prepared = self.prepared(world, actor, targets, self.driver.snapshot.is_none())?;
        Ok(
            match self.select(world, actor, targets, "precombat", &prepared)? {
                Some((action, execution)) => {
                    crate::literal_rotation::PrecombatPoll::Selected { action, execution }
                }
                None => crate::literal_rotation::PrecombatPoll::Finished,
            },
        )
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
        let prepared = self.prepared(world, actor, targets, false)?;
        if let Some((action, execution)) =
            self.select(world, actor, targets, "default", &prepared)?
        {
            self.driver.execute(world, actor, action, execution)?;
        }
        Ok(())
    }
}
