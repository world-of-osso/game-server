//! Production level-70 follower equipped Tome of Unstable Power procs.
use super::follower_dps_diagnostic::*;
use super::*;
use shared::protocol::CombatLogKind;

const DRIVER: u32 = 432777;
const BARRAGE: u32 = 434021;

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn follower_devourer_equipped_tome_delivers_unstable_barrage() {
    let _seed = crate::random::seed_scope(70448);
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("snapshot path");
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2515).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 1480, 70);
    for _ in 0..5 {
        app.update();
    }
    let driver = app
        .world()
        .get::<crate::auras::ActiveAuras>(actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == DRIVER)
        .expect("equipped Tome driver")
        .clone();
    assert!(matches!(
        driver.source,
        crate::auras::AuraSource::Equipment(_)
    ));
    assert_eq!(driver.cast_item_level, Some(73));
    let budget = driver
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .amount;
    assert!(budget > 0.0, "item-scaled native damage budget");
    crate::combat_refs::set_in_combat_with(app.world_mut(), actor, target);
    let barrage = |app: &App| {
        app.world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .find(|event| {
                event.source == Some(actor.to_bits())
                    && event.spell_id == Some(BARRAGE)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            })
            .cloned()
    };
    // Native RPPM1.5 needs simulated combat time; real Consume hits are the proc events.
    for _ in 0..240 {
        crate::spell_cast::cast_triggered(app.world_mut(), actor, 473662, Some(target), 1);
        for _ in 0..20 {
            app.update();
        }
        if barrage(&app).is_some() {
            break;
        }
    }
    let event = barrage(&app).expect("equipped native driver delivers Unstable Barrage");
    let versatility = app
        .world()
        .get::<crate::stats::CharacterStats>(actor)
        .unwrap()
        .versatility_pct;
    let base =
        event.amount as f32 / (1.0 + versatility / 100.0) / if event.crit { 2.0 } else { 1.0 };
    // SimC76f8118 unique_gear_dragonflight.cpp7438–7445: 0.9, 1.0 or 1.1 of the budget.
    let ratio = base / budget;
    assert!(
        [0.9, 1.0, 1.1]
            .iter()
            .any(|step| (ratio - step).abs() < 0.02),
        "Barrage scales the native budget by one image step: ratio {ratio}, amount {}, budget {budget}",
        event.amount
    );
}
