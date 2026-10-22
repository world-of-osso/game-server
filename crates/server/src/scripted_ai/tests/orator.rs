//! TC a352b1fa CityOfThreads/boss_orator_krix_vizk.cpp; native70448 actor216619.
//! Follower205 explicitly projects the authored Normal placement, without
//! publishing invented205 spawn/stat rows.
use super::*;
use shared::components::PowerType;

const ORATOR: u32 = 216619;
const MAP: u32 = 2669;
const BIT: u32 = 1;
const SUBJUGATE: u32 = 434722;

fn orator_copy(difficulty: u32) -> (Harness, Entity, InMap) {
    let (mut h, boss, map) = bound_boss_at_difficulty(ORATOR, MAP, BIT, difficulty, None);
    assert_eq!(h.app.world().get::<UnitFlags>(boss).unwrap().0, 64);
    assert!(
        h.app.world().get::<Mana>(boss).is_none(),
        "actual sourceclass4 is Energy"
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
#[ignore = "RED: TC Orator AI JustAppeared and combat are not ported"]
fn encounter_coverage_orator_source_appearance_and_first_subjugate_normal_projected205() {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = orator_copy(difficulty);
        assert_eq!(
            energy(&h, boss),
            0.0,
            "TC JustAppeared setsEnergy0; fixture must not forge it"
        );
        assert!(h.has_aura(boss, 454689), "TC JustAppeared native Oration");
        h.engage(boss);
        h.advance(4.3);
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
            .expect("TC Subjugate first4.5s ordinary native cast");
        assert_eq!(cast.spell_id, SUBJUGATE, "difficulty{difficulty}");
        assert_eq!(cast.duration, 1.25, "actual70448 CastingTimeIndex206");
    }
}

#[test]
#[ignore = "Qualification: native Subjugate effects, not a boss timer port"]
fn encounter_coverage_orator_native_subjugate_damage_aura_and_copy_isolation() {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = orator_copy(difficulty);
        let player = h.player();
        let before = energy(&h, boss);
        crate::spell_cast::creature_cast(h.app.world_mut(), boss, SUBJUGATE, Some(player), false)
            .expect("ordinary native Subjugate, no invented power or triggered flags");
        h.advance(1.4);
        assert!(
            h.inbox(0)
                .log
                .iter()
                .any(|event| event.kind == CombatLogKind::Damage
                    && event.source == Some(boss.to_bits())
                    && event.target == Some(player.to_bits())
                    && event.spell_id == Some(SUBJUGATE)
                    && event.amount > 0)
        );
        assert!(
            h.has_aura(player, SUBJUGATE),
            "native aura33 belongs to the actual hit recipient"
        );
        assert!(
            !h.has_aura(h.players[1].0, SUBJUGATE),
            "coincident peer copy is isolated"
        );
        assert!(
            h.inbox(1)
                .log
                .iter()
                .all(|event| event.source != Some(boss.to_bits()))
        );
        assert_eq!(
            energy(&h, boss),
            before,
            "native Subjugate has no Energy cost"
        );
    }
}
