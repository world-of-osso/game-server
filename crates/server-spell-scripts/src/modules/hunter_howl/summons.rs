//! Real Howl actors retain the source spell's native lifetime and hostile primary.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{self, TriggeredSpellOverrides, summon::SummonedCreature};
use combat::spell_info::SpellStore;

struct BeastSummon {
    ready: u32,
    spell: u32,
    duration_source: Option<u32>,
}

const SUMMONS: &[BeastSummon] = &[
    BeastSummon {
        ready: 471878,
        spell: 1222271,
        duration_source: None,
    },
    // SimC76f8118 hunter3797/7835: Bear's lifetime comes from471993.
    // Its actual native summon471990 is otherwise indefinite.
    BeastSummon {
        ready: 472325,
        spell: 471990,
        duration_source: Some(471993),
    },
];

fn read_duration(world: &World, rule: &BeastSummon) -> Option<i32> {
    rule.duration_source.map(|source| {
        let milliseconds = world
            .resource::<SpellStore>()
            .get(source)
            .expect("native Howl lifetime spell")
            .misc
            .as_ref()
            .and_then(|misc| misc.duration.as_ref())
            .expect("native Howl lifetime record")
            .duration_ms;
        assert!(milliseconds > 0, "Howl actor lifetime must be positive");
        milliseconds
    })
}

fn read_owned_summons(world: &mut World, caster: Entity, spell: u32) -> Vec<Entity> {
    world
        .query::<(Entity, &SummonedCreature)>()
        .iter(world)
        .filter(|(_, summon)| summon.owner == caster && summon.spell_id == spell)
        .map(|(unit, _)| unit)
        .collect()
}

pub(super) fn summon(world: &mut World, caster: Entity, ready: u32, primary: Entity) -> bool {
    let rule = SUMMONS
        .iter()
        .find(|rule| rule.ready == ready)
        .expect("only an actor-bearing Howl readiness can summon");
    let before = read_owned_summons(world, caster, rule.spell);
    let values = TriggeredSpellOverrides {
        duration_ms: read_duration(world, rule),
        ..Default::default()
    };
    // Native destinations remain caster-relative; the child's action retains
    // the consuming Kill Command's actual hostile primary.
    spell_cast::cast_triggered_with_overrides(world, caster, rule.spell, Some(primary), 1, values);
    let created = read_owned_summons(world, caster, rule.spell)
        .iter()
        .any(|unit| !before.contains(unit));
    if !created {
        warn!(
            ?caster,
            spell = rule.spell,
            "Howl retains readiness because native summon did not create an actor"
        );
    }
    created
}
