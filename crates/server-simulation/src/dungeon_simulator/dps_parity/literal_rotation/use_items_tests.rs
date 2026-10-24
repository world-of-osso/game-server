//! SimC76f8118 player.cpp10221–10480: first ready item proxy in slot order.
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "native use_items selection: two actually equipped ready on-use trinkets"]
fn literal_use_items_native_selects_first_ready_equipped_slot() {
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
    let mut driver = Driver::new(Program::parse(&[("default", "use_items")]).unwrap());
    let (action, execution) = driver
        .select(
            "default",
            &std::collections::BTreeMap::<String, f64>::new(),
            &prepared,
            &equipment,
        )
        .unwrap()
        .expect("use_items must select the first native ready equipped item");
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
        vec![uses[0].1.on_use.spell_id],
        "one source use_items executes only the first ready native slot, not an instant batch"
    );
}
