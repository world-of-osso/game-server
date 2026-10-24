//! SimC76f8118 player.cpp10221–10480: first ready item proxy in slot order.
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

fn run_use_items_case(rows: &[(&str, &str)], expected_slot: Option<usize>) {
    let (mut profile, traits) = super::super::read_reference(269).unwrap();
    for (slot, id) in [("trinket1", 61034), ("trinket2", 193701)] {
        let item = profile
            .equipment
            .iter_mut()
            .find(|item| item.slot == slot)
            .unwrap();
        item.item_id = id;
        item.bonus_list.clear();
        item.item_level_override = None;
        item.options.clear();
    }
    let identity = super::super::inputs::read_actor(&profile).unwrap();
    let mut run = super::super::prepare_literal_reference(profile, &traits, 1, identity).unwrap();
    let equipment = Equipment::load(run.app.world(), run.actor, &mut Default::default()).unwrap();
    let uses: Vec<_> = equipment.uses().collect();
    assert_eq!(
        uses.iter()
            .map(|(slot, item)| (*slot, item.item_id))
            .collect::<Vec<_>>(),
        vec![("trinket1", 61034), ("trinket2", 193701)]
    );
    let prepared = Prepared::read(
        run.app.world_mut(),
        run.actor,
        &run.targets,
        &equipment,
        std::iter::empty::<execution::NativeAction<'static>>(),
        false,
    )
    .unwrap();
    for slot in ["trinket1", "trinket2"] {
        let explicit = Action {
            name: "use_item".into(),
            target: None,
            options: [("slot".into(), slot.into())].into(),
        };
        assert!(
            prepared.execution(&explicit, &equipment).unwrap().is_some(),
            "actual equipped {slot} on-use must pass native admission first"
        );
    }
    let mut driver = Driver::new(Program::parse(rows).unwrap());
    let selected = driver
        .select(
            "default",
            &std::collections::BTreeMap::<String, f64>::new(),
            &prepared,
            &equipment,
        )
        .unwrap();
    let Some(expected_slot) = expected_slot else {
        assert!(
            selected.is_none(),
            "excluded source items must not leak through use_items"
        );
        return;
    };
    let (action, execution) =
        selected.expect("use_items must select the first native ready equipped item");
    assert_eq!(action.name, "use_items");
    driver
        .execute(run.app.world_mut(), run.actor, action, execution)
        .unwrap();
    run.app.update();
    let casts: Vec<_> = run
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.kind == CombatLogKind::CastSuccess
                && uses
                    .iter()
                    .any(|(_, item)| event.spell_id == Some(item.on_use.spell_id))
        })
        .map(|event| event.spell_id.unwrap())
        .collect();
    assert_eq!(
        casts,
        vec![uses[expected_slot].1.on_use.spell_id],
        "one source use_items executes only the first ready native slot, not an instant batch"
    );
}

#[test]
#[ignore = "native use_items selection: two actually equipped ready on-use trinkets"]
fn literal_use_items_native_selects_first_ready_equipped_slot() {
    run_use_items_case(&[("default", "use_items")], Some(0));
}

#[test]
#[ignore = "native use_items custom slot order must drive the actual on-use cast"]
fn literal_use_items_native_custom_slots_preserve_source_order() {
    run_use_items_case(&[("default", "use_items,slots=trinket2:trinket1")], Some(1));
}

#[test]
#[ignore = "native use_items default check_existing excludes explicit source slots"]
fn literal_use_items_native_skips_explicit_slot_without_rewriting_rows() {
    run_use_items_case(
        &[
            ("default", "use_item,slot=trinket1,if=0"),
            ("default", "use_items"),
        ],
        Some(1),
    );
}

#[test]
#[ignore = "native use_items check_existing0 explicitly includes separately declared items"]
fn literal_use_items_native_check_existing_zero_keeps_the_first_slot() {
    run_use_items_case(
        &[
            ("default", "use_item,slot=trinket1,if=0"),
            ("default", "use_items,check_existing=0"),
        ],
        Some(0),
    );
}

#[test]
#[ignore = "native use_items excludes a named item explicitly covered outside precombat"]
fn literal_use_items_native_named_source_item_is_excluded() {
    run_use_items_case(
        &[
            ("default", "use_item,name=algethar_puzzle_box,if=0"),
            ("default", "use_items,slots=trinket2"),
        ],
        None,
    );
}

#[test]
#[ignore = "native named precombat use must not disable the item's later use_items proxy"]
fn literal_use_items_native_precombat_named_item_remains_available() {
    run_use_items_case(
        &[
            ("precombat", "use_item,name=algethar_puzzle_box,if=0"),
            ("default", "use_items,slots=trinket2"),
        ],
        Some(1),
    );
}

#[test]
#[ignore = "native use_items cannot select items entirely covered by explicit rows"]
fn literal_use_items_native_all_source_slots_excluded_is_unavailable() {
    run_use_items_case(
        &[
            ("default", "use_item,slot=trinket1,if=0"),
            ("default", "use_item,slot=trinket2,if=0"),
            ("default", "use_items"),
        ],
        None,
    );
}
