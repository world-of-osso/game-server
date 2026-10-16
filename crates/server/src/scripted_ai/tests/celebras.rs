//! Pinned Celebras source combat, corruption summons and redeemed death actor.
use super::*;

#[test]
fn encounter_coverage_celebras_source_spells_summons_and_isolated_credit() {
    const BIT: u32 = 3;
    const ENCOUNTER: u32 = 425;
    let (mut h, boss, map) = bound_boss(12225, 349, BIT);
    let player = h.player();
    let origin = *h.app.world().get::<Position>(boss).unwrap();
    h.app.world_mut().entity_mut(player).insert((
        Position {
            x: origin.x + 3.0,
            ..origin
        },
        Health {
            current: 1_000_000.0,
            max: 1_000_000.0,
        },
    ));
    let mut visibility = crate::interest::NetworkVisibility::default();
    visibility.gain_visibility(h.players[0].1);
    h.app.world_mut().entity_mut(boss).insert(visibility);
    h.engage(boss);
    let mut roots_seen = false;
    for _ in 0..720 {
        h.advance(0.05);
        roots_seen |= h.has_aura(player, 12747);
    }
    for spell in [21807, 12747, 21968] {
        assert!(
            !spell_gos(&h, boss, spell).is_empty(),
            "source Celebras spell{spell}"
        );
    }
    assert!(
        h.inbox(0)
            .log
            .iter()
            .any(|event| event.source == Some(boss.to_bits())
                && event.target == Some(player.to_bits())
                && event.spell_id == Some(21807)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0),
        "real Wrath damage"
    );
    assert!(roots_seen, "real victim Entangling Roots");
    let adds = h.creatures_of(13743);
    assert_eq!(adds.len(), 3, "actual21968 creates three Corrupt Forces");
    for add in &adds {
        assert_eq!(*h.app.world().get::<InMap>(*add).unwrap(), map);
        assert_eq!(
            h.app
                .world()
                .get::<crate::zones::TempSummon>(*add)
                .unwrap()
                .summoner,
            boss
        );
    }
    let death_position = *h.app.world().get::<Position>(boss).unwrap();
    h.hit_pct(boss, 200.0);
    h.advance(0.1);
    assert!(
        h.creatures_of(13743).is_empty(),
        "death cleans corruption summons"
    );
    let redeemed = h.creatures_of(13716);
    assert_eq!(
        redeemed.len(),
        1,
        "source death creates Celebras the Redeemed"
    );
    let redeemed = redeemed[0];
    assert_eq!(*h.app.world().get::<InMap>(redeemed).unwrap(), map);
    let position = h.app.world().get::<Position>(redeemed).unwrap();
    assert!(
        Vec3::new(
            position.x - death_position.x,
            position.y - death_position.y,
            position.z - death_position.z
        )
        .length()
            < 0.01,
        "redeemed at death position"
    );
    let template = &h.app.world().resource::<WorldData>().templates[&13716];
    assert_eq!(
        template.health_modifier, 35.0,
        "TDB base redeemed health, not legacy8"
    );
    assert!(
        (template.mana_modifier - 5.3615).abs() < 0.0001,
        "TDB base redeemed mana, not legacy4"
    );
    assert_eq!(
        instance_script::get_boss_state(h.app.world_mut(), map, BIT),
        Some(EncounterState::Done)
    );
    assert_eq!(
        h.app
            .world()
            .resource::<crate::maps::InstanceManager>()
            .completed_encounters(map),
        1 << BIT
    );
    assert_eq!(
        h.inbox(0)
            .ends
            .iter()
            .filter(|end| end.encounter_id == ENCOUNTER && end.success)
            .count(),
        1
    );
    assert!(h.inbox(1).ends.is_empty());
    assert!(
        h.inbox(1)
            .spell_gos
            .iter()
            .all(|go| go.caster != boss.to_bits())
    );
    h.advance(30.0);
    assert_eq!(
        h.creatures_of(13716),
        vec![redeemed],
        "manual-despawn redeemed survives corpse cleanup"
    );
}
