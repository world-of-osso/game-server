use super::*;

#[test]
fn temple_pair_engages_together_and_completes_only_after_both_die() {
    let (mut h, adderis, map) = bound_boss_at_difficulty(262530, 1877, 0, 8, None);
    let inputs = prepared_inputs::encounter_inputs(262822, 1877, false);
    let template = crate::test_data::world_data()
        .templates
        .get(&262822)
        .unwrap()
        .clone();
    h.app
        .world_mut()
        .resource_mut::<WorldData>()
        .templates
        .insert(262822, template);
    let (aspix, _) = spawn_prepared_actor(&mut h.app, inputs.authored_spawn(8), map, 8);
    h.advance(0.1);
    h.engage(adderis);
    h.advance(0.1);
    let player = h.player();
    for boss in [adderis, aspix] {
        assert!(
            h.app
                .world()
                .get::<shared::components::ThreatTable>(boss)
                .is_some_and(|threat| threat
                    .entries()
                    .iter()
                    .any(|entry| entry.entity == player.to_bits())),
            "both bosses must enter the player's combat"
        );
    }
    crate::combat::deal_direct_damage(h.app.world_mut(), player, adderis, 1_000_000_000.0);
    assert_eq!(
        h.app
            .world()
            .resource::<crate::maps::InstanceManager>()
            .completed_encounters(map)
            & 1,
        0,
        "the first death must not complete the pair"
    );
    crate::combat::deal_direct_damage(h.app.world_mut(), player, aspix, 1_000_000_000.0);
    assert_eq!(
        h.app
            .world()
            .resource::<crate::maps::InstanceManager>()
            .completed_encounters(map)
            & 1,
        1,
        "the second death completes the pair"
    );
}

#[test]
fn temple_keystone_bosses_cast_observed_openers() {
    for (entry, bit, spell, elapsed) in [
        (262822, 0, 1288457, 5.0),
        (262530, 0, 1288428, 47.0),
        (133384, 1, 1290797, 8.0),
        (263658, 2, 1290531, 30.0),
    ] {
        let (mut h, boss, _) = bound_boss_at_difficulty(entry, 1877, bit, 8, None);
        let spells = crate::spell_cast::test_harness::world_db()
            .unwrap()
            .spell_store()
            .clone();
        h.app.insert_resource(spells);
        let mut visibility = crate::interest::NetworkVisibility::default();
        visibility.gain_visibility(h.players[0].1);
        h.app.world_mut().entity_mut(boss).insert(visibility);
        let floor = h.app.world().get::<Position>(boss).unwrap().y;
        h.app
            .insert_resource(crate::ground::GroundMap::flat_arena(floor));
        let player = h.player();
        let mut health = h.app.world_mut().get_mut::<Health>(player).unwrap();
        health.max = 1_000_000_000.0;
        health.current = health.max;
        let origin = h.app.world().resource::<Time>().elapsed_secs_f64();
        h.app
            .insert_resource(crate::scripted_ai::tests::cast_admission::CastAdmissions {
                actor: boss,
                origin,
                entries: Vec::new(),
            });
        h.engage(boss);
        h.advance(elapsed);
        assert!(
            h.inbox(0)
                .spell_gos
                .iter()
                .any(|go| go.caster == boss.to_bits() && go.spell_id == spell),
            "creature {entry} did not cast {spell}: admissions={:?} cast={:?} gos={:?}",
            h.app
                .world()
                .resource::<crate::scripted_ai::tests::cast_admission::CastAdmissions>()
                .entries,
            h.app.world().get::<shared::casting::CastState>(boss),
            h.inbox(0).spell_gos
        );
    }
}
