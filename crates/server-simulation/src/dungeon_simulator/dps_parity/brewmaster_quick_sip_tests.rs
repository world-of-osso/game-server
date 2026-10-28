//! Quick Sip carries Shuffle extension remainder and purifies the live pool.
use super::*;
use server::test_support::healer_intake::{Hit, HitKind};

#[test]
#[ignore = "requires canonical Brewmaster profile and world database"]
fn shuffle_extensions_clear_quick_sip_thresholds_from_live_stagger() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(268, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    let hit = Hit {
        tick: 0,
        source_guid: 1,
        pack: 0,
        target_slot: 0,
        school: 1,
        raw: 1_000_000.0,
        level_scaling_factor: 1.0,
        kind: HitKind::Melee {
            hand: 0,
            outcome: "Hit".into(),
            level_difference: 0,
            critical_multiplier: 1.0,
            weapon_speed: 2.0,
        },
    };
    crate::combat::replay_healer_hit(run.app.world_mut(), target, actor, &hit).unwrap();
    for (spell, percent) in [(121253, 0.08_f32), (205523, 0.08), (121253, 0.16)] {
        let before = crate::auras::stagger::pool_size(run.app.world(), actor);
        assert!(before > 0.0);
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, spell, Some(target), false)
            .unwrap();
        let after = crate::auras::stagger::pool_size(run.app.world(), actor);
        assert!(
            (before - after - before * percent).abs() < 2.0,
            "spell {spell} must clear {percent}: {before} -> {after}"
        );
        for _ in 0..24 {
            run.app.update();
        }
    }
}
