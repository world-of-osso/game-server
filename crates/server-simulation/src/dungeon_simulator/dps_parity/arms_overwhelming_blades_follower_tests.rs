use super::follower_dps_diagnostic::*;
use super::*;
use crate::auras::ActiveAuras;

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot with Arms follower profile"]
fn follower_arms_overwhelming_blades_applies_overwhelmed_on_reap_and_spin() {
    for selected in [false, true] {
        let _seed = crate::random::seed_scope(70448);
        let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").unwrap();
        let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
        let copy = create_open_follower_copy(&mut app, 2813).unwrap();
        let (mut app, actor, target, _) = prepare_class_follower(app, copy, 71, 90);
        for _ in 0..5 {
            app.update();
        }
        assert!(crate::auras::has_aura(app.world(), actor, 444772));
        if !selected {
            crate::auras::remove_auras(
                app.world_mut(),
                actor,
                crate::auras::RemoveReason::Consumed,
                |aura| aura.spell_id == 444772,
            );
        }
        assert!(crate::spell_cast::try_cast_triggered(
            app.world_mut(),
            actor,
            446005,
            Some(target),
            1
        ));
        let read_stacks = |app: &App| {
            app.world()
                .get::<ActiveAuras>(target)
                .and_then(|auras| {
                    auras
                        .iter()
                        .find(|aura| aura.spell_id == 445836 && aura.caster == actor)
                        .map(|aura| aura.stacks)
                })
                .unwrap_or(0)
        };
        assert_eq!(
            read_stacks(&app),
            u8::from(selected),
            "one Reap impact adds one stack only when selected"
        );
        crate::auras::remove_auras(
            app.world_mut(),
            target,
            crate::auras::RemoveReason::Consumed,
            |aura| aura.spell_id == 445836,
        );
        let action =
            crate::spell_cast::knowledge::resolve_actor_action(app.world(), actor, 227847).unwrap();
        crate::spell_cast::creature_cast(app.world_mut(), actor, action, Some(actor), false)
            .unwrap();
        for _ in 0..500 {
            app.update();
            if !crate::auras::has_aura(app.world(), actor, action) {
                break;
            }
        }
        let hit_count = app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .filter(|e| {
                e.source == Some(actor.to_bits())
                    && e.target == Some(target.to_bits())
                    && e.spell_id == Some(50622)
                    && e.amount > 0
            })
            .count();
        assert!(hit_count > 0);
        // Native 445836 caps Overwhelmed at five stacks.
        assert_eq!(
            usize::from(read_stacks(&app)),
            if selected { hit_count.min(5) } else { 0 },
            "each Bladestorm impact adds one stack only when selected"
        );
    }
}
