//! Literal Seed→Wither ordering across native expiry and real add replacement.
use super::*;
use crate::auras::ActiveAuras;
use crate::class_progression::TraitLoadout;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

fn wither_applying_casts(run: &Run) -> usize {
    run.app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && matches!(event.spell_id, Some(27243 | 445468))
                && event.kind == CombatLogKind::CastSuccess
        })
        .count()
}

fn assert_absolute_boss_wither_survives(count: usize) {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(265, count).unwrap();
    assert!(
        run.app
            .world()
            .get::<TraitLoadout>(run.actor)
            .unwrap()
            .active_talent_spells()
            .any(|talent| talent.spell_id == 196103),
        "the exact native profile must select Absolute Corruption"
    );
    let boss = run.targets[0];
    let started = run.app.world().resource::<Time>().elapsed();
    install_fixture_lifetimes(&mut run, started);
    engage(&mut run);
    drive_player(&mut run).unwrap();
    let mut retired = Vec::new();
    let mut first_application = None;
    let mut settled_seed_casts = None;
    for frame in 1..=1200 {
        advance_fixture_frame(
            &mut run,
            started + Duration::from_millis(frame * 50),
            started,
            &mut retired,
            &mut |_| {},
        )
        .unwrap();
        let now = crate::spell_cast::now(run.app.world());
        if first_application.is_none() {
            first_application = run
                .app
                .world()
                .get::<ActiveAuras>(boss)
                .and_then(|auras| {
                    auras
                        .iter()
                        .find(|aura| aura.caster == run.actor && aura.spell_id == 445474)
                })
                .map(|aura| (now, aura.expires_at));
        }
        if let Some((applied_at, _)) = first_application {
            // Let already admitted casts/projectiles finish. Do not count those
            // as a refreshability decision made after the actual Wither existed.
            if settled_seed_casts.is_none() && now >= applied_at + 8.0 {
                settled_seed_casts = Some(wither_applying_casts(&run));
            }
        }
    }
    let now = crate::spell_cast::now(run.app.world());
    let (_, original_expiry) =
        first_application.expect("literal paid action must reach owned boss Wither");
    if count == 3 {
        assert!(
            !run.target_waves.is_empty(),
            "reproduce real30s add replacement"
        );
        assert_eq!(
            run.targets[0], boss,
            "the original boss must survive the wave"
        );
    }
    // SimC76f8118 actions1072–1077 (and759–762 for Corruption): selected
    // Absolute makes the DoT outlive the fight. Native18s expiry is not valid.
    assert!(
        original_expiry.is_none_or(|expiry| expiry > now),
        "the first owned Absolute Wither must survive60s, original expiry={original_expiry:?}, now={now}"
    );
    assert_eq!(
        wither_applying_casts(&run),
        settled_seed_casts.unwrap(),
        "an existing Absolute boss Wither cannot become refreshable and reseed later waves"
    );
}

#[test]
#[ignore = "requires exact private265 native player inputs"]
fn seed_absolute_native_single_target_boss_wither_does_not_expire_or_reseed() {
    assert_absolute_boss_wither_survives(1);
}

#[test]
#[ignore = "requires exact private265 native player inputs"]
fn seed_absolute_native_three_target_waves_do_not_refresh_absolute_boss_wither() {
    assert_absolute_boss_wither_survives(3);
}
