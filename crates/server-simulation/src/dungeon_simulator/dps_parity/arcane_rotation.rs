//! Thin spec62 bindings for the pinned profile; shared Driver owns APL execution.
use super::{
    inputs::Profile,
    literal_rotation::{
        self, Driver, PrecombatPoll,
        equipment::Equipment,
        execution::{NativeAction, Prepared},
    },
};
use crate::simc_apl_native::{Bindings, Encounter, NativeFacts};
use bevy::prelude::*;
use serde_json::{Value, json};
use server::simc_apl::{Action, ActionFacts, Facts, Program};
use shared::components::PowerType;

const ACTIONS: &[(&str, u32, bool)] = &[
    ("arcane_intellect", 1459, true),
    ("mirror_image", 55342, true),
    ("arcane_blast", 30451, false),
    ("arcane_barrage", 44425, false),
    ("arcane_missiles", 5143, false),
    ("arcane_orb", 153626, false),
    ("arcane_surge", 365350, false),
    ("touch_of_the_magi", 321507, false),
    ("prismatic_bolt", 1295924, false),
    ("evocation", 12051, true),
    ("presence_of_mind", 205025, true),
    ("counterspell", 2139, false),
    ("arcane_pulse", 260364, true),
    ("berserking", 26297, true),
    ("blood_fury", 33702, true),
    ("fireblood", 265221, true),
    ("ancestral_call", 274738, true),
];

pub(super) struct Rotation {
    driver: Driver,
    bindings: Bindings,
    equipment: Equipment,
    encounter: Encounter,
    potion: String,
}

const BUFFS: &[(&str, u32)] = &[
    ("arcane_surge", 365362),
    ("clearcasting", 263725),
    ("cumulative_power", 1296930),
    ("arcane_salvo", 1242974),
    ("prismatic_bolt", 1295942),
    ("overpowered_missiles", 1277009),
    ("arcane_soul", 451038),
    ("lustrous_gleam", 1295147),
    // SimC76f8118 sc_mage.cpp1411: the debuff differs from the cast.
    ("touch_of_the_magi", 210824),
];
const TALENTS: &[(&str, u32)] = &[
    ("spellfire_spheres", 448601),
    ("splintering_sorcery", 443739),
    ("high_voltage", 461248),
    ("orb_mastery", 1243435),
    ("charged_orb", 384651),
    ("orb_barrage", 384858),
];

fn bindings() -> Bindings {
    let names = |entries: &[(&str, u32)]| {
        entries
            .iter()
            .map(|&(alias, id)| (alias.into(), id))
            .collect()
    };
    Bindings {
        actions: ACTIONS
            .iter()
            .map(|&(alias, id, _)| (alias.into(), id))
            .collect(),
        buffs: names(BUFFS),
        talents: names(TALENTS),
        resources: [
            ("mana".into(), PowerType::Mana),
            ("arcane_charges".into(), PowerType::ArcaneCharges),
        ]
        .into(),
        global_cooldown_spell: 30451,
        ..Default::default()
    }
}

fn actions() -> impl Iterator<Item = NativeAction<'static>> {
    ACTIONS
        .iter()
        .map(|&(name, spell, self_target)| NativeAction {
            name,
            spell,
            self_target,
            talent: None,
        })
}

fn program(profile: &Profile) -> Result<Program, String> {
    let lines: Vec<String> = serde_json::from_value(
        profile
            .options
            .get("actions")
            .ok_or("Arcane profile has no literal actions")?
            .clone(),
    )
    .map_err(|error| error.to_string())?;
    // SimC76f8118 class_modules/apl/mage.cpp arcane(): preserve every source row,
    // including precombat variables, rather than constructing a native priority.
    let rows = lines
        .iter()
        .map(|line| parse_source_row(line))
        .collect::<Result<Vec<_>, String>>()?;
    Program::parse(&rows)
}

fn parse_source_row(line: &str) -> Result<(&str, &str), String> {
    let (head, body) = line
        .split_once('=')
        .ok_or("Arcane action lacks assignment")?;
    let head = head.strip_suffix('+').unwrap_or(head);
    let list = if head == "actions" {
        "default"
    } else {
        head.strip_prefix("actions.").ok_or("invalid Arcane list")?
    };
    Ok((list, body.strip_prefix('/').unwrap_or(body)))
}

impl Rotation {
    pub fn load(
        world: &mut World,
        actor: Entity,
        profile: &Profile,
        encounter: Encounter,
    ) -> Result<Self, String> {
        let mut bindings = bindings();
        let equipment = Equipment::load(world, actor, &mut bindings)?;
        let rotation = Self {
            driver: Driver::new(program(profile)?),
            bindings,
            equipment,
            encounter,
            potion: profile
                .consumables
                .get("potion")
                .ok_or("Arcane profile has no configured potion")?
                .clone(),
        };
        literal_rotation::capture_fixture(world, actor, encounter)?;
        Ok(rotation)
    }

    pub fn report(&self) -> Value {
        json!({"decisions": self.driver.decisions, "snapshot_stats": self.driver.snapshot})
    }

    pub fn start_combat_at(&mut self, started_at: f64) {
        self.encounter = Encounter::Fixture {
            started_at,
            duration: 300.0,
            style: crate::simc_apl_native::FightStyle::Patchwerk,
        };
    }

