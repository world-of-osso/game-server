//! Actual Warrior2067 equipment views through the production aura schedule.
use super::*;
use crate::auras::ActiveAuras;
use crate::class_progression::ActiveSpec;
use crate::stats::EquippedItems;
use shared::protocol::EquipmentSlot;

const ARMS_TWO_PIECE: u32 = 1296643;
const ARMS_FOUR_PIECE: u32 = 1296644;
const PROTECTION_TWO_PIECE: u32 = 1296647;
const PROTECTION_FOUR_PIECE: u32 = 1296648;
const SET_SLOTS: &[EquipmentSlot] = &[
    EquipmentSlot::Head,
    EquipmentSlot::Shoulder,
    EquipmentSlot::Chest,
    EquipmentSlot::Hands,
];

fn advance(run: &mut Run) {
    for _ in 0..6 {
        run.app.update();
    }
}

fn aura_count(run: &Run, spell: u32) -> usize {
    run.app
        .world()
        .get::<ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .filter(|aura| aura.spell_id == spell)
        .count()
}

fn equip_set_slots(run: &mut Run, original: &EquippedItems, keep: &[EquipmentSlot]) {
    let mut equipped = original.clone();
    equipped
        .items
        .retain(|(slot, _, _)| !SET_SLOTS.contains(slot) || keep.contains(slot));
    run.app.world_mut().entity_mut(run.actor).insert(equipped);
    advance(run);
}

#[test]
#[ignore = "actual Arms2067 reference Player equip/unequip; requires native profile inputs"]
fn dps_parity_equipped_set_arms_equip_unequip_and_spec_change() {
    let mut run = prepare(71, 1).expect("actual Arms Player and original2067 identities");
    let original = run
        .app
        .world()
        .get::<EquippedItems>(run.actor)
        .unwrap()
        .clone();
    assert!(
        crate::item_proc_data::has_equipped_bonus(run.app.world(), run.actor, ARMS_TWO_PIECE)
            .unwrap()
    );
    equip_set_slots(&mut run, &original, &[]);
    assert_eq!(aura_count(&run, ARMS_TWO_PIECE), 0);
    assert!(
        !crate::item_proc_data::has_equipped_bonus(run.app.world(), run.actor, ARMS_TWO_PIECE)
            .unwrap()
    );

    equip_set_slots(
        &mut run,
        &original,
        &[EquipmentSlot::Head, EquipmentSlot::Chest],
    );
    assert!(
        crate::item_proc_data::has_equipped_bonus(run.app.world(), run.actor, ARMS_TWO_PIECE)
            .unwrap()
    );
    assert_eq!(
        aura_count(&run, ARMS_TWO_PIECE),
        1,
        "native1296643 after two original set pieces equip"
    );
    assert_eq!(aura_count(&run, ARMS_FOUR_PIECE), 0);
    assert_eq!(aura_count(&run, PROTECTION_TWO_PIECE), 0);

    // Same real Warrior equipment; the ordinary active-spec signal must reconcile bonuses.
    run.app
        .world_mut()
        .entity_mut(run.actor)
        .insert(ActiveSpec(73));
    advance(&mut run);
    assert_eq!(aura_count(&run, ARMS_TWO_PIECE), 0);
    assert_eq!(aura_count(&run, PROTECTION_TWO_PIECE), 1);
    run.app
        .world_mut()
        .entity_mut(run.actor)
        .insert(ActiveSpec(71));
    advance(&mut run);
    assert_eq!(aura_count(&run, ARMS_TWO_PIECE), 1);
    assert_eq!(aura_count(&run, PROTECTION_TWO_PIECE), 0);

    equip_set_slots(&mut run, &original, &[EquipmentSlot::Head]);
    assert_eq!(
        aura_count(&run, ARMS_TWO_PIECE),
        0,
        "removed below native threshold2"
    );
    equip_set_slots(&mut run, &original, SET_SLOTS);
    assert_eq!(aura_count(&run, ARMS_TWO_PIECE), 1);
    assert_eq!(aura_count(&run, ARMS_FOUR_PIECE), 1);
    eprintln!(
        "native Arms2067: 0→2 pieces apply1296643;1 removes;4 restores2/4pc; spec switch selects own bonus"
    );
}

#[test]
#[ignore = "actual Protection2067 reference Player; checks no Arms aura and native2/4 thresholds"]
fn dps_parity_equipped_set_protection_filters_arms_and_tracks_thresholds() {
    let mut run = prepare(73, 1).expect("actual Protection Player/reference gear");
    let original = run
        .app
        .world()
        .get::<EquippedItems>(run.actor)
        .unwrap()
        .clone();
    assert_eq!(aura_count(&run, ARMS_TWO_PIECE), 0);
    assert!(
        crate::item_proc_data::has_equipped_bonus(run.app.world(), run.actor, PROTECTION_TWO_PIECE)
            .unwrap()
    );
    assert_eq!(
        aura_count(&run, PROTECTION_TWO_PIECE),
        1,
        "native Prot2pc, not an Arms spell special case"
    );
    assert_eq!(aura_count(&run, PROTECTION_FOUR_PIECE), 1);
    equip_set_slots(&mut run, &original, &[EquipmentSlot::Head]);
    assert_eq!(aura_count(&run, PROTECTION_TWO_PIECE), 0);
    assert_eq!(aura_count(&run, PROTECTION_FOUR_PIECE), 0);
    equip_set_slots(
        &mut run,
        &original,
        &[EquipmentSlot::Head, EquipmentSlot::Chest],
    );
    assert_eq!(aura_count(&run, PROTECTION_TWO_PIECE), 1);
    assert_eq!(aura_count(&run, PROTECTION_FOUR_PIECE), 0);
    assert_eq!(aura_count(&run, ARMS_TWO_PIECE), 0);
    eprintln!("native Protection2067: own2/4pc only;1 piece removes both;2 restores only2pc");
}
