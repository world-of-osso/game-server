//! Native Discipline Priest priorities; the SimC 76f8118 healer APL is experimental.
use crate::literal_rotation::{
    Driver,
    equipment::Equipment,
    execution::{NativeAction, Prepared},
};
use crate::simc_apl_native::{
    Bindings, DotRefresh, DotRefreshPolicy, Encounter, NativeFacts, native_dot_spell,
};
use bevy::prelude::{Entity, World};
use server::simc_apl::{Action, ActionFacts, Facts, Program};
use shared::components::{Health, PowerType};

pub const SPELLS: &[(&str, u32, bool)] = &[
    ("pain_suppression", 33206, false),
    ("barrier", 62618, true),
    ("radiance", 194509, false),
    ("void_shield", 17, false),
    ("shield", 17, false),
    ("evangelism", 472433, false),
    ("penance", 47540, false),
    ("flash_heal", 2061, false),
    ("shadow_word_pain", 589, false),
    ("shadow_word_death", 32379, false),
    ("mind_blast", 8092, false),
    ("smite", 585, false),
];
pub struct Discipline {
    driver: Driver,
    bindings: Bindings,
    equipment: Equipment,
    encounter: Encounter,
}
impl Discipline {
    pub fn load(world: &mut World, healer: Entity, encounter: Encounter) -> Result<Self, String> {
        let mut bindings = Bindings {
            actions: SPELLS.iter().map(|(n, id, _)| ((*n).into(), *id)).collect(),
            resources: [("mana".into(), PowerType::Mana)].into(),
            buffs: [("shadow_word_pain".into(), 589)].into(),
            dot_refresh: [(
                "shadow_word_pain".into(),
                DotRefresh {
                    resolve: native_dot_spell,
                    policy: DotRefreshPolicy::Pandemic,
                },
            )]
            .into(),
            global_cooldown_spell: 17,
            ..Default::default()
        };
        let equipment = Equipment::load(world, healer, &mut bindings)?;
        let program = Program::parse(&[
            (
                "item",
                "use_item,name=soulcoiler_ritual_vessel,use_off_gcd=1,if=gcd.remains>=0.75*gcd.max",
            ),
            ("dot", "shadow_word_pain,if=refreshable"),
            ("default", "pain_suppression,if=target.health.pct<35"),
            ("default", "barrier,if=party.injured>=3"),
            ("ramp", "evangelism"),
            ("ramp", "radiance,if=cooldown.mind_blast.remains<=gcd.max*2"),
            ("radiance_fallback", "radiance"),
            (
                "shield",
                "void_shield,if=party.darkness_up&(party.darkness_up|cooldown.penance.remains<=gcd.max|party.active_atonements<=5)",
            ),
            (
                "shield",
                "shield,if=party.darkness_up=0&party.borrowed_time&(party.darkness_up|cooldown.penance.remains<=gcd.max|party.active_atonements<=5)&(mana.pct*2.5>=fight_remains|party.shield_discipline&mana.pct*6>=fight_remains)",
            ),
            ("heal", "penance,if=target.health.pct<45"),
            (
                "heal",
                "flash_heal,if=target.atonement_remains<=gcd.max&(mana.pct*7.5>=fight_remains|party.shield_discipline&mana.pct*18>=fight_remains|mana.pct>=90)",
            ),
            ("damage", "mind_blast"),
            ("damage", "penance"),
            ("damage", "shadow_word_death,if=target.health.pct<=20"),
            ("filler", "smite"),
        ])?;
        Ok(Discipline {
            driver: Driver::new(program),
            bindings,
            equipment,
            encounter,
        })
    }
}
pub fn atonement_remaining(world: &World, healer: Entity, target: Entity) -> f64 {
    let now = crate::spell_cast::now(world);
    world
        .get::<crate::auras::ActiveAuras>(target)
        .into_iter()
        .flat_map(|auras| auras.iter())
        .filter(|aura| aura.spell_id == 194384 && aura.caster == healer)
        .map(|aura| {
            aura.expires_at
                .map_or(f64::INFINITY, |expires| (expires - now).max(0.0))
        })
        .fold(0.0, f64::max)
}

