//! Literal Ret adapter qualification; native facts and execution stay shared.
use crate::simc_apl_native::Bindings;
use shared::components::PowerType;

// SimC76f8118 paladin spell lookups, checked against native70448 identities.
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
        buffs: names(BUFFS),
        talents: names(TALENTS),
        resources: [("holy_power".into(), PowerType::HolyPower)].into(),
        ..Bindings::default()
    }
}

fn execution_actions()
-> impl Iterator<Item = super::literal_rotation::execution::NativeAction<'static>> {
    std::iter::empty()
}

fn read_program() -> Result<server::simc_apl::Program, String> {
    server::simc_apl::Program::parse(&[])
}

#[cfg(test)]
mod dispatch_tests;
#[cfg(test)]
mod tests;
