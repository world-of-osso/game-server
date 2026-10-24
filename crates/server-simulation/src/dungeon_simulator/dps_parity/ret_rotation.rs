//! Literal Ret adapter qualification; native facts and execution stay shared.
use super::literal_rotation::equipment::Equipment;
use crate::simc_apl_native::{Bindings, NativeFacts};
use server::simc_apl::{Action, ActionFacts, Facts};
use shared::components::PowerType;

// SimC76f8118 paladin spell lookups, checked against native70448 identities.
// Self targets follow the native caster/aura and caster-centered area effects.
const ACTIONS: &[(&str, u32, bool)] = &[
    ("rebuke", 96231, false),
    ("execution_sentence", 343527, false),
    ("avenging_wrath", 31884, true),
    ("wake_of_ashes", 255937, false),
    ("divine_toll", 375576, false),
    ("blade_of_justice", 184575, false),
    ("hammer_of_wrath", 24275, false),
    ("templars_verdict", 85256, false),
    ("divine_storm", 53385, true),
    ("hammer_of_light", 427453, false),
    ("templar_strike", 407480, false),
    ("templar_slash", 406647, false),
    ("crusader_strike", 35395, false),
    ("arcane_torrent", 155145, true),
    ("fireblood", 265221, true),
    ("lights_judgment", 255647, false),
    ("potion", 1236994, true),
    ("judgment", 20271, false),
];

const BUFFS: &[(&str, u32)] = &[
    ("avenging_wrath", 31884),
    ("divine_arbiter_verdict", 1306161),
    ("empyrean_legacy", 387178),
    ("divine_arbiter_divine_storm", 1306162),
    ("hammer_of_light_free", 433732),
    ("undisputed_ruling", 432629),
    ("art_of_war", 406086),
    ("righteous_cause", 402916),
    ("expurgation", 383346),
    ("execution_sentence_debuff", 343527),
];

const TALENTS: &[(&str, u32)] = &[
    ("radiant_glory", 458359),
    ("execution_sentence", 343527),
    ("holy_flames", 406545),
    ("crusade", 1253598),
    ("walk_into_light", 1263782),
];

fn bindings() -> Bindings {
    let names = |entries: &[(&str, u32)]| {
        entries
            .iter()
            .map(|&(name, id)| (name.to_owned(), id))
            .collect()
    };
    Bindings {
        actions: ACTIONS
            .iter()
            .map(|&(name, id, _)| (name.to_owned(), id))
            .collect(),
        buffs: names(BUFFS),
        talents: names(TALENTS),
        resources: [("holy_power".into(), PowerType::HolyPower)].into(),
        dot_refresh: Default::default(),
        global_cooldown_spell: 20271,
    }
}

fn execution_actions()
-> impl Iterator<Item = super::literal_rotation::execution::NativeAction<'static>> {
    // Inventory is not provisioned for the native potion; no fictitious admission.
    ACTIONS
        .iter()
        .filter(|(name, _, _)| *name != "potion")
        .map(
            |&(name, spell, self_target)| super::literal_rotation::execution::NativeAction {
                name,
                spell,
                self_target,
                talent: None,
            },
        )
}

fn read_program() -> Result<server::simc_apl::Program, String> {
    let rows = include_str!("ret_rotation/76f8118.simc")
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(parse_row)
        .collect::<Result<Vec<_>, _>>()?;
    server::simc_apl::Program::parse(&rows)
}

fn parse_row(line: &str) -> Result<(&str, &str), String> {
    let (head, literal) = line
        .split_once("+=")
        .ok_or("Ret source lacks += assignment")?;
    let list = if head == "actions" {
        "default"
    } else {
        head.strip_prefix("actions.")
            .ok_or("Ret source is not an action list")?
    };
    let literal = literal
        .strip_prefix('/')
        .ok_or("Ret source action lacks slash")?;
    Ok((list, literal))
}

struct RetFacts<'a> {
    native: NativeFacts<'a>,
    equipment: &'a Equipment,
}

fn with_equipment<'a>(native: NativeFacts<'a>, equipment: &'a Equipment) -> RetFacts<'a> {
    RetFacts { native, equipment }
}

fn is_equipment_operand(name: &str) -> bool {
    name.starts_with("trinket.") || name.starts_with("equipped.")
}

impl Facts for RetFacts<'_> {
    fn value(&self, name: &str) -> Result<f64, String> {
        if is_equipment_operand(name) {
            self.equipment.fact(name, &self.native)
        } else {
            self.native.value(name)
        }
    }
}

impl ActionFacts for RetFacts<'_> {
    fn target_ids(&self, action: &Action) -> Result<Vec<u64>, String> {
        self.native.target_ids(action)
    }

    fn value_for(&self, action: &Action, name: &str) -> Result<f64, String> {
        if is_equipment_operand(name) {
            self.value(name)
        } else {
            self.native.value_for(action, name)
        }
    }
}

#[cfg(test)]
mod dispatch_tests;
#[cfg(test)]
mod equipment_tests;
#[cfg(test)]
mod tests;
