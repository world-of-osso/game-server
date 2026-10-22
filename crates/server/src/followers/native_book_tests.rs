//! Native spell knowledge is not granted by controller action aliases.
use super::kit_tests::at_level;
use super::{Follower, FollowerCatalog, spawning};
use crate::class_progression::{KnownSpells, TraitLoadout};
use crate::spell_cast::test_harness::Harness;
use bevy::prelude::*;
use shared::components::Player;

const RALIA: u32 = 232800;
const FERAL_FRENZY: u32 = 274837;
const FRANTIC_FRENZY: u32 = 1243807;

fn extend_native_family_alias(world: &mut World, spec: u32) {
    let mut catalog = world.resource_mut::<FollowerCatalog>();
    let family_end = catalog
        .kits
        .iter()
        .filter(|row| row.spec_id == spec && row.spell_id == FRANTIC_FRENZY)
        .map(|row| row.max_level)
        .max()
        .expect("native starter book must publish its actual Frantic replacement");
    let base = catalog
        .kits
        .iter_mut()
        .find(|row| row.spec_id == spec && row.spell_id == FERAL_FRENZY)
        .expect("native starter book must publish its initial Feral action");
    // Reproduce the source-derived family projection, not a native talent grant.
    // TraitDefinition138989 explicitly maps1243807 OverridesSpellID274837.
    base.max_level = family_end;
}

fn initialize_with_native_family_alias(level: u8) -> (Harness, Entity) {
    let (mut h, actor) = at_level(RALIA, level, |_| {});
    let profile = h
        .app
        .world()
        .get::<Follower>(actor)
        .unwrap()
        .profile
        .clone();
    extend_native_family_alias(h.app.world_mut(), profile.spec_id);
    let owner = h.caster;
    spawning::initialize(h.app.world_mut(), owner, actor, profile).unwrap();
    (h, actor)
}

#[test]
#[ignore = "native starter-book/kit-alias boundary; requires read-only native fixture DB"]
fn follower_native_book_does_not_learn_both_frenzy_variants_from_controller_alias() {
    for level in [57, 90, 55, 56, 54] {
        let (h, actor) = initialize_with_native_family_alias(level);
        let known = h.app.world().get::<KnownSpells>(actor).unwrap();
        assert!(
            !(known.contains(FERAL_FRENZY) && known.contains(FRANTIC_FRENZY)),
            "controller alias must not grant both native Frenzy variants at level{level}"
        );
        assert_eq!(known.contains(FERAL_FRENZY), (55..=56).contains(&level));
        assert_eq!(known.contains(FRANTIC_FRENZY), level >= 57);
        assert!(h.app.world().get::<TraitLoadout>(actor).is_some());
        assert!(h.app.world().get::<Player>(actor).is_none());
    }
}

fn count_players(world: &mut World) -> usize {
    world.query::<&Player>().iter(world).count()
}

#[test]
#[ignore = "native initializer must attach the validated source loadout without Player identity"]
fn follower_native_book_attaches_native_traits_without_a_player_entity() {
    let (mut h, actor) = at_level(RALIA, 32, |_| {});
    let before = count_players(h.app.world_mut());
    let profile = h
        .app
        .world()
        .get::<Follower>(actor)
        .unwrap()
        .profile
        .clone();
    let owner = h.caster;
    spawning::initialize(h.app.world_mut(), owner, actor, profile).unwrap();
    assert!(
        h.app.world().get::<TraitLoadout>(actor).is_some(),
        "ordinary NPC must retain its native starter trait loadout"
    );
    assert!(h.app.world().get::<Player>(actor).is_none());
    let after = count_players(h.app.world_mut());
    assert_eq!(
        after, before,
        "native book computation must not spawn a Player entity"
    );
}
