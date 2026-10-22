//! TC a352b1fa TheStoneVault/boss_skarmorak.cpp; native70448 source210156.
//! Normal1 uses the authored actor; Follower205 explicitly projects that same
//! native placement. No205 spawn/stat row is fabricated or published.
use super::*;
use shared::components::PowerType;

const SKARMORAK: u32 = 210156;
const MAP: u32 = 2652;
const BIT: u32 = 1;
const SMASH: u32 = 422233;

fn skarmorak_copy(difficulty: u32) -> (Harness, Entity, InMap) {
    let (mut h, boss, map) = bound_boss_at_difficulty(SKARMORAK, MAP, BIT, difficulty, None);
    assert_eq!(h.app.world().get::<UnitFlags>(boss).unwrap().0, 64);
    assert!(
        h.app.world().get::<Mana>(boss).is_none(),
        "actual sourceclass4 is Energy, not Mana"
    );
    assert_eq!(
        energy(&h, boss),
        100.0,
        "native class4 NPC full initialization; no fixture grant"
    );
    let mut visibility = crate::interest::NetworkVisibility::default();
    visibility.gain_visibility(h.players[0].1);
    h.app.world_mut().entity_mut(boss).insert(visibility);
    let mut near = *h.app.world().get::<Position>(boss).unwrap();
    near.x += 3.0;
    let player = h.player();
    let peer = h.players[1].0;
    h.app.world_mut().entity_mut(player).insert(near);
    h.app.world_mut().entity_mut(peer).insert(near);
    (h, boss, map)
}

fn energy(h: &Harness, boss: Entity) -> f32 {
    crate::spell_cast::powers::power_value(h.app.world(), boss, PowerType::Energy)
        .expect("sourceclass4 native Energy pool")
        .0
}

#[test]
#[ignore = "RED: TC Skarmorak AI is not ported"]
fn encounter_coverage_skarmorak_first_smash_normal_and_projected205_tc_clock() {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = skarmorak_copy(difficulty);
        h.engage(boss);
        h.advance(4.4);
        assert!(
            h.app
                .world()
                .get::<shared::casting::CastState>(boss)
                .is_none()
        );
        h.advance(0.3);
        let cast = h
            .app
            .world()
            .get::<shared::casting::CastState>(boss)
            .expect("TC first Crystalline Smash4.6s must enter ordinary native cast");
        assert_eq!(cast.spell_id, SMASH, "difficulty{difficulty}");
        assert_eq!(cast.duration, 2.0, "actual70448 casting time5");
        assert_eq!(
            energy(&h, boss),
            100.0,
            "ordinary cast spends at completion only"
        );
    }
}

#[test]
#[ignore = "Qualification: native source power and damage, not a timer port"]
fn encounter_coverage_skarmorak_native_smash_damage_and_actual_energy_cost() {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = skarmorak_copy(difficulty);
        let player = h.player();
        let peer = h.players[1].0;
        crate::spell_cast::creature_cast(h.app.world_mut(), boss, SMASH, Some(player), false)
            .expect("ordinary actual422233 from sourceclass4 full Energy");
        assert_eq!(energy(&h, boss), 100.0);
        h.advance(2.2);
        assert_eq!(
            energy(&h, boss),
            75.0,
            "actual native25Energy cost, no manual spend"
        );
        assert!(
            h.inbox(0)
                .log
                .iter()
                .any(|event| event.kind == CombatLogKind::Damage
                    && event.source == Some(boss.to_bits())
                    && event.target == Some(player.to_bits())
                    && event.spell_id == Some(SMASH)
                    && event.amount > 0)
        );
        assert!(
            h.inbox(1)
                .log
                .iter()
                .all(|event| event.source != Some(boss.to_bits()))
        );
        let peer_health = h.health(peer).current;
        h.advance(1.0);
        assert_eq!(
            h.health(peer).current,
            peer_health,
            "coincident peer remains isolated"
        );
        assert_eq!(
            energy(&h, boss),
            75.0,
            "source flags2=0 does not regenerate Energy"
        );
    }
}
