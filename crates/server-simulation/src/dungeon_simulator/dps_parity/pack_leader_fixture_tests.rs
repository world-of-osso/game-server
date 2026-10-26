//! Native70448 tree774/subtree43: selector99833→123350 and Howl94991→117588.
use super::*;
use shared::components::{ActiveSpec, Player};

#[test]
#[ignore = "requires the private native70448 spec255 Pack Leader player fixture"]
fn native_pack_leader_fixture_validates_255_identity_traits_and_pet() {
    let run = prepare_measurement(255, 1).expect("native Survival Pack Leader preparation");
    let world = run.app.world();
    assert_eq!(world.get::<Player>(run.actor).unwrap().class, 3);
    assert_eq!(world.get::<ActiveSpec>(run.actor).unwrap().0, 255);
    assert!(world.get::<crate::followers::Follower>(run.actor).is_none());
    let spells: Vec<_> = world
        .get::<crate::class_progression::TraitLoadout>(run.actor)
        .unwrap()
        .active_talent_spells()
        .map(|spell| spell.spell_id)
        .collect();
    assert!(spells.contains(&471876), "native selected Howl: {spells:?}");
    assert!(
        !spells.contains(&1253599),
        "Sentinel must not remain selected"
    );
    assert!(crate::auras::has_aura(world, run.actor, 471876));
    let pet = crate::hunter_pet::summoned_pet(world, run.actor)
        .expect("ordinary precombat Call Pet supplies the native Kill Command pet");
    assert_eq!(
        world
            .get::<crate::hunter_pet::HunterPet>(pet)
            .unwrap()
            .owner,
        run.actor
    );
    eprintln!(
        "SURVIVAL_PACK_LEADER_NATIVE_VALIDATED class=3 spec=255 hero=43 howl=471876 pet={pet:?}"
    );
}
