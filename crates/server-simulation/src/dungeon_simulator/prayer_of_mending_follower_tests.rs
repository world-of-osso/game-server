//! Prayer of Mending treats class followers as party players when jumping.

#[test]
#[ignore = "requires native Priest spell data"]
fn prayer_of_mending_can_jump_to_an_equally_injured_class_follower() {
    use crate::spell_cast::test_harness::{harness, spawn_player};
    use shared::components::{Health, Player};

    let mut player_jumps = 0;
    let mut follower_jumps = 0;
    for seed in 1..=12 {
        let _seed = crate::random::seed_scope(seed);
        let mut h = harness(Vec::new(), &[33076, 41635, 33110, 155793], 10).unwrap();
        let player = spawn_player(&mut h.app, 10, 3.0);
        let follower = spawn_player(&mut h.app, 10, 4.0);
        for unit in [player, follower] {
            *h.app.world_mut().get_mut::<Health>(unit).unwrap() = Health {
                current: 50_000.0,
                max: 100_000.0,
            };
        }
        h.app
            .world_mut()
            .entity_mut(follower)
            .remove::<Player>()
            .insert((
                crate::zones::TempSummon { summoner: h.caster },
                crate::followers::Follower {
                    owner: h.caster,
                    profile: crate::followers::Profile {
                        entry: 1,
                        name: "Follower".into(),
                        class_id: 5,
                        spec_id: 257,
                        role: shared::group::GroupRole::Dps,
                        attack_spell: 0,
                        cooldown_spell: 0,
                        heal_spell: 0,
                        taunt_spell: 0,
                        form_spell: 0,
                        melee: false,
                    },
                },
            ));
        h.cast_and_deliver(33076, Some(h.caster));
        h.advance(3);
        assert!(crate::auras::has_aura(h.app.world(), h.caster, 41635));
        super::healer_simc_replay::deliver_party_hit(h.app.world_mut(), h.npc, h.caster, 100.0);
        h.advance(15);
        player_jumps += usize::from(crate::auras::has_aura(h.app.world(), player, 41635));
        follower_jumps += usize::from(crate::auras::has_aura(h.app.world(), follower, 41635));
    }
    assert!(
        player_jumps > 0 && follower_jumps > 0,
        "both equally injured party actors must be eligible: player={player_jumps} follower={follower_jumps}"
    );
    assert_eq!(
        player_jumps + follower_jumps,
        12,
        "each real ward must jump once"
    );
}
