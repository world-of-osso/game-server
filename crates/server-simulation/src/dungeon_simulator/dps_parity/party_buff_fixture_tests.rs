//! A parity fixture's composition is its one actor: before the fight it holds exactly
//! the party buffs that actor provides, cast natively as production followers do.
use super::*;
use server_follower_apl::party_buffs::{PARTY_BUFFS, holds_buff};

fn held_party_buffs(run: &Run) -> Vec<u32> {
    PARTY_BUFFS
        .into_iter()
        .map(|(spell, _)| spell)
        .filter(|&spell| holds_buff(run.app.world(), run.actor, spell).unwrap())
        .collect()
}

#[test]
#[ignore = "requires pinned parity profiles and DPS_PARITY_WORLD_DB"]
fn fixture_actor_holds_exactly_the_buffs_it_provides() {
    for (spec, expected) in [
        (63, &[1459][..]),
        (71, &[6673]),
        (103, &[1126]),
        (258, &[21562]),
        (262, &[462854]),
        (1467, &[364342]),
        (70, &[]),
    ] {
        let _seed = crate::random::seed_scope(70448);
        let run = prepare_measurement(spec, 1).unwrap();
        assert_eq!(held_party_buffs(&run), expected, "spec {spec}");
    }
}
