//! Native Holy Priest priorities; the SimC 76f8118 healer APL is experimental.
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
    ("guardian_spirit", 47788, false),
    ("divine_hymn", 64843, true),
    ("apotheosis", 200183, true),
    ("symbol_of_hope", 64901, true),
    ("serenity", 2050, false),
    ("sanctify", 34861, true),
    ("circle", 204883, false),
    ("prayer_of_healing", 596, false),
    ("prayer_of_mending", 33076, false),
    ("renew", 139, false),
    ("flash_heal", 2061, false),
    ("heal", 2060, false),
    ("halo", 120517, true),
    ("holy_fire", 14914, false),
    ("smite", 585, false),
];
pub struct Holy {
    driver: Driver,
    bindings: Bindings,
    equipment: Equipment,
    encounter: Encounter,
}
impl Holy {
    pub fn load(world: &mut World, healer: Entity, encounter: Encounter) -> Result<Self, String> {
        let mut bindings = Bindings {
            actions: SPELLS
                .iter()
                .map(|(name, id, _)| ((*name).into(), *id))
                .collect(),
            resources: [("mana".into(), PowerType::Mana)].into(),
            global_cooldown_spell: 2060,
            ..Default::default()
        };
        let equipment = Equipment::load(world, healer, &mut bindings)?;
        let program = Program::parse(&[
            ("default", "guardian_spirit,if=target.health.pct<30"),
            ("default", "divine_hymn,if=party.injured>=3"),
            ("default", "apotheosis,if=target.health.pct<35"),
            ("default", "symbol_of_hope,if=mana.pct<60"),
            ("default", "serenity,if=party.heal_worthwhile"),
            ("default", "sanctify,if=party.injured>=3"),
            ("default", "circle,if=party.injured>=3"),
            ("default", "prayer_of_healing,if=party.injured>=3"),
            ("default", "prayer_of_mending"),
            ("default", "halo"),
            ("default", "renew,if=target.health.pct<85&target.renew_up=0"),
            ("default", "flash_heal,if=party.heal_worthwhile"),
            ("default", "heal,if=party.heal_worthwhile"),
            ("damage", "holy_fire"),
            ("damage", "smite"),
        ])?;
        Ok(Holy {
            driver: Driver::new(program),
            bindings,
            equipment,
            encounter,
        })
    }
}
pub struct PartyFacts<'a> {
    native: NativeFacts<'a>,
    injured: usize,
    renew: bool,
    worthwhile: bool,
}
impl Facts for PartyFacts<'_> {
    fn value(&self, name: &str) -> Result<f64, String> {
        match name {
            "party.injured" => Ok(self.injured as f64),
            "target.renew_up" => Ok(f64::from(self.renew)),
            "party.heal_worthwhile" => Ok(f64::from(self.worthwhile)),
            _ => self.native.value(name),
        }
    }
}
impl ActionFacts for PartyFacts<'_> {
    fn value_for(&self, action: &Action, name: &str) -> Result<f64, String> {
        match name {
            "party.injured" | "target.renew_up" | "party.heal_worthwhile" => self.value(name),
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
impl Holy {
    /// Heal the party; damage `enemy` when no heal is due.
    pub fn drive(
        &mut self,
        world: &mut World,
        healer: Entity,
        party: &[Entity],
        enemy: Option<Entity>,
    ) -> Result<(), String> {
        let living: Vec<_> = party
            .iter()
            .copied()
            .filter_map(|unit| {
                let h = world.get::<Health>(unit)?;
                (h.current > 0.0 && h.max > 0.0).then_some((unit, h.current / h.max))
            })
            .collect();
        let Some((target, _)) = living.iter().copied().min_by(|a, b| a.1.total_cmp(&b.1)) else {
            return Ok(());
        };
        let injured = living.iter().filter(|(_, ratio)| *ratio < 0.85).count();
        let spell = world
            .resource::<crate::spell_info::SpellStore>()
            .get(2061)
            .ok_or("Holy Flash Heal definition absent")?;
        let effect = spell
            .effects
            .iter()
            .find(|effect| effect.effect == 10)
            .ok_or("Holy Flash Heal effect absent")?;
        let expected = crate::spell_cast::effects::scaled_amount(world, healer, spell, effect);
        let health = world
            .get::<Health>(target)
            .ok_or("Holy target health absent")?;
        let worthwhile = expected > 0.0 && health.max - health.current >= expected * 0.80;
        let targets = [target];
        let actions = SPELLS
            .iter()
            .filter(|(name, _, _)| !["holy_fire", "smite"].contains(name))
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
            injured,
            renew: crate::auras::has_aura(world, target, 139),
            worthwhile,
        };
        if let Some((action, execution)) =
            self.driver
                .select("default", &facts, &prepared, &self.equipment)?
        {
            self.driver.execute(world, healer, action, execution)?;
            return Ok(());
        }
        {
            if let Some(enemy) = enemy {
                let targets = [enemy];
                let actions = SPELLS
                    .iter()
                    .filter(|(name, _, _)| ["holy_fire", "smite"].contains(name))
                    .map(|&(name, spell, self_target)| NativeAction {
                        name,
                        spell,
                        self_target,
                        talent: None,
                    });
                let prepared =
                    Prepared::read(world, healer, &targets, &self.equipment, actions, false)?;
                let facts = NativeFacts::new(world, healer, &self.bindings, self.encounter, 1)
                    .with_targets(Some(enemy), &targets);
                if let Some((action, execution)) =
                    self.driver
                        .select("damage", &facts, &prepared, &self.equipment)?
                {
                    self.driver.execute(world, healer, action, execution)?;
                }
            }
        }
        Ok(())
    }
}

impl crate::follower::SpecRotation for Holy {
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
        Holy::drive(self, world, actor, &party, targets.first().copied())
    }
}
