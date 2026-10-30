//! Restoration Druid replay triage using native spell admission and party health.

use crate::literal_rotation::{
    Driver,
    equipment::Equipment,
    execution::{NativeAction, Prepared},
};
use crate::simc_apl_native::{Bindings, Encounter, NativeFacts};
use bevy::prelude::{Entity, World};
use server::simc_apl::{Action, ActionFacts, Facts, Program};
use shared::components::{Health, PowerType};

pub const SPELLS: &[(&str, u32, bool)] = &[
    ("innervate", 29166, true),
    ("tranquility", 740, true),
    ("convoke_the_spirits", 391528, true),
    ("grove_guardians", 102693, false),
    ("wild_growth", 48438, false),
    ("efflorescence", 145205, false),
    ("ironbark", 102342, false),
    ("swiftmend", 18562, false),
    ("lifebloom", 33763, false),
    ("rejuvenation", 774, false),
    ("regrowth", 8936, false),
];

pub const DAMAGE_SPELLS: &[(&str, u32, bool)] =
    &[("moonfire", 8921, false), ("wrath", 5176, false)];

pub struct Restoration {
    driver: Driver,
    bindings: Bindings,
    equipment: Equipment,
    encounter: Encounter,
}

impl Restoration {
    pub fn load(world: &mut World, healer: Entity, encounter: Encounter) -> Result<Self, String> {
        let mut bindings = Bindings {
            actions: SPELLS
                .iter()
                .chain(DAMAGE_SPELLS.iter())
                .map(|(name, spell, _)| ((*name).into(), *spell))
                .collect(),
            resources: [("mana".into(), PowerType::Mana)].into(),
            global_cooldown_spell: 8936,
            ..Default::default()
        };
        let equipment = Equipment::load(world, healer, &mut bindings)?;
        // Pinned SimC has no supported Restoration healing APL; these are explicit
        // native triage decisions, not a claim of a SimC healer rotation.
        let program = Program::parse(&[
            ("default", "innervate,if=mana.pct<75"),
            (
                "default",
                "tranquility,if=target.health.pct<35&party.injured>=2",
            ),
            ("default", "ironbark,if=target.health.pct<35"),
            ("default", "swiftmend,if=target.health.pct<100"),
            ("default", "wild_growth,if=party.injured>=2"),
            (
                "default",
                "efflorescence,if=party.injured>=2&party.efflorescence_up=0",
            ),
            ("default", "grove_guardians,if=target.health.pct<95"),
            ("default", "convoke_the_spirits,if=party.injured>=2"),
            (
                "default",
                "lifebloom,if=target.health.pct<95&target.lifebloom_up=0",
            ),
            (
                "default",
                "rejuvenation,if=target.health.pct<95&target.rejuvenation_covered=0",
            ),
            ("default", "regrowth,if=target.health.pct<90"),
            ("damage", "moonfire,if=target.moonfire_up=0"),
            ("damage", "wrath"),
        ])?;
        Ok(Restoration {
            driver: Driver::new(program),
            bindings,
            equipment,
            encounter,
        })
    }
}

pub struct PartyFacts<'a> {
    native: NativeFacts<'a>,
    lifebloom_up: bool,
    rejuvenation_covered: bool,
    injured: usize,
    efflorescence_up: bool,
    moonfire_up: bool,
}
impl Facts for PartyFacts<'_> {
    fn value(&self, name: &str) -> Result<f64, String> {
        match name {
            "target.lifebloom_up" => Ok(f64::from(self.lifebloom_up)),
            "target.rejuvenation_covered" => Ok(f64::from(self.rejuvenation_covered)),
            "party.injured" => Ok(self.injured as f64),
            "party.efflorescence_up" => Ok(f64::from(self.efflorescence_up)),
            "target.moonfire_up" => Ok(f64::from(self.moonfire_up)),
            _ => self.native.value(name),
        }
    }
}
impl ActionFacts for PartyFacts<'_> {
    fn value_for(&self, action: &Action, name: &str) -> Result<f64, String> {
        match name {
            "target.lifebloom_up"
            | "target.rejuvenation_covered"
            | "party.injured"
            | "party.efflorescence_up"
            | "target.moonfire_up" => self.value(name),
            _ => self.native.value_for(action, name),
        }
    }
    fn target_ids(&self, action: &Action) -> Result<Vec<u64>, String> {
        self.native.target_ids(action)
    }
    fn target_ready(&self, action: &Action) -> Result<bool, String> {
        self.native.target_ready(action)
    }
    fn primary_resource(&self) -> Result<String, String> {
        Ok("mana".into())
    }
}

