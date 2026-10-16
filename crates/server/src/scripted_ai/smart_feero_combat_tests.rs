//! Authored acceptance immunity and ambusher appearance attack boundary.
use super::*;

#[test]
fn smartai_feero_acceptance_removes_npc_immunity_without_clearing_other_flags() {
    let (_temp, world, creature, _player) = feero();
    assert_eq!(
        world.get::<UnitFlags>(creature).unwrap().0 & 0x200,
        0,
        "TDB list448400 action145 must let authored ambushers fight Feero"
    );
}

#[test]
fn smartai_feero_authored_assassin_appearance_attacks_escort_not_acceptor() {
    let (_temp, mut world, creature, player) = feero();
    for node in 1..=19 {
        reach(&mut world, creature, player, node);
    }
    let ambushers: Vec<_> = world
        .query::<(Entity, &Npc)>()
        .iter(&world)
        .filter(|(_, npc)| npc.template_id == 3879)
        .map(|(entity, _)| entity)
        .collect();
    assert_eq!(ambushers.len(), 4);
    for ambusher in ambushers {
        let ai = creature_ai(&world, ambusher).unwrap();
        world.entity_mut(ambusher).insert(CreatureAiState {
            ai: Some(ai),
            pending: Default::default(),
        });
        dispatch(&mut world, ambusher, Hook::JustAppeared);
        assert_eq!(
            world
                .get::<crate::networking::CombatTarget>(ambusher)
                .unwrap()
                .0,
            Some(creature),
            "source11/49 chooses Feero rather than player"
        );
    }
}
