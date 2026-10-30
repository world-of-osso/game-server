//! Mistweaver healing decisions over native readiness and party state.
use crate::literal_rotation::{
    Driver,
    equipment::Equipment,
    execution::{NativeAction, Prepared},
};
use crate::simc_apl_native::{Bindings, Encounter, NativeFacts};
use bevy::prelude::{Entity, World};
use server::simc_apl::{Action, ActionFacts, Facts, Program};
use shared::components::{Health, PowerType};
use std::collections::VecDeque;

pub const SPELLS: &[(&str, u32, bool)] = &[
    ("mana_tea", 115294, true),
    ("revival", 115310, true),
    ("life_cocoon", 116849, false),
    ("thunder_focus_tea", 116680, true),
    ("renewing_mist", 115151, false),
    ("enveloping_mist", 124682, false),
    ("soothing_mist", 115175, false),
    ("sheiluns_gift", 205406, false),
    ("vivify", 116670, false),
    ("rising_sun_kick", 107428, false),
    ("spinning_crane_kick", 101546, true),
    ("blackout_kick", 100784, false),
    ("tiger_palm", 100780, false),
];

pub struct Mistweaver {
    driver: Driver,
    bindings: Bindings,
    equipment: Equipment,
    encounter: Encounter,
    /// Next `ObservedEvents` number on the tank this healer has not read.
    seen: u64,
    incoming: VecDeque<(f64, f32)>,
    last_tank_heal: f32,
}

impl Mistweaver {
    pub fn load(world: &mut World, healer: Entity, encounter: Encounter) -> Result<Self, String> {
        let mut bindings = Bindings {
            actions: SPELLS
                .iter()
                .map(|(name, spell, _)| ((*name).into(), *spell))
                .collect(),
            resources: [("mana".into(), PowerType::Mana)].into(),
            global_cooldown_spell: 116670,
            ..Default::default()
        };
        let equipment = Equipment::load(world, healer, &mut bindings)?;
        // SimC76f8118 apl_monk.cpp supplies only Brewmaster and Windwalker defaults.
        let program = Program::parse(&[
            ("default", "life_cocoon,if=target.health.pct<35"),
            ("default", "revival,if=party.injured>=3"),
            (
                "default",
                "mana_tea,if=mana.pct<65&player.mana_tea_stacks>=4",
            ),
            ("default", "thunder_focus_tea,if=target.health.pct<75"),
            (
                "default",
                "renewing_mist,if=target.health.pct<95&target.renewing_mist_up=0",
            ),
            (
                "default",
                "enveloping_mist,if=target.health.pct<65&target.enveloping_mist_up=0",
            ),
            ("default", "sheiluns_gift,if=party.heal_worthwhile"),
            ("default", "vivify,if=party.heal_worthwhile"),
            ("default", "soothing_mist,if=target.health.pct<90"),
            ("priority_damage", "rising_sun_kick"),
            ("damage", "rising_sun_kick"),
            ("damage", "spinning_crane_kick,if=active_enemies>=2"),
            ("damage", "blackout_kick"),
            ("damage", "tiger_palm"),
        ])?;
        Ok(Mistweaver {
            driver: Driver::new(program),
            bindings,
            equipment,
            encounter,
            seen: 0,
            incoming: VecDeque::new(),
            last_tank_heal: 0.0,
        })
    }
}

pub struct PartyFacts<'a> {
    native: NativeFacts<'a>,
    injured: usize,
    renewing_up: bool,
    enveloping_up: bool,
    worthwhile: bool,
    mana_tea_stacks: u8,
}

impl Facts for PartyFacts<'_> {
    fn value(&self, name: &str) -> Result<f64, String> {
        match name {
            "party.injured" => Ok(self.injured as f64),
            "target.renewing_mist_up" => Ok(f64::from(self.renewing_up)),
            "target.enveloping_mist_up" => Ok(f64::from(self.enveloping_up)),
            "party.heal_worthwhile" => Ok(f64::from(self.worthwhile)),
            "player.mana_tea_stacks" => Ok(f64::from(self.mana_tea_stacks)),
            _ => self.native.value(name),
        }
    }
}

