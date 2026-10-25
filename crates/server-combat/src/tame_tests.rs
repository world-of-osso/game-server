//! Real-DB tests against data/world.db.

use std::collections::HashMap;
use std::sync::OnceLock;

use rusqlite::{Connection, OpenFlags};

use super::*;
use crate::world_data::{load_expected_stats, load_templates};

/// Diseased Young Wolf: Beast, family 1 (Wolf), TypeFlags TAMEABLE.
const DISEASED_YOUNG_WOLF: u32 = 299;
/// Devilsaur: Beast, family 39 (Devilsaur), TypeFlags TAMEABLE | TAMEABLE_EXOTIC.
const DEVILSAUR: u32 = 6498;
/// Razorbeak Skylord: Beast, family 160 (Gryphon), no TAMEABLE type flag.
const RAZORBEAK_SKYLORD: u32 = 2659;
/// Hogger: Humanoid.
const HOGGER: u32 = 448;
/// Old Blanchy: a tameable Beast (family 299) in Retail, a Critter in AzerothCore.
const OLD_BLANCHY: u32 = 582;

fn templates() -> Option<&'static HashMap<u32, CreatureTemplate>> {
    static TEMPLATES: OnceLock<HashMap<u32, CreatureTemplate>> = OnceLock::new();
    Some(TEMPLATES.get_or_init(|| {
        let path = crate::test_data::world_db_path();
        let conn = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        load_templates(&conn, &load_expected_stats(&conn))
    }))
}

/// A level 20 hunter without a pet stable, charm, or Beast Mastery's exotic taming.
fn hunter() -> TameHunter<'static> {
    TameHunter {
        level: 20,
        can_tame_exotic: false,
        pet_stable: None,
        known_spells: &[],
        has_charm: false,
    }
}

fn target(template: &CreatureTemplate) -> TameTarget<'_> {
    TameTarget {
        template,
        level: 20,
        owned: false,
    }
}

#[test]
fn hunter_tames_a_tameable_beast() {
    let Some(templates) = templates() else { return };
    assert_eq!(
        can_tame(&hunter(), &target(&templates[&DISEASED_YOUNG_WOLF])),
        Ok(())
    );
}

#[test]
fn exotic_beast_needs_exotic_taming() {
    let Some(templates) = templates() else { return };
    let devilsaur = target(&templates[&DEVILSAUR]);
    assert_eq!(
        can_tame(&hunter(), &devilsaur),
        Err(TameFailReason::NotTameable)
    );
    let beast_mastery = TameHunter {
        can_tame_exotic: true,
        ..hunter()
    };
    assert_eq!(can_tame(&beast_mastery, &devilsaur), Ok(()));
}

#[test]
fn beast_without_the_tameable_flag_is_not_tameable() {
    let Some(templates) = templates() else { return };
    let exotic_hunter = TameHunter {
        can_tame_exotic: true,
        ..hunter()
    };
    let skylord = target(&templates[&RAZORBEAK_SKYLORD]);
    assert_eq!(
        can_tame(&exotic_hunter, &skylord),
        Err(TameFailReason::NotTameable)
    );
}

#[test]
fn humanoid_is_not_tameable() {
    let Some(templates) = templates() else { return };
    let exotic_hunter = TameHunter {
        can_tame_exotic: true,
        ..hunter()
    };
    let hogger = target(&templates[&HOGGER]);
    assert_eq!(
        can_tame(&exotic_hunter, &hogger),
        Err(TameFailReason::NotTameable)
    );
}

#[test]
fn retail_beast_azerothcore_lists_as_critter_is_tameable() {
    let Some(templates) = templates() else { return };
    assert_eq!(
        can_tame(&hunter(), &target(&templates[&OLD_BLANCHY])),
        Ok(())
    );
}

#[test]
fn target_above_the_hunter_level_is_too_high() {
    let Some(templates) = templates() else { return };
    let wolf = TameTarget {
        level: 21,
        ..target(&templates[&DISEASED_YOUNG_WOLF])
    };
    assert_eq!(
        can_tame(&hunter(), &wolf),
        Err(TameFailReason::TooHighLevel)
    );
}

#[test]
fn hunter_with_a_pet_out_already_has_a_summon() {
    let Some(templates) = templates() else { return };
    let stable = PetStableSlots {
        pet_out: true,
        active: [true, false, false, false, false],
    };
    let hunter = TameHunter {
        pet_stable: Some(&stable),
        known_spells: &CALL_PET_SPELLS,
        ..hunter()
    };
    let wolf = target(&templates[&DISEASED_YOUNG_WOLF]);
    assert_eq!(
        can_tame(&hunter, &wolf),
        Err(TameFailReason::AlreadyHaveSummon)
    );
}

#[test]
fn free_slot_needs_its_call_pet_spell() {
    let Some(templates) = templates() else { return };
    let stable = PetStableSlots {
        pet_out: false,
        active: [true, false, false, false, false],
    };
    let wolf = target(&templates[&DISEASED_YOUNG_WOLF]);
    let call_pet_1_only = TameHunter {
        pet_stable: Some(&stable),
        known_spells: &CALL_PET_SPELLS[..1],
        ..hunter()
    };
    assert_eq!(
        can_tame(&call_pet_1_only, &wolf),
        Err(TameFailReason::TooManyPets)
    );
    let call_pet_2 = TameHunter {
        known_spells: &CALL_PET_SPELLS[..2],
        ..call_pet_1_only
    };
    assert_eq!(can_tame(&call_pet_2, &wolf), Ok(()));
}

#[test]
fn full_active_slots_are_too_many_pets() {
    let Some(templates) = templates() else { return };
    let stable = PetStableSlots {
        pet_out: false,
        active: [true; MAX_ACTIVE_PETS],
    };
    let hunter = TameHunter {
        pet_stable: Some(&stable),
        known_spells: &CALL_PET_SPELLS,
        ..hunter()
    };
    let wolf = target(&templates[&DISEASED_YOUNG_WOLF]);
    assert_eq!(can_tame(&hunter, &wolf), Err(TameFailReason::TooManyPets));
}

#[test]
fn charmed_creature_blocks_taming() {
    let Some(templates) = templates() else { return };
    let hunter = TameHunter {
        has_charm: true,
        ..hunter()
    };
    let wolf = target(&templates[&DISEASED_YOUNG_WOLF]);
    assert_eq!(
        can_tame(&hunter, &wolf),
        Err(TameFailReason::AlreadyHaveCharm)
    );
}

#[test]
fn owned_creature_is_already_controlled() {
    let Some(templates) = templates() else { return };
    let wolf = TameTarget {
        owned: true,
        ..target(&templates[&DISEASED_YOUNG_WOLF])
    };
    assert_eq!(
        can_tame(&hunter(), &wolf),
        Err(TameFailReason::CreatureAlreadyOwned)
    );
}
