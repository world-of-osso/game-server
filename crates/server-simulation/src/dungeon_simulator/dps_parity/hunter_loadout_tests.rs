use super::*;
use crate::followers::simulator::ParityActor;
use crate::hunter_pet::PetStable;

#[test]
#[ignore = "requires a prepared native Beast Mastery SimC profile and world database"]
fn simc_hunter_loadout_installs_default_pet_for_follower_and_player() {
    for actor in [ParityActor::Follower, ParityActor::Player { race: 1 }] {
        let (profile, traits) = read_reference(253).unwrap();
        let run = spawn_reference(profile, &traits, 1, actor).unwrap();
        let stable = run.app.world().get::<PetStable>(run.actor).unwrap();
        assert_eq!(
            stable.stored.active[0].as_ref().unwrap().creature_entry,
            174884
        );
    }
}
