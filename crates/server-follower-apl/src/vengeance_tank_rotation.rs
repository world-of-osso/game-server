//! Pinned MID2 Vengeance Demon Hunter action identities and native facts.
use crate::literal_rotation::equipment::Equipment;
use crate::simc_apl_native::{Bindings, Encounter, NativeFacts};
use bevy::prelude::{Entity, World};
use server::simc_apl::{Action, ActionFacts, Facts};
use shared::components::PowerType;

// SimC 76f8118 sc_death_knight.cpp action constructors and buff lookups.
pub const ACTIONS: &[(&str, u32)] = &[
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
pub const BUFFS: &[(&str, u32)] = &[
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
pub const TALENTS: &[(&str, u32)] = &[
    ("fiery_demise", 389220),
    ("soul_sigils", 395446),
    ("fallout", 227174),
    ("charred_flesh", 336639),
    ("sigil_of_spite", 390163),
    ("soul_carver", 207407),
    ("untethered_rage_2", 1270448),
    ("untethered_rage_3", 1270449),
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
        talents: names(TALENTS),
        resources: [("fury".into(), PowerType::Fury)].into(),
        global_cooldown_spell: 263642,
        ..Default::default()
    }
}

pub struct Context<'a> {
    native: NativeFacts<'a>,
    equipment: &'a Equipment,
    world: &'a World,
    actor: Entity,
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

pub fn program() -> Result<server::simc_apl::Program, String> {
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
                    "demon_spikes" | "immolation_aura" | "fel_devastation" | "spirit_bomb"
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