impl ActionFacts for PartyFacts<'_> {
    fn value_for(&self, action: &Action, name: &str) -> Result<f64, String> {
        match name {
            "party.injured"
            | "target.renewing_mist_up"
            | "target.enveloping_mist_up"
            | "party.heal_worthwhile"
            | "player.mana_tea_stacks" => self.value(name),
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

impl Mistweaver {
    fn sample_incoming(&mut self, world: &mut World, healer: Entity, tank: Entity) -> f32 {
        crate::followers::observe_unit(world, tank);
        let now = crate::spell_cast::now(world);
        let (events, next) = world
            .get::<crate::followers::ObservedEvents>(tank)
            .expect("observed tank")
            .since(self.seen);
        let mut tank_heal = 0.0_f32;
        for event in events {
            if event.kind == shared::protocol::CombatLogKind::Heal
                && event.source == Some(healer.to_bits())
                && event.spell_id == Some(399491)
                && event.target == Some(tank.to_bits())
            {
                tank_heal = tank_heal.max(event.amount.max(0) as f32);
            }
            if event.kind == shared::protocol::CombatLogKind::Damage
                && event.target == Some(tank.to_bits())
            {
                self.incoming.push_back((now, event.amount.max(0) as f32));
            }
        }
        self.seen = next;
        if tank_heal > 0.0 {
            self.last_tank_heal = tank_heal;
        }
        while self
            .incoming
            .front()
            .is_some_and(|(time, _)| *time <= now - 5.0)
        {
            self.incoming.pop_front();
        }
        self.incoming.iter().map(|(_, amount)| amount).sum::<f32>() / 5.0
    }

    fn worthwhile_heal(
        world: &World,
        healer: Entity,
        target: Entity,
        party: &[Entity],
        incoming_dps: f32,
        last_tank_heal: f32,
    ) -> Result<bool, String> {
        let Some(spell_id) =
            crate::literal_rotation::execution::resolve_action(world, healer, 116670)?
        else {
            return Ok(false);
        };
        let spell = world
            .resource::<crate::spell_info::SpellStore>()
            .get(spell_id)
            .ok_or("Mistweaver heal definition absent")?;
        let effect = spell
            .effects
            .iter()
            .find(|effect| effect.effect == 10)
            .ok_or("Mistweaver direct heal effect absent")?;
        let per_target =
            crate::spell_cast::effects::scaled_amount(world, healer, spell, effect).max(0.0);
        let health = world
            .get::<Health>(target)
            .ok_or("Mistweaver target health absent")?;
        let expected = per_target.max(last_tank_heal);
        if spell_id == 399491 {
            let count = crate::spell_cast::scripts::script_effect_value(world, healer, spell_id, 1)
                .max(1.0) as usize;
            let mut missing: Vec<_> = party
                .iter()
                .filter_map(|&unit| world.get::<Health>(unit))
                .filter(|health| health.current > 0.0)
                .map(|health| (health.max - health.current).max(0.0))
                .collect();
            missing.sort_by(|a, b| b.total_cmp(a));
            let group_missing: f32 = missing
                .into_iter()
                .take(count)
                .map(|amount| amount.min(expected))
                .sum();
            let emergency = health.current / health.max < 0.65
                && health.max - health.current >= expected * 0.75;
            return Ok(expected > 0.0
                && (emergency
                    || (group_missing >= expected * count as f32 * 0.75
                        && group_missing + incoming_dps >= expected)));
        }
        let missing = (health.max - health.current).max(0.0);
        Ok(expected > 0.0 && missing >= expected * 0.75 && missing + incoming_dps >= expected)
    }
}

impl Mistweaver {
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
                let health = world.get::<Health>(unit)?;
                (health.current > 0.0 && health.max > 0.0)
                    .then_some((unit, health.current / health.max))
            })
            .collect();
        let Some((lowest, lowest_ratio)) =
            living.iter().copied().min_by(|a, b| a.1.total_cmp(&b.1))
        else {
            return Ok(());
        };
        let tank =
            crate::follower::party_tank(world, party).ok_or("Mistweaver party tank absent")?;
        let tank_alive = living.iter().any(|(unit, _)| *unit == tank);
        let tank_mist = crate::auras::has_aura(world, tank, 119611)
            || crate::auras::has_aura(world, tank, 115151);
        let tank_injured = world
            .get::<Health>(tank)
            .is_some_and(|health| health.current < health.max * 0.95);
        let target = if tank_alive && tank_injured && !tank_mist && lowest_ratio >= 0.35 {
            tank
        } else {
            lowest
        };
        let incoming_dps = self.sample_incoming(world, healer, tank);
        let last_heal = if target == tank {
            self.last_tank_heal
        } else {
            0.0
        };
        let worthwhile =
            Self::worthwhile_heal(world, healer, target, party, incoming_dps, last_heal)?;
        let injured = living.iter().filter(|(_, ratio)| *ratio < 0.90).count();
        let renewing_up = crate::auras::has_aura(world, target, 119611)
            || crate::auras::has_aura(world, target, 115151);
        let enveloping_up = crate::auras::has_aura(world, target, 124682);
        let mana_tea_stacks = world
            .get::<crate::auras::ActiveAuras>(healer)
            .and_then(|auras| {
                auras
                    .iter()
                    .find(|aura| aura.spell_id == 115867 && aura.caster == healer)
            })
            .map_or(0, |aura| aura.stacks);
        if lowest_ratio >= 0.65
            && let Some(enemy) = enemy
        {
            let enemies = [enemy];
            let action = NativeAction {
                name: "rising_sun_kick",
                spell: 107428,
                self_target: false,
                talent: None,
            };
            let prepared =
                Prepared::read(world, healer, &enemies, &self.equipment, [action], false)?;
            let facts = NativeFacts::new(world, healer, &self.bindings, self.encounter, 1)
                .with_targets(Some(enemy), &enemies);
            if let Some((action, execution)) =
                self.driver
                    .select("priority_damage", &facts, &prepared, &self.equipment)?
            {
                self.driver.execute(world, healer, action, execution)?;
                return Ok(());
            }
        }
        let targets = [target];
        let actions = SPELLS
            .iter()
            .filter(|(name, _, _)| {
                !matches!(
                    *name,
                    "rising_sun_kick" | "spinning_crane_kick" | "blackout_kick" | "tiger_palm"
                )
            })
            .map(|&(name, spell, self_target)| NativeAction {
                name,
                spell,
                self_target,
                talent: None,
            });
        let prepared = Prepared::read(world, healer, &targets, &self.equipment, actions, false)?;
        let facts = PartyFacts {
            native: NativeFacts::new(world, healer, &self.bindings, self.encounter, targets.len())
                .with_targets(Some(target), &targets),
            injured,
            renewing_up,
            enveloping_up,
            worthwhile,
            mana_tea_stacks,
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
        let pulse = world
            .resource::<crate::spell_info::SpellStore>()
            .get(107270)
            .ok_or("Spinning Crane Kick pulse missing")?;
        let radius = crate::spell_cast::targets::radius(&pulse.effects[0], 1);
        let center = crate::spell_cast::targets::position(world, healer)
            .ok_or("Mistweaver position missing")?;
        let mut enemies = crate::spell_cast::targets::area_units(
            world,
            healer,
            center,
            radius,
            crate::spell_cast::targets::Relation::Enemy,
            0,
        );
        if !enemies.contains(&enemy) {
            enemies = vec![enemy];
        }
        let damage = SPELLS
            .iter()
            .filter(|(name, _, _)| {
                matches!(
                    *name,
                    "rising_sun_kick" | "spinning_crane_kick" | "blackout_kick" | "tiger_palm"
                )
            })
            .map(|&(name, spell, self_target)| NativeAction {
                name,
                spell,
                self_target,
                talent: None,
            });
        let prepared = Prepared::read(world, healer, &enemies, &self.equipment, damage, false)?;
        let facts = NativeFacts::new(world, healer, &self.bindings, self.encounter, enemies.len())
            .with_targets(Some(enemy), &enemies);
        if let Some((action, execution)) =
            self.driver
                .select("damage", &facts, &prepared, &self.equipment)?
        {
            self.driver.execute(world, healer, action, execution)?;
        }
        Ok(())
    }
}

impl crate::follower::SpecRotation for Mistweaver {
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
        Mistweaver::drive(self, world, actor, &party, targets.first().copied())
    }
}
