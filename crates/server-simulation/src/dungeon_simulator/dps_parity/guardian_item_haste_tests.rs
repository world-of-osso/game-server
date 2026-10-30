//! Keeper's Seething Core item effect.
use super::*;
use crate::auras::ActiveAuras;
use crate::stats::CharacterStats;

#[test]
#[ignore = "requires Guardian equipped Keeper's Seething Core and native world data"]
fn keepers_core_grants_scaled_haste_through_stack_expiration() {
    use crate::spell_cast::{self, TriggeredSpellOverrides};

    let mut run = prepare_registered_player(104, 1).unwrap();
    let actor = run.actor;
    let driver = run
        .app
        .world()
        .get::<ActiveAuras>(actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 1292065)
        .expect("equipped Keeper's Seething Core");
    let level = driver.cast_item_level.expect("equipped item level");
    assert_eq!(level, 341);
    let rating = |run: &Run| {
        run.app
            .world()
            .get::<CharacterStats>(actor)
            .unwrap()
            .ratings
            .haste
    };
    let baseline = rating(&run);
    let apply = |run: &mut Run| {
        spell_cast::cast_triggered_with_overrides(
            run.app.world_mut(),
            actor,
            1295582,
            Some(actor),
            1,
            TriggeredSpellOverrides {
                item_level: Some(level),
                ..Default::default()
            },
        );
        run.app.update();
    };
    apply(&mut run);
    assert_eq!(rating(&run) - baseline, 131.0);
    for _ in 0..20 {
        run.app.update();
    }
    apply(&mut run);
    assert_eq!(rating(&run) - baseline, 314.0);
    for _ in 0..250 {
        run.app.update();
        let stacks = run
            .app
            .world()
            .get::<ActiveAuras>(actor)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 1295582)
            .map_or(0, |aura| aura.stacks);
        if stacks == 1 {
            break;
        }
    }
    run.app.update();
    assert_eq!(
        rating(&run) - baseline,
        131.0,
        "the first stack expires independently"
    );
    for _ in 0..30 {
        run.app.update();
    }
    assert_eq!(
        rating(&run),
        baseline,
        "the last expiration removes the haste"
    );
}