    pub fn precombat(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<PrecombatPoll, String> {
        let prepared = Prepared::read(
            world,
            actor,
            targets,
            &self.equipment,
            actions(),
            self.driver.snapshot.is_none(),
        )?;
        let facts = ArcaneFacts::new(
            world,
            actor,
            &self.bindings,
            &self.equipment,
            self.encounter,
            targets,
            &self.potion,
        );
        self.driver
            .poll_precombat(&facts, &prepared, &self.equipment)
    }

    pub fn execute(
        &mut self,
        world: &mut World,
        actor: Entity,
        action: Action,
        execution: super::literal_rotation::execution::Execution,
    ) -> Result<(), String> {
        self.driver.execute(world, actor, action, execution)
    }

    pub fn drive(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<(), String> {
        self.process_channel(world, actor, targets)?;
        let prepared = Prepared::read(world, actor, targets, &self.equipment, actions(), false)?;
        let facts = ArcaneFacts::new(
            world,
            actor,
            &self.bindings,
            &self.equipment,
            self.encounter,
            targets,
            &self.potion,
        );
        if let Some((action, execution)) =
            self.driver
                .select("default", &facts, &prepared, &self.equipment)?
        {
            self.driver.execute(world, actor, action, execution)?;
        }
        Ok(())
    }

    fn process_channel(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<(), String> {
        if !world
            .get::<shared::casting::CastState>(actor)
            .is_some_and(|cast| cast.cast_type == shared::casting::CastType::Channel)
        {
            return Ok(());
        }
        let after_channel =
            Prepared::read_after_channel(world, actor, targets, &self.equipment, actions())?;
        let facts = ArcaneFacts::new(
            world,
            actor,
            &self.bindings,
            &self.equipment,
            self.encounter,
            targets,
            &self.potion,
        );
        if let Some(decision) =
            self.driver
                .channel_decision(world, actor, &facts, &after_channel, &self.equipment)?
        {
            self.driver
                .execute_channel_decision(world, actor, decision)?;
        }
        Ok(())
    }
}

struct ArcaneFacts<'a> {
    native: NativeFacts<'a>,
    equipment: &'a Equipment,
    world: &'a World,
    actor: Entity,
    bindings: &'a Bindings,
    potion: &'a str,
}
impl<'a> ArcaneFacts<'a> {
    fn new(
        world: &'a World,
        actor: Entity,
        bindings: &'a Bindings,
        equipment: &'a Equipment,
        encounter: Encounter,
        targets: &'a [Entity],
        potion: &'a str,
    ) -> Self {
        Self {
            native: NativeFacts::new(world, actor, bindings, encounter, targets.len())
                .with_targets(targets.first().copied(), targets),
            equipment,
            world,
            actor,
            bindings,
            potion,
        }
    }

    fn previous_gcd_matches(&self, alias: &str) -> Result<bool, String> {
        let spell = self
            .bindings
            .actions
            .get(alias)
            .ok_or_else(|| format!("unknown Arcane history action {alias}"))?;
        // SimC76f8118 player.cpp7397–7400 records foreground GCD actions at
        // queue time, including a native cast still in progress, not off-GCD casts.
        let store = self.world.resource::<crate::spell_info::SpellStore>();
        let gcd_actions: Vec<_> = self
            .bindings
            .actions
            .values()
            .copied()
            .filter(|id| {
                store.get(*id).is_some_and(|spell| {
                    spell
                        .cooldowns
                        .as_ref()
                        .is_some_and(|cooldown| cooldown.start_recovery_ms > 0)
                })
            })
            .collect();
        let previous = if let Some(cast) = self
            .world
            .get::<shared::casting::CastState>(self.actor)
            .filter(|cast| gcd_actions.contains(&cast.spell_id))
        {
            Some(cast.spell_id)
        } else {
            self.world
                .get::<crate::spell_cast::SpellHistory>(self.actor)
                .and_then(|history| history.last_cast_in(&gcd_actions))
        };
        Ok(previous == Some(*spell))
    }
}
impl Facts for ArcaneFacts<'_> {
    fn value(&self, name: &str) -> Result<f64, String> {
        if name == "buff.arcane_charge.stack" {
            return self.native.value("arcane_charges");
        }
        if let Some(alias) = name.strip_prefix("potion.") {
            return Ok(f64::from(
                self.potion == alias || self.potion.strip_suffix("_2") == Some(alias),
            ));
        }
        if let Some(alias) = name.strip_prefix("prev_gcd.1.") {
            return self.previous_gcd_matches(alias).map(f64::from);
        }
        if name.starts_with("equipped.")
            || name.starts_with("set_bonus.")
            || name.starts_with("trinket.")
        {
            return self.equipment.fact(name, &self.native);
        }
        self.native.value(name)
    }
}
impl ActionFacts for ArcaneFacts<'_> {
    fn target_ready(&self, action: &Action) -> Result<bool, String> {
        // SimC76f8118 sc_mage.cpp2887/2931/3226/3673:
        // class action readiness is independent of the source row's if_expr.
        const GATES: &[(&str, &str, bool)] = &[
            ("arcane_blast", "buff.prismatic_bolt.up", false),
            ("prismatic_bolt", "buff.prismatic_bolt.up", true),
            ("arcane_missiles", "buff.clearcasting.up", true),
            ("counterspell", "target.debuff.casting.up", true),
        ];
        for &(name, operand, expected) in GATES {
            if action.name == name {
                let active = self.native.value_for(action, operand)? != 0.0;
                return Ok(active == expected);
            }
        }
        Ok(true)
    }

    fn value_for(&self, action: &Action, name: &str) -> Result<f64, String> {
        if name.starts_with("target.")
            || name.starts_with("debuff.")
            || name.starts_with("dot.")
            || name.starts_with("gcd")
        {
            self.native.value_for(action, name)
        } else {
            self.value(name)
        }
    }
}