pub fn has_owned_hot(world: &World, healer: Entity, target: Entity, spell: u32) -> bool {
    world
        .get::<crate::auras::ActiveAuras>(target)
        .is_some_and(|auras| {
            auras
                .iter()
                .any(|aura| aura.spell_id == spell && aura.caster == healer)
        })
}

impl Restoration {
    /// Heal the party; damage `enemy` when no heal is due.
    pub fn drive(
        &mut self,
        world: &mut World,
        healer: Entity,
        party: &[Entity],
        enemy: Option<Entity>,
    ) -> Result<(), String> {
        let target = party
            .iter()
            .copied()
            .filter_map(|unit| {
                let health = world.get::<Health>(unit)?;
                (health.current > 0.0 && health.max > 0.0)
                    .then_some((unit, health.current / health.max))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(unit, _)| unit);
        let Some(target) = target else {
            return Ok(());
        };
        let injured = party
            .iter()
            .filter(|unit| {
                world
                    .get::<Health>(**unit)
                    .is_some_and(|h| h.current > 0.0 && h.current < h.max * 0.90)
            })
            .count();
        let efflorescence_up = world
            .query::<&crate::spell_cast::summon::SummonedCreature>()
            .iter(world)
            .any(|summon| summon.owner == healer && summon.spell_id == 145205);
        let targets = [target];
        let actions = SPELLS
            .iter()
            .map(|&(name, spell, self_target)| NativeAction {
                name,
                spell,
                self_target,
                talent: None,
            });
        let prepared = Prepared::read(world, healer, &targets, &self.equipment, actions, false)?;
        let facts = PartyFacts {
            native: NativeFacts::new(world, healer, &self.bindings, self.encounter, 1)
                .with_targets(Some(target), &targets),
            lifebloom_up: has_owned_hot(world, healer, target, 33763),
            rejuvenation_covered: has_owned_hot(world, healer, target, 774)
                && (!crate::auras::has_aura(world, healer, 155675)
                    || has_owned_hot(world, healer, target, 155777)),
            injured,
            efflorescence_up,
            moonfire_up: false,
        };
        if let Some((action, execution)) =
            self.driver
                .select("default", &facts, &prepared, &self.equipment)?
        {
            self.driver.execute(world, healer, action, execution)?;
            return Ok(());
        }
        let Some(enemy) = enemy else {
            return Ok(());
        };
        let targets = [enemy];
        let actions = DAMAGE_SPELLS
            .iter()
            .map(|&(name, spell, self_target)| NativeAction {
                name,
                spell,
                self_target,
                talent: None,
            });
        let prepared = Prepared::read(world, healer, &targets, &self.equipment, actions, false)?;
        let facts = PartyFacts {
            native: NativeFacts::new(world, healer, &self.bindings, self.encounter, 1)
                .with_targets(Some(enemy), &targets),
            lifebloom_up: false,
            rejuvenation_covered: false,
            injured,
            efflorescence_up,
            moonfire_up: has_owned_hot(world, healer, enemy, 164812),
        };
        if let Some((action, execution)) =
            self.driver
                .select("damage", &facts, &prepared, &self.equipment)?
        {
            self.driver.execute(world, healer, action, execution)?;
        }
        Ok(())
    }
}

impl crate::follower::SpecRotation for Restoration {
    fn poll_precombat(
        &mut self,
        _world: &mut World,
        _actor: Entity,
        _targets: &[Entity],
    ) -> Result<crate::literal_rotation::PrecombatPoll, String> {
        // The triage program has no precombat list.
        Ok(crate::literal_rotation::PrecombatPoll::Finished)
    }

    fn execute(
        &mut self,
        world: &mut World,
        actor: Entity,
        action: Action,
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
        let party = crate::follower::party_of(world, actor);
        Restoration::drive(self, world, actor, &party, targets.first().copied())
    }
}
