//! Spec259 bindings for the shared literal evaluator; no production follower priorities.
use super::literal_rotation::{
    self, Driver, PrecombatPoll,
    equipment::Equipment,
    execution::{NativeAction, NativeComposite, Prepared},
};
use crate::simc_apl_native::{Bindings, DotRefresh, DotRefreshPolicy, Encounter, NativeFacts};
use bevy::prelude::*;
use server::simc_apl::{Action, ActionFacts, Facts, Program};
use shared::components::PowerType;

const ACTIONS: &[(&str, u32, bool)] = &[
    ("stealth", 1784, true),
    ("slice_and_dice", 315496, true),
    ("kick", 1766, false),
    ("thistle_tea", 381623, true),
    ("ambush", 8676, false),
    ("deathmark", 360194, false),
    ("kingsbane", 385627, false),
    ("garrote", 703, false),
    ("rupture", 1943, false),
    ("crimson_tempest", 1247227, false),
    ("shiv", 5938, false),
    ("fan_of_knives", 51723, true),
    ("mutilate", 1329, false),
    ("envenom", 32645, false),
    ("vanish", 1856, true),
    ("berserking", 26297, true),
    ("blood_fury", 20572, true),
    ("fireblood", 265221, true),
    ("ancestral_call", 274738, true),
];
const TALENTS: &[(&str, u32)] = &[
    ("blindside", 328085),
    ("improved_ambush", 381620),
    ("shrouded_suffocation", 385478),
    // SimC76f8118 trait_data.inc1271: Rogue talent;1265355 is the Warrior homonym.
    ("scent_of_blood", 381799),
    ("razor_wire", 1249802),
    ("crimson_tempest", 1247227),
    ("toxic_stiletto", 1267182),
    ("darkest_night", 457058),
    ("clear_the_witnesses", 1248793),
    ("improved_garrote", 381632),
];

pub(super) struct Rotation {
    pub driver: Driver,
    bindings: Bindings,
    equipment: Equipment,
    encounter: Encounter,
    coatings: Vec<NativeAction<'static>>,
}

impl Rotation {
    pub fn load(
        world: &mut World,
        actor: Entity,
        encounter: Encounter,
        source: &str,
    ) -> Result<Self, String> {
        // SimC76f8118 apl_rogue.cpp73–129: preserve every source row, order and predicate.
        let rows = source
            .lines()
            .filter(|line| !line.is_empty())
            .map(|line| {
                let (key, value) = line.split_once('=').ok_or("invalid literal profile row")?;
                let key = key.trim_end_matches('+');
                let list = if key == "actions" {
                    "default"
                } else {
                    key.strip_prefix("actions.")
                        .ok_or("invalid literal action list")?
                };
                Ok((list, value.trim_start_matches('/')))
            })
            .collect::<Result<Vec<_>, String>>()?;
        let mut bindings = Bindings {
            actions: ACTIONS
                .iter()
                .map(|&(name, id, _)| (name.into(), id))
                .collect(),
            buffs: [
                ("garrote", 703),
                ("rupture", 1943),
                ("deathmark", 360194),
                ("kingsbane", 385627),
                ("envenom", 32645),
                ("improved_garrote", 392401),
                ("darkest_night", 457280),
                ("scent_of_blood", 394080),
                ("lingering_darkness", 457273),
                ("bloodlust", 2825),
            ]
            .into_iter()
            .map(|(name, id)| (name.into(), id))
            .collect(),
            talents: TALENTS
                .iter()
                .map(|&(name, id)| (name.into(), id))
                .collect(),
            resources: [
                ("energy".into(), PowerType::Energy),
                ("combo_points".into(), PowerType::ComboPoints),
            ]
            .into(),
            global_cooldown_spell: 1329,
            ..default()
        };
        for alias in ["garrote", "rupture"] {
            bindings.dot_refresh.insert(
                alias.into(),
                DotRefresh {
                    resolve: crate::simc_apl_native::native_dot_spell,
                    policy: DotRefreshPolicy::Pandemic,
                },
            );
        }
        let equipment = Equipment::load(world, actor, &mut bindings)?;
        let coatings = read_coatings(world, actor);
        literal_rotation::capture_fixture(world, actor, encounter)?;
        Ok(Self {
            driver: Driver::new(Program::parse(&rows)?),
            bindings,
            equipment,
            encounter,
            coatings,
        })
    }

    fn prepared(
        &self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
        snapshot: bool,
    ) -> Result<Prepared, String> {
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
        Prepared::read(world, actor, targets, &self.equipment, actions, snapshot)?.with_composites(
            world,
            actor,
            targets,
            [NativeComposite {
                name: "apply_poison",
                steps: &self.coatings,
            }],
        )
    }

