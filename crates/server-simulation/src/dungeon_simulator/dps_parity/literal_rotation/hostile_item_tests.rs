//! Named item selection must preserve native hostile and self admission.
use super::*;
use crate::item_templates::ItemTemplates;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;
use std::collections::BTreeMap;

fn equipped_named_uses(spec: u32) -> Vec<(String, String)> {
    let run = super::super::prepare_literal(spec, 3).unwrap();
    let equipment = Equipment::load(run.app.world(), run.actor, &mut Default::default()).unwrap();
    let names = &run.app.world().resource::<ItemTemplates>().item_names;
    let uses: Vec<_> = equipment
        .uses()
        .map(|(slot, item)| {
            let name = names
                .iter()
                .find(|(_, ids)| ids.contains(&item.item_id))
                .map(|(name, _)| name.clone())
                .expect("native equipped item name");
            (slot.to_owned(), name)
        })
        .collect();
    assert!(
        !uses.is_empty(),
        "spec{spec} needs an actual native on-use item"
    );
    uses
}

fn exercise_named_uses(spec: u32) {
    let _seed = crate::random::seed_scope(70_448);
    for (slot, name) in equipped_named_uses(spec) {
        let mut run = super::super::prepare_literal(spec, 3).unwrap();
        super::super::engage(&mut run);
        let equipment =
            Equipment::load(run.app.world(), run.actor, &mut Default::default()).unwrap();
        let item = equipment
            .uses()
            .find(|(candidate, _)| *candidate == slot)
            .unwrap()
            .1;
        let spell = run
            .app
            .world()
            .resource::<crate::spell_info::SpellStore>()
            .get(item.on_use.spell_id)
            .unwrap();
        let hostile = matches!(
            crate::spell_cast::targets::explicit_requirement(spell.effects.iter()),
            Some(crate::spell_cast::targets::Relation::Enemy)
        );
        let expected = if hostile { run.targets[0] } else { run.actor };
        let prepared = Prepared::read(
            run.app.world_mut(),
            run.actor,
            &run.targets,
            &equipment,
            std::iter::empty::<execution::NativeAction<'static>>(),
            false,
        )
        .unwrap();
        let row = format!("use_item,name={name}");
        let mut driver = Driver::new(Program::parse(&[("default", row.as_str())]).unwrap());
        let (action, execution) = driver
            .select(
                "default",
                &BTreeMap::<String, f64>::new(),
                &prepared,
                &equipment,
            )
            .unwrap()
            .unwrap_or_else(|| {
                panic!("spec{spec} named native {slot}/{name} must select its admitted use")
            });
        run.app
            .world_mut()
            .resource_mut::<ObservedCombatEvents>()
            .0
            .clear();
        driver
            .execute(run.app.world_mut(), run.actor, action, execution)
            .unwrap();
        super::super::preparation::finish_pending_setup(&mut run).unwrap();
        let events = &run.app.world().resource::<ObservedCombatEvents>().0;
        assert!(
            events
                .iter()
                .any(|event| event.kind == CombatLogKind::CastSuccess
                    && event.source == Some(run.actor.to_bits())
                    && event.spell_id == Some(item.on_use.spell_id)
                    && event.target == Some(expected.to_bits())),
            "spec{spec} native {slot}/{name} must really cast at its native recipient {expected:?}"
        );
        if item.on_use.spell_id == 1297908 {
            assert!(
                events
                    .iter()
                    .any(|event| event.kind == CombatLogKind::Damage
                        && event.spell_id == Some(1297908)
                        && event.amount > 0
                        && event.target == Some(expected.to_bits())),
                "named Font must deliver real hostile channel damage"
            );
        }
    }
}

#[test]
#[ignore = "requires native Havoc profile and gear"]
fn literal_named_on_use_native_havoc() {
    exercise_named_uses(577);
}

#[test]
#[ignore = "requires native Assassination profile and gear"]
fn literal_named_on_use_native_assassination() {
    exercise_named_uses(259);
}

#[test]
#[ignore = "requires native Frost Mage profile and gear"]
fn literal_named_on_use_native_frost_mage() {
    exercise_named_uses(64);
}

#[test]
#[ignore = "requires native Ret profile and gear"]
fn literal_named_on_use_native_retribution() {
    exercise_named_uses(70);
}
