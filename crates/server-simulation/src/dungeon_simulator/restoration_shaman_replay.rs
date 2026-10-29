//! Restoration Shaman healing decisions over native readiness and party state.
#[cfg(test)]
#[path = "restoration_shaman_replay_tests.rs"]
mod tests;
use super::dps_parity::literal_rotation::{
    self, Driver,
    equipment::Equipment,
    execution::{NativeAction, Prepared},
};
use super::healer_simc_replay::Policy;
use crate::simc_apl_native::{Bindings, Encounter, FightStyle, NativeFacts};
use bevy::prelude::{Entity, World};
use server::simc_apl::{Action, ActionFacts, Facts, Program};
use shared::components::{Health, PowerType};

const SPELLS: &[(&str, u32, bool)] = &[
    ("earthliving_weapon", 382021, true),
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

const DAMAGE_SPELLS: &[(&str, u32, bool)] = &[
    ("flame_shock", 470411, false),
    ("lava_burst", 51505, false),
    ("lightning_bolt", 188196, false),
];

struct Restoration {
    enemy: Entity,
    driver: Driver,
    bindings: Bindings,
    equipment: Equipment,
    encounter: Encounter,
}

pub(super) fn prepare(
    run: &mut super::dps_parity::Run,
    healer: Entity,
) -> Result<Box<dyn Policy>, String> {
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
    let equipment = Equipment::load(run.app.world_mut(), healer, &mut bindings)?;
    let encounter = Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: FightStyle::Patchwerk,
    };
    literal_rotation::capture_fixture(run.app.world_mut(), healer, encounter)?;
    let program = Program::parse(&[
        ("default", "earthliving_weapon,if=player.earthliving_up=0"),
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
        ("default", "chain_heal,if=party.injured>=2"),
        ("default", "healing_wave,if=target.healing_wave_useful"),
        ("damage", "flame_shock"),
        ("damage", "lava_burst"),
        ("damage", "lightning_bolt"),
    ])?;
    Ok(Box::new(Restoration {
        enemy: run.targets[0],
        driver: Driver::new(program),
        bindings,
        equipment,
        encounter,
    }))
}

struct PartyFacts<'a> {
    native: NativeFacts<'a>,
    injured: usize,
    earth_shield_up: bool,
    earthliving_up: bool,
    surging_totem: bool,
    healing_wave_useful: bool,
}

impl Facts for PartyFacts<'_> {
    fn value(&self, name: &str) -> Result<f64, String> {
        match name {
            "party.injured" => Ok(self.injured as f64),
            "target.earth_shield_up" => Ok(f64::from(self.earth_shield_up)),
            "player.earthliving_up" => Ok(f64::from(self.earthliving_up)),
            "player.surging_totem" => Ok(f64::from(self.surging_totem)),
            "target.healing_wave_useful" => Ok(f64::from(self.healing_wave_useful)),
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
            | "player.surging_totem" => self.value(name),
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

impl Policy for Restoration {
    fn drive(&mut self, world: &mut World, healer: Entity, party: &[Entity]) -> Result<(), String> {
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
        let earthliving_up = crate::auras::has_aura(world, healer, 382022);
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
            surging_totem,
            healing_wave_useful,
        };
        if let Some((action, execution)) =
            self.driver
                .select("default", &facts, &prepared, &self.equipment)?
        {
            self.driver.execute(world, healer, action, execution)?;
            return Ok(());
        }
        let targets = [self.enemy];
        let actions = DAMAGE_SPELLS
            .iter()
            .map(|&(name, spell, self_target)| NativeAction {
                name,
                spell,
                self_target,
                talent: None,
            });
        let prepared = Prepared::read(world, healer, &targets, &self.equipment, actions, false)?;
        let facts = NativeFacts::new(world, healer, &self.bindings, self.encounter, 1)
            .with_targets(Some(self.enemy), &targets);
        if let Some((action, execution)) =
            self.driver
                .select("damage", &facts, &prepared, &self.equipment)?
        {
            self.driver.execute(world, healer, action, execution)?;
        }
        Ok(())
    }
}
