use super::*;
use crate::simc_apl_native::{Bindings, Encounter, FightStyle, NativeFacts};
use literal_rotation::{
    equipment::Equipment,
    execution::{self, Execution},
};

#[test]
#[ignore = "requires private native252 Heart/Guillotine equipment and world DB"]
fn literal_item_proc_state_tracks_own_native_use_aura_and_expiry() {
    let (mut profile, traits) = read_reference(252).unwrap();
    profile.omnium_talents = None;
    let mut run = prepare_literal_reference(
        profile,
        &traits,
        1,
        crate::followers::simulator::ParityActor::Player { race: 8 },
    )
    .unwrap();
    let mut bindings = Bindings::default();
    let equipment = Equipment::load(run.app.world(), run.actor, &mut bindings).unwrap();
    let encounter = Encounter::Fixture {
        started_at: 0.0,
        duration: 300.0,
        style: FightStyle::Patchwerk,
    };
    let read = |run: &Run, slot: u8, property: &str| {
        let facts = NativeFacts::new(run.app.world(), run.actor, &bindings, encounter, 1);
        equipment
            .fact(&format!("trinket.{slot}.proc.any_dps.{property}"), &facts)
            .unwrap()
    };
    assert_eq!(read(&run, 1, "up"), 0.0);
    assert_eq!(read(&run, 1, "remains"), 0.0);
    let item = equipment
        .uses()
        .find(|(slot, _)| *slot == "trinket1")
        .unwrap()
        .1;
    assert_eq!(item.item_id, 270175, "exact native Heart item");
    execution::execute(
        run.app.world_mut(),
        run.actor,
        Execution::Item(item, run.actor),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    assert_eq!(read(&run, 1, "up"), 1.0);
    let remaining = read(&run, 1, "remains");
    assert!(remaining > 10.0 && remaining <= 20.0);
    assert_eq!(
        read(&run, 2, "up"),
        0.0,
        "Guillotine cannot borrow Heart's stat aura"
    );
    assert_eq!(read(&run, 2, "remains"), 0.0);
    for _ in 0..20 {
        run.app.update();
    }
    assert!(read(&run, 1, "remains") < remaining);
    for _ in 0..420 {
        run.app.update();
    }
    assert_eq!(read(&run, 1, "up"), 0.0);
    assert_eq!(read(&run, 1, "remains"), 0.0);
    crate::auras::apply_aura(
        run.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: run.targets[0],
            target: run.actor,
            spell_id: 1297761,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 1297761));
    assert_eq!(
        read(&run, 1, "up"),
        0.0,
        "a foreign caster cannot supply this item's proc"
    );
    assert_eq!(read(&run, 1, "remains"), 0.0);
}
