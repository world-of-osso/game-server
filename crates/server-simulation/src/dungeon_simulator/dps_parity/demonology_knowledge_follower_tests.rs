//! Actual follower Hand of Gul'dan drives Demonic Knowledge's shuffled deck.
use super::follower_dps_diagnostic::*;
use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn follower_demonology_hand_of_guldan_draws_rank_two_demonic_knowledge_deck() {
    let _seed = crate::random::seed_scope(70448);
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").unwrap();
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2515).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 266, 70);
    // Other Imp-fade Core procs would obscure the observed deck draws.
    let known: Vec<_> = app
        .world()
        .get::<crate::class_progression::KnownSpells>(actor)
        .unwrap()
        .iter()
        .filter(|&spell| spell != 426115)
        .collect();
    app.world_mut()
        .entity_mut(actor)
        .insert(crate::class_progression::KnownSpells::from_baseline(known))
        .remove::<crate::followers::Follower>();
    let hp = app
        .world_mut()
        .get_mut::<shared::components::Health>(target)
        .unwrap()
        .into_inner();
    hp.current = 1_000_000_000.0;
    hp.max = hp.current;
    assert_eq!(
        app.world()
            .get::<crate::class_progression::TraitLoadout>(actor)
            .unwrap()
            .talent_rank(386185),
        Some(2)
    );
    let count_casts = |app: &App| {
        app.world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| {
                event.source == Some(actor.to_bits())
                    && event.spell_id == Some(105174)
                    && event.kind == CombatLogKind::CastSuccess
            })
            .count()
    };
    let mut wins = 0;
    for _ in 0..80 {
        crate::auras::remove_auras(
            app.world_mut(),
            actor,
            crate::auras::RemoveReason::Cancelled,
            |aura| aura.spell_id == 264173,
        );
        crate::spell_cast::powers::change_power(
            app.world_mut(),
            actor,
            shared::components::PowerType::SoulShards,
            100.0,
        );
        let before = count_casts(&app);
        crate::spell_cast::creature_cast(app.world_mut(), actor, 105174, Some(target), false)
            .unwrap();
        for _ in 0..60 {
            app.update();
            if count_casts(&app) > before {
                break;
            }
        }
        assert_eq!(
            count_casts(&app),
            before + 1,
            "real Hand of Gul'dan must complete"
        );
        wins += u32::from(crate::auras::has_aura(app.world(), actor, 264173));
        for _ in 0..12 {
            app.update();
        }
    }
    assert_eq!(
        wins, 12,
        "rank2 must grant Core on 12 draws in each 80-card deck"
    );
}