    pub fn poll_precombat(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<PrecombatPoll, String> {
        let prepared = self.prepared(world, actor, targets, self.driver.snapshot.is_none())?;
        let facts = RogueFacts::new(
            world,
            actor,
            &self.bindings,
            &self.equipment,
            self.encounter,
            targets,
        );
        self.driver
            .poll_precombat(&facts, &prepared, &self.equipment)
    }

    pub fn start_combat_at(&mut self, started_at: f64) {
        if let Encounter::Fixture {
            duration, style, ..
        } = self.encounter
        {
            self.encounter = Encounter::Fixture {
                started_at,
                duration,
                style,
            };
        }
    }

    pub fn drive(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<(), String> {
        let prepared = self.prepared(world, actor, targets, false)?;
        let facts = RogueFacts::new(
            world,
            actor,
            &self.bindings,
            &self.equipment,
            self.encounter,
            targets,
        );
        if let Some((action, execution)) =
            self.driver
                .select("default", &facts, &prepared, &self.equipment)?
        {
            self.driver.execute(world, actor, action, execution)?;
        }
        Ok(())
    }
}

pub(crate) fn read_coatings(world: &World, actor: Entity) -> Vec<NativeAction<'static>> {
    let known = world.get::<crate::class_progression::KnownSpells>(actor);
    let knows = |spell| known.is_some_and(|spells| spells.contains(spell));
    // sc_rogue.cpp3196–3298: default Deadly/Crippling, then selected DTB Amplifying/Atrophic.
    let lethal = if knows(2823) { 2823 } else { 315584 };
    let mut spells = vec![lethal, 3408];
    if knows(381801) {
        spells.push(if knows(381664) { 381664 } else { 315584 });
        if knows(5761) {
            spells.push(5761);
        } else if knows(381637) {
            spells.push(381637);
        }
    }
    spells
        .into_iter()
        .map(|spell| NativeAction {
            name: "apply_poison",
            spell,
            talent: None,
            self_target: true,
        })
        .collect()
}

pub(crate) fn stealthed(world: &World, actor: Entity) -> bool {
    // sc_rogue.cpp9691–9697: BASIC | ROGUE, including Subterfuge/Shadow Dance.
    [1784, 11327, 115191, 115192, 185313]
        .into_iter()
        .any(|id| crate::auras::has_aura(world, actor, id))
}

pub(crate) struct RogueFacts<'a> {
    native: NativeFacts<'a>,
    world: &'a World,
    actor: Entity,
    equipment: &'a Equipment,
    targets: &'a [Entity],
}
impl<'a> RogueFacts<'a> {
    pub(crate) fn new(
        world: &'a World,
        actor: Entity,
        bindings: &'a Bindings,
        equipment: &'a Equipment,
        encounter: Encounter,
        targets: &'a [Entity],
    ) -> Self {
        Self {
            native: NativeFacts::new(world, actor, bindings, encounter, targets.len())
                .with_targets(targets.first().copied(), targets),
            world,
            actor,
            equipment,
            targets,
        }
    }
}
impl RogueFacts<'_> {
    fn base_energy_deficit(&self) -> Result<f64, String> {
        // SimC76f8118 sc_rogue.cpp9822–9832 subtracts temporary capacity,
        // not permanent talent capacity. Native max-power auras own the values.
        let now = crate::spell_cast::now(self.world);
        let temporary: f64 = self
            .world
            .get::<crate::auras::ActiveAuras>(self.actor)
            .into_iter()
            .flat_map(|auras| auras.iter())
            .filter(|aura| !aura.passive && aura.expires_at.is_some_and(|at| at > now))
            .flat_map(|aura| aura.effects.iter().map(move |effect| (aura, effect)))
            .filter(|(_, effect)| {
                matches!(effect.aura_type, 35 | 418)
                    && effect.misc_values[0] == PowerType::Energy as i32
            })
            .map(|(aura, effect)| f64::from(effect.amount) * f64::from(aura.stacks.max(1)))
            .sum();
        let multiplier = self
            .world
            .get::<crate::auras::AuraStatMods>(self.actor)
            .map_or(1.0, |mods| {
                mods.max_power_multiplier(PowerType::Energy as i32)
            });
        let display = self
            .world
            .resource::<crate::spell_cast::powers::PowerTypeTable>()
            .get(PowerType::Energy)
            .ok_or("native Energy metadata absent")?
            .display_modifier;
        Ok((self.native.value("energy.max")?
            - self.native.value("energy")?
            - temporary * f64::from(multiplier) / f64::from(display))
        .max(0.0))
    }

    fn weapon_is_dagger(&self, slot: shared::protocol::EquipmentSlot) -> Result<f64, String> {
        let items = self
            .world
            .get::<crate::stats::EquippedItems>(self.actor)
            .ok_or("native equipment absent")?;
        let Some(&(_, source, id)) = items
            .items
            .iter()
            .find(|(equipped, _, _)| *equipped == slot)
        else {
            return Ok(0.0);
        };
        let item = self
            .world
            .resource::<crate::item_templates::ItemTemplates>()
            .get(source, id)
            .ok_or("native weapon absent")?;
        Ok(f64::from(u8::from(
            item.scaling.class_id == 2 && item.scaling.subclass_id == 15,
        )))
    }

    fn combined_regen(&self) -> Result<f64, String> {
        // sc_rogue.cpp9838–9906: potential regen plus poisoned bleed energy / hasted2s.
        let baseline = self.native.value("energy.regen")?;
        let known = self
            .world
            .get::<crate::class_progression::KnownSpells>(self.actor);
        if !known.is_some_and(|spells| spells.contains(79134)) {
            return Ok(baseline);
        }
        let spell = self
            .world
            .resource::<crate::spell_info::SpellStore>()
            .get(79134)
            .ok_or("native Venomous Wounds absent")?;
        let amount = spell
            .effects
            .iter()
            .find(|effect| effect.index == 1)
            .ok_or("native Venomous Wounds effect1 absent")?
            .base_points;
        let mut bleeds = 0;
        for &target in self.targets {
            let owned = |spell| {
                self.world
                    .get::<crate::auras::ActiveAuras>(target)
                    .is_some_and(|auras| {
                        auras
                            .iter()
                            .any(|aura| aura.caster == self.actor && aura.spell_id == spell)
                    })
            };
            if owned(2818) || owned(383414) {
                bleeds += usize::from(owned(703)) + usize::from(owned(1943));
            }
        }
        Ok(baseline
            + bleeds as f64 * f64::from(amount) / (2.0 * self.native.value("spell_haste")?))
    }
}
impl Facts for RogueFacts<'_> {
    fn value(&self, name: &str) -> Result<f64, String> {
        match name {
            // sc_rogue.cpp1258–1264/9634–9636: spend limit follows native
            // base combo-point capacity plus selected Stratagem capacity.
            "cp_max_spend" => self.native.value("combo_points.max"),
            "energy.base_deficit" => self.base_energy_deficit(),
            // sc_rogue.cpp9835–9908: base deficit divided by current regeneration.
            // Native aura110 includes Adrenaline Rush in the sampled rate.
            "energy.base_time_to_max" => {
                Ok(self.base_energy_deficit()? / self.native.value("energy.regen")?)
            }
            "stealthed.rogue" => Ok(f64::from(u8::from(stealthed(self.world, self.actor)))),
            "spell_targets.fan_of_knives" => Ok(self.targets.len() as f64),
            "main_hand.dagger" => self.weapon_is_dagger(shared::protocol::EquipmentSlot::MainHand),
            "off_hand.dagger" => self.weapon_is_dagger(shared::protocol::EquipmentSlot::OffHand),
            "energy.regen_combined" => self.combined_regen(),
            // There is no persistent Improved Garrote snapshot in native aura state yet.
            // SimC action.cpp pmultiplier is1 for an unempowered native application.
            "dot.garrote.pmultiplier" => Ok(1.0),
            _ if name.starts_with("trinket.") => self.equipment.fact(name, &self.native),
            _ => self.native.value(name),
        }
    }
}
impl ActionFacts for RogueFacts<'_> {
    fn target_ids(&self, action: &Action) -> Result<Vec<u64>, String> {
        self.native.target_ids(action)
    }
    fn target_ready(&self, action: &Action) -> Result<bool, String> {
        self.native.target_ready(action)
    }
    fn value_for(&self, action: &Action, name: &str) -> Result<f64, String> {
        match name {
            // SimC76f8118 action.cpp3795–3808 evaluates the proposed action's
            // GCD. Preserve that context through the shared native binding.
            "gcd" | "gcd.max" | "gcd.min" | "gcd.remains" => self.native.value_for(action, name),
            "remains" | "refreshable" => self
                .native
                .value_for(action, &format!("dot.{}.{}", action.name, name)),
            "pmultiplier" => self.value(&format!("dot.{}.pmultiplier", action.name)),
            _ if name.starts_with("dot.")
                || name.starts_with("debuff.")
                || name.starts_with("target.") =>
            {
                if name == "dot.garrote.pmultiplier" {
                    self.value(name)
                } else {
                    self.native.value_for(action, name)
                }
            }
            _ => self.value(name),
        }
    }
}
