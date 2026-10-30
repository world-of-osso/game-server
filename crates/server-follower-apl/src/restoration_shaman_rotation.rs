//! Restoration Shaman healing decisions over native readiness and party state.
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
    ("earthliving_weapon", 382021, true),
    ("water_shield", 52127, true),
    ("mana_tide_totem", 16191, true),
    ("spirit_link_totem", 98008, true),
    ("healing_stream_totem", 5394, true),
    ("earth_shield", 974, false),
    ("unleash_life", 73685, false),
    ("healing_tide_totem", 108280, true),
    ("healing_rain", 73920, false),
    ("surging_totem", 444995, false),
    ("riptide", 61295, false),
    ("healing_surge", 8004, false),
    ("chain_heal", 1064, false),
    ("healing_wave", 77472, false),
];

pub const DAMAGE_SPELLS: &[(&str, u32, bool)] = &[
    ("flame_shock", 470411, false),
    ("lava_burst", 51505, false),
    ("chain_lightning", 188443, false),
    ("lightning_bolt", 188196, false),
];

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
            global_cooldown_spell: 77472,
            ..Default::default()
        };
        let equipment = Equipment::load(world, healer, &mut bindings)?;
        let program = Program::parse(&[
            ("default", "earthliving_weapon,if=player.earthliving_up=0"),
            ("default", "water_shield,if=player.water_shield_up=0"),
            ("default", "mana_tide_totem,if=mana.pct<75"),
            ("default", "spirit_link_totem,if=target.health.pct<35"),
            ("default", "healing_tide_totem,if=party.injured>=3"),
            ("default", "earth_shield,if=target.earth_shield_up=0"),
            ("default", "surging_totem"),
            ("default", "healing_rain,if=player.surging_totem=0"),
            ("default", "healing_stream_totem,if=target.health.pct<90"),
            ("default", "riptide"),
            ("default", "healing_surge,if=target.health.pct<45"),
            ("default", "unleash_life,if=target.health.pct<90"),
            (
                "default",
                "chain_heal,if=party.chain_heal_worthwhile&mana.pct>35",
            ),
            ("default", "healing_wave,if=target.healing_wave_useful"),
            ("damage", "flame_shock"),
            ("damage", "lava_burst"),
            ("damage", "chain_lightning,if=active_enemies>=2"),
            ("damage", "lightning_bolt"),
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
    injured: usize,
    earth_shield_up: bool,
    earthliving_up: bool,
    water_shield_up: bool,
    surging_totem: bool,
    healing_wave_useful: bool,
    chain_heal_worthwhile: bool,
}

impl Facts for PartyFacts<'_> {
    fn value(&self, name: &str) -> Result<f64, String> {
        match name {
            "party.injured" => Ok(self.injured as f64),
            "target.earth_shield_up" => Ok(f64::from(self.earth_shield_up)),
            "player.earthliving_up" => Ok(f64::from(self.earthliving_up)),
            "player.water_shield_up" => Ok(f64::from(self.water_shield_up)),
            "player.surging_totem" => Ok(f64::from(self.surging_totem)),
            "target.healing_wave_useful" => Ok(f64::from(self.healing_wave_useful)),
            "party.chain_heal_worthwhile" => Ok(f64::from(self.chain_heal_worthwhile)),
            _ => self.native.value(name),
        }
    }
}

impl ActionFacts for PartyFacts<'_> {
    fn value_for(&self, action: &Action, name: &str) -> Result<f64, String> {
        match name {
            "party.injured"
            | "target.earth_shield_up"
            | "target.healing_wave_useful"
            | "player.earthliving_up"
            | "player.water_shield_up"
            | "player.surging_totem"
            | "party.chain_heal_worthwhile" => self.value(name),
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

impl Restoration {
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
        let Some((target, _)) = living.iter().copied().min_by(|a, b| a.1.total_cmp(&b.1)) else {
            return Ok(());
        };
        let injured = living.iter().filter(|(_, ratio)| *ratio < 0.90).count();
        let chain_spell = world
            .resource::<crate::spell_info::SpellStore>()
            .get(1064)
            .ok_or("Chain Heal native spell data missing")?;
        let chain_effect = chain_spell
            .effects
            .iter()
            .find(|effect| effect.effect == 10)
            .ok_or("Chain Heal native heal effect missing")?;
        let chain_expected =
            crate::spell_cast::effects::scaled_amount(world, healer, chain_spell, chain_effect);
        let missing: Vec<_> = party
            .iter()
            .filter_map(|unit| world.get::<Health>(*unit))
            .filter(|health| health.current > 0.0 && health.current < health.max)
            .map(|health| health.max - health.current)
            .collect();
        let chain_heal_worthwhile =
            missing.len() >= 2 && missing.iter().sum::<f32>() >= chain_expected * 0.8;
        let earthliving_up = crate::auras::has_aura(world, healer, 382022);
        let water_shield_up = crate::auras::has_aura(world, healer, 52127);
        let surging_totem = crate::auras::has_aura(world, healer, 455630);
        let earth_shield_up = world
            .get::<crate::auras::ActiveAuras>(target)
            .is_some_and(|auras| {
                auras
                    .iter()
                    .any(|aura| aura.spell_id == 974 && aura.caster == healer)
            });
        let spell = world
            .resource::<crate::spell_info::SpellStore>()
            .get(77472)
            .ok_or("Healing Wave native spell data missing")?;
        let effect = spell
            .effects
            .iter()
            .find(|effect| effect.effect == 10)
            .ok_or("Healing Wave native heal effect missing")?;
        let expected_heal = crate::spell_cast::effects::scaled_amount(world, healer, spell, effect);
        let health = world
            .get::<Health>(target)
            .expect("living party member health");
        // Require room for most of the uncritical native base heal.
        let healing_wave_useful = health.max - health.current >= expected_heal * 0.8;
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
            native: NativeFacts::new(world, healer, &self.bindings, self.encounter, targets.len())
                .with_targets(Some(target), &targets),
            injured,
            earth_shield_up,
            earthliving_up,
            water_shield_up,
            surging_totem,
            healing_wave_useful,
            chain_heal_worthwhile,
        };
        if let Some((action, execution)) =
            self.driver
                .select("default", &facts, &prepared, &self.equipment)?
        {
            self.driver.execute(world, healer, action, execution)?;
            return Ok(());
        }
        let spell = world
            .resource::<crate::spell_info::SpellStore>()
            .get(188443)
            .ok_or("Chain Lightning native spell data missing")?
            .clone();
        let effect = spell
            .effects
            .iter()
            .find(|effect| effect.effect == 2)
            .ok_or("Chain Lightning damage effect missing")?;
        let Some(enemy) = enemy else {
            return Ok(());
        };
        let mut targets = vec![enemy];
        targets.extend(crate::spell_cast::targets::chain_targets(
            world,
            healer,
            &spell,
            effect,
            enemy,
            crate::spell_cast::targets::Relation::Enemy,
        ));
        let actions = DAMAGE_SPELLS
            .iter()
            .map(|&(name, spell, self_target)| NativeAction {
                name,
                spell,
                self_target,
                talent: None,
            });
        let prepared = Prepared::read(world, healer, &targets, &self.equipment, actions, false)?;
        let facts = NativeFacts::new(world, healer, &self.bindings, self.encounter, targets.len())
            .with_targets(Some(enemy), &targets);
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