pub struct PartyFacts<'a> {
    native: NativeFacts<'a>,
    injured: usize,
    active_atonements: usize,
    atonement_remains: f64,
    borrowed_time: bool,
    darkness: bool,
    shield_discipline: bool,
}
impl Facts for PartyFacts<'_> {
    fn value(&self, name: &str) -> Result<f64, String> {
        match name {
            "party.injured" => Ok(self.injured as f64),
            "party.active_atonements" => Ok(self.active_atonements as f64),
            "party.borrowed_time" => Ok(f64::from(self.borrowed_time)),
            "target.atonement_remains" => Ok(self.atonement_remains),
            "party.darkness_up" => Ok(f64::from(self.darkness)),
            "party.shield_discipline" => Ok(f64::from(self.shield_discipline)),
            _ => self.native.value(name),
        }
    }
}
impl ActionFacts for PartyFacts<'_> {
    fn value_for(&self, action: &Action, name: &str) -> Result<f64, String> {
        match name {
            "party.injured"
            | "party.active_atonements"
            | "party.borrowed_time"
            | "target.atonement_remains"
            | "party.darkness_up"
            | "party.shield_discipline" => self.value(name),
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
impl Discipline {
    fn select(
        &mut self,
        world: &mut World,
        healer: Entity,
        target: Entity,
        injured: usize,
        active_atonements: usize,
        list: &str,
        names: &[&str],
    ) -> Result<bool, String> {
        let targets = [target];
        let actions = SPELLS
            .iter()
            .filter(|(name, _, _)| names.contains(name))
            .map(|&(name, spell, self_target)| NativeAction {
                name,
                spell,
                self_target,
                talent: None,
            });
        let prepared = Prepared::read(world, healer, &targets, &self.equipment, actions, false)?;
        let atonement_remains = atonement_remaining(world, healer, target);
        let facts = PartyFacts {
            native: NativeFacts::new(world, healer, &self.bindings, self.encounter, 1)
                .with_targets(Some(target), &targets),
            injured,
            active_atonements,
            atonement_remains,
            borrowed_time: world
                .get::<crate::class_progression::KnownSpells>(healer)
                .is_some_and(|known| known.contains(390691)),
            darkness: crate::auras::has_aura(world, healer, 1253591),
            shield_discipline: crate::auras::has_aura(world, healer, 197045),
        };
        if let Some((action, execution)) =
            self.driver
                .select(list, &facts, &prepared, &self.equipment)?
        {
            self.driver.execute(world, healer, action, execution)?;
            return Ok(true);
        }
        Ok(false)
    }
}
impl Discipline {
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
        let Some((lowest, ratio)) = living.iter().copied().min_by(|a, b| a.1.total_cmp(&b.1))
        else {
            return Ok(());
        };
        let tank =
            crate::follower::party_tank(world, party).ok_or("Discipline party tank absent")?;
        let target = if ratio < 0.55 || !living.iter().any(|(unit, _)| *unit == tank) {
            lowest
        } else {
            tank
        };
        let injured = living.iter().filter(|(_, ratio)| *ratio < 0.85).count();
        let active_atonements = living
            .iter()
            .filter(|(unit, _)| atonement_remaining(world, healer, *unit) > 0.0)
            .count();
        // SimC76f8118 apl_priest.cpp:234: refresh Pain before other actions.
        if let Some(enemy) = enemy
            && self.select(
                world,
                healer,
                enemy,
                injured,
                active_atonements,
                "dot",
                &["shadow_word_pain"],
            )?
        {
            return Ok(());
        }
        if self.select(
            world,
            healer,
            healer,
            injured,
            active_atonements,
            "item",
            &[],
        )? {
            return Ok(());
        }
        if self.select(
            world,
            healer,
            target,
            injured,
            active_atonements,
            "default",
            &["pain_suppression", "barrier"],
        )? {
            return Ok(());
        }
        if self.select(
            world,
            healer,
            lowest,
            injured,
            active_atonements,
            "heal",
            &["penance"],
        )? {
            return Ok(());
        }
        if let Some(enemy) = enemy {
            world
                .entity_mut(healer)
                .insert(crate::networking::CombatTarget(Some(enemy)));
            if self.select(
                world,
                healer,
                enemy,
                injured,
                active_atonements,
                "damage",
                &["mind_blast"],
            )? {
                return Ok(());
            }
        }
        // SimC76f8118 apl_priest.cpp:244-248: target the shortest mark,
        // penalize self, and cast Radiance near Mind Blast's next cooldown.
        let ramp_target = living
            .iter()
            .min_by(|(left, _), (right, _)| {
                let score = |unit| {
                    atonement_remaining(world, healer, unit)
                        + if unit == healer { 10.0 } else { 0.0 }
                };
                score(*left).total_cmp(&score(*right))
            })
            .map(|(unit, _)| *unit);
        if let Some(target) = ramp_target
            && self.select(
                world,
                healer,
                target,
                injured,
                active_atonements,
                "ramp",
                &["evangelism", "radiance"],
            )?
        {
            return Ok(());
        }
        if let Some(enemy) = enemy
            && self.select(
                world,
                healer,
                enemy,
                injured,
                active_atonements,
                "damage",
                &["penance"],
            )?
        {
            return Ok(());
        }
        // SimC76f8118 apl_priest.cpp:231-232,249: prefer an Atoned target,
        // then the shortest remaining mark, with a ten-second self penalty.
        let shield_target = living
            .iter()
            .min_by(|(left, _), (right, _)| {
                let score = |unit| {
                    let remains = atonement_remaining(world, healer, unit);
                    remains
                        + if unit == healer { 10.0 } else { 0.0 }
                        + if remains <= 0.0 { 100.0 } else { 0.0 }
                };
                score(*left).total_cmp(&score(*right))
            })
            .map(|(unit, _)| *unit);
        if let Some(target) = shield_target
            && self.select(
                world,
                healer,
                target,
                injured,
                active_atonements,
                "shield",
                &["void_shield", "shield"],
            )?
        {
            return Ok(());
        }
        if let Some(enemy) = enemy
            && self.select(
                world,
                healer,
                enemy,
                injured,
                active_atonements,
                "damage",
                &["shadow_word_death"],
            )?
        {
            return Ok(());
        }
        let flash_target = living
            .iter()
            .min_by(|(left, _), (right, _)| {
                let score = |unit| {
                    atonement_remaining(world, healer, unit)
                        + if unit == healer { 20.0 } else { 0.0 }
                };
                score(*left).total_cmp(&score(*right))
            })
            .map(|(unit, _)| *unit);
        if let Some(target) = flash_target
            && self.select(
                world,
                healer,
                target,
                injured,
                active_atonements,
                "heal",
                &["flash_heal"],
            )?
        {
            return Ok(());
        }
        if let Some(target) = ramp_target
            && self.select(
                world,
                healer,
                target,
                injured,
                active_atonements,
                "radiance_fallback",
                &["radiance"],
            )?
        {
            return Ok(());
        }
        if let Some(enemy) = enemy {
            let _ = self.select(
                world,
                healer,
                enemy,
                injured,
                active_atonements,
                "filler",
                &["smite"],
            )?;
        }
        Ok(())
    }
}

impl crate::follower::SpecRotation for Discipline {
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
        Discipline::drive(self, world, actor, &party, targets.first().copied())
    }
}
