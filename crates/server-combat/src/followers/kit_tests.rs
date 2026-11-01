//! Native follower test fixtures.
use super::profile_behavior_tests::spawn_profile_fixture_with_plugins;
use super::*;
use crate::spell_cast::test_harness::Harness;
use shared::components::Level;

pub(super) fn level32(entry: u32) -> (Harness, Entity) {
    level32_with_plugins(entry, |_| {})
}

fn level32_with_plugins(entry: u32, configure: impl FnOnce(&mut App)) -> (Harness, Entity) {
    at_level(entry, 32, configure)
}

pub(crate) fn at_level(
    entry: u32,
    level: u8,
    configure: impl FnOnce(&mut App),
) -> (Harness, Entity) {
    let (mut h, unit) = at_level_unengaged(entry, level, configure);
    h.attack(h.npc);
    h.advance(3);
    (h, unit)
}

pub(crate) fn at_level_unengaged(
    entry: u32,
    level: u8,
    configure: impl FnOnce(&mut App),
) -> (Harness, Entity) {
    let (mut h, unit, profile) = spawn_profile_fixture_with_plugins(entry, configure);
    h.app.world_mut().get_mut::<Level>(h.caster).unwrap().0 = level;
    spawning::initialize(h.app.world_mut(), h.caster, unit, profile).unwrap();
    (h, unit)
}

pub(super) fn uses_spell(h: &mut Harness, unit: Entity, spell: u32) {
    for _ in 0..600 {
        update_ai(h.app.world_mut());
        h.advance(1);
        if h.inbox
            .go
            .iter()
            .any(|go| go.caster == unit.to_bits() && go.spell_id == spell)
        {
            return;
        }
    }
    panic!(
        "follower did not cast native spell {spell}; casts: {:?}",
        h.inbox
            .go
            .iter()
            .filter(|go| go.caster == unit.to_bits())
            .map(|go| go.spell_id)
            .collect::<Vec<_>>()
    );
}
