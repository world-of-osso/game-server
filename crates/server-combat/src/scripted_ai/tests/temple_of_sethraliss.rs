use super::*;

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
            "creature {entry} did not cast {spell}: {:?}",
            h.app
                .world()
                .resource::<crate::scripted_ai::tests::cast_admission::CastAdmissions>()
                .entries
        );
    }
}
