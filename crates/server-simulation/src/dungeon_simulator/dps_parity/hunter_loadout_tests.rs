use super::*;
use crate::followers::simulator::ParityActor;
use crate::hunter_pet::PetStable;

#[test]
#[ignore = "requires a prepared native Beast Mastery SimC profile and world database"]
fn simc_hunter_loadout_installs_default_pet_for_follower_and_player() {
    for actor in [ParityActor::Follower, ParityActor::Player { race: 1 }] {
        let (profile, traits) = read_reference(253).unwrap();
        let mut run = spawn_reference(profile, &traits, 1, actor).unwrap();
        let world = run.app.world();
        let equipment = world
            .get::<crate::stats::EquippedItems>(run.actor)
            .unwrap()
            .clone();
        let talents = world
            .get::<crate::class_progression::TraitLoadout>(run.actor)
            .unwrap()
            .clone();
        let known = world
            .get::<crate::class_progression::KnownSpells>(run.actor)
            .unwrap()
            .clone();
        for _ in 0..3 {
            run.app.update();
        }
        let world = run.app.world();
        assert_eq!(
            world.get::<crate::stats::EquippedItems>(run.actor),
            Some(&equipment)
        );
        assert_eq!(
            world.get::<crate::class_progression::TraitLoadout>(run.actor),
            Some(&talents)
        );
        assert_eq!(
            world.get::<crate::class_progression::KnownSpells>(run.actor),
            Some(&known)
        );
        let stable = world.get::<PetStable>(run.actor).unwrap();
        assert_eq!(
            stable.stored.active[0].as_ref().unwrap().creature_entry,
            174884
        );
    }
}
