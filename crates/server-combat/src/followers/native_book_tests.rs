//! Native spell knowledge is not granted by controller action aliases.
use super::kit_tests::at_level;
use super::{Follower, spawning};
use crate::class_progression::{KnownSpells, TraitLoadout};
use bevy::prelude::*;
use shared::components::Player;

const RALIA: u32 = 232800;

#[test]
#[ignore = "native Ret source book on an actual Garrick NPC, without published Ret controller rows"]
fn follower_native_book_retribution_uses_requested_spec_and_native_starter_talents() {
    use crate::trait_config::{
        TraitContext, build_config, starter_purchases, talent_spell_entries,
    };
    for level in [1, 10, 32, 57, 90] {
        let (mut h, actor) = at_level(209057, level, |_| {});
        let mut profile = h
            .app
            .world()
            .get::<Follower>(actor)
            .unwrap()
            .profile
            .clone();
        profile.spec_id = 70;
        let expected: Vec<_> = {
            let classes = h.app.world().resource::<crate::spell_info::ClassSpells>();
            let class = classes
                .classes
                .iter()
                .find(|class| class.class_id == 2)
                .unwrap();
            let context = TraitContext { spec_id: 70, level };
            let purchases = starter_purchases(&class.traits, context).unwrap();
            let config = build_config(&class.traits, context, &purchases).unwrap();
            talent_spell_entries(&class.traits, &config)
                .map(|entry| entry.spell_id)
                .collect()
        };
        let owner = h.caster;
        let players_before = count_players(h.app.world_mut());
        spawning::initialize(h.app.world_mut(), owner, actor, profile).unwrap();
        assert_eq!(
            h.app
                .world()
                .get::<shared::components::ActiveSpec>(actor)
                .map(|spec| spec.0),
            Some(70),
            "Ret initializer must use requested NPC specialization at level{level}, even without kit rows"
        );
        let known = h.app.world().get::<KnownSpells>(actor).unwrap();
        for spell in &expected {
            assert!(
                known.contains(*spell),
                "Ret starter spell{spell} missing at level{level}"
            );
        }
        let traits = h
            .app
            .world()
            .get::<TraitLoadout>(actor)
            .expect("Ret native trait loadout");
        assert_eq!(
            traits
                .active_talent_spells()
                .map(|spell| spell.spell_id)
                .collect::<Vec<_>>(),
            expected
        );
        assert!(h.app.world().get::<Player>(actor).is_none());
        assert_eq!(count_players(h.app.world_mut()), players_before);
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
