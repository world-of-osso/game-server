use super::*;

#[test]
#[ignore = "requires generated MID2 support inputs and canonical world DB"]
fn mid2_support_holy_paladin_input_loads_and_spawns() {
    let (profile, traits) = read_reference(65).unwrap();
    assert!(
        traits
            .iter()
            .any(|entry| entry.node_id == 103853 && entry.active_ranks > 0)
    );
    let identity = inputs::read_actor(&profile).unwrap();
    let run = spawn_reference(profile, &traits, 1, identity).unwrap();
    assert_eq!(
        run.app
            .world()
            .get::<shared::components::ActiveSpec>(run.actor)
            .unwrap()
            .0,
        65
    );
}
