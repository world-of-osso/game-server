//! Which creature a hunter can tame: TrinityCore `spell_hun_tame_beast::CheckCast`
//! (Tame Beast 1515, spell_hunter.cpp:1266-1321) and `CreatureTemplate::IsTameable`
//! (CreatureData.h:547-559). The generic spell target checks (dead target, range, line of
//! sight) are the cast's, not this rule's (`SpellInfo::CheckTarget`).

use crate::world_data::CreatureTemplate;

/// `MAX_ACTIVE_PETS` (PetDefines.h:36).
pub(crate) const MAX_ACTIVE_PETS: usize = 5;
/// Call Pet 1-5, the spell a hunter needs for each active pet slot
/// (`spell_hun_tame_beast::CallPetSpellIds`).
pub(crate) const CALL_PET_SPELLS: [u32; MAX_ACTIVE_PETS] = [883, 83242, 83243, 83244, 83245];

/// `CREATURE_TYPE_BEAST` (SharedDefines.h).
const CREATURE_TYPE_BEAST: u8 = 1;
/// `CREATURE_TYPE_FLAG_TAMEABLE` (SharedDefines.h:5132).
const CREATURE_TYPE_FLAG_TAMEABLE: u32 = 0x0000_0001;
/// `CREATURE_TYPE_FLAG_TAMEABLE_EXOTIC` (SharedDefines.h:5148).
const CREATURE_TYPE_FLAG_TAMEABLE_EXOTIC: u32 = 0x0001_0000;

/// Why Tame Beast fails, with the client error it reports (GlobalStrings).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TameFailReason {
    /// `SPELL_FAILED_HIGHLEVEL` "Target is too high level".
    TooHighLevel,
    /// `SPELL_FAILED_BAD_TARGETS` "Invalid target": not tameable, or exotic without
    /// the ability to tame exotic beasts.
    NotTameable,
    /// `SPELL_FAILED_ALREADY_HAVE_SUMMON` "You already control a summoned creature".
    AlreadyHaveSummon,
    /// `PetTameResult::TooMany` / `PETTAME_TOOMANY` "You have too many pets already":
    /// every active slot holds a pet, or the free slot's Call Pet spell is unknown.
    TooManyPets,
    /// `SPELL_FAILED_ALREADY_HAVE_CHARM` "You already control a charmed creature".
    AlreadyHaveCharm,
    /// `PetTameResult::CreatureAlreadyOwned` / `PETTAME_CREATUREALREADYOWNED`
    /// "Creature is already controlled".
    CreatureAlreadyOwned,
}

/// The hunter's `PetStable`: which active slots hold a pet, and whether one is out.
pub(crate) struct PetStableSlots {
    /// `CurrentPetIndex` is set.
    pub pet_out: bool,
    /// `ActivePets[i]` holds a pet.
    pub active: [bool; MAX_ACTIVE_PETS],
}

pub(crate) struct TameHunter<'a> {
    pub level: u8,
    /// `Player::CanTameExoticPets`: a `SPELL_AURA_ALLOW_TAME_PET_TYPE` (146) aura.
    pub can_tame_exotic: bool,
    /// `None` before the hunter's first pet (`Player::GetPetStable` is null).
    pub pet_stable: Option<&'a PetStableSlots>,
    pub known_spells: &'a [u32],
    /// `GetCharmedGUID` is set.
    pub has_charm: bool,
}

pub(crate) struct TameTarget<'a> {
    pub template: &'a CreatureTemplate,
    /// `Creature::GetLevelForTarget(hunter)`: its scaled level.
    pub level: u8,
    /// `GetOwnerGUID` is set.
    pub owned: bool,
}

impl CreatureTemplate {
    /// `CreatureTemplate::IsExotic`.
    pub(crate) fn is_exotic(&self) -> bool {
        self.type_flags & CREATURE_TYPE_FLAG_TAMEABLE_EXOTIC != 0
    }

    /// `CreatureTemplate::IsTameable`: a Beast with a family and the TAMEABLE type flag,
    /// exotic ones only for a hunter that can tame exotic beasts.
    pub(crate) fn is_tameable(&self, can_tame_exotic: bool) -> bool {
        if self.creature_type != CREATURE_TYPE_BEAST
            || self.family == 0
            || self.type_flags & CREATURE_TYPE_FLAG_TAMEABLE == 0
        {
            return false;
        }
        can_tame_exotic || !self.is_exotic()
    }
}

/// `spell_hun_tame_beast::CheckCast`, in its order.
pub(crate) fn can_tame(hunter: &TameHunter, target: &TameTarget) -> Result<(), TameFailReason> {
    if target.level > hunter.level {
        return Err(TameFailReason::TooHighLevel);
    }
    if !target.template.is_tameable(hunter.can_tame_exotic) {
        return Err(TameFailReason::NotTameable);
    }
    if let Some(stable) = hunter.pet_stable {
        check_pet_slots(stable, hunter.known_spells)?;
    }
    if hunter.has_charm {
        return Err(TameFailReason::AlreadyHaveCharm);
    }
    if target.owned {
        return Err(TameFailReason::CreatureAlreadyOwned);
    }
    Ok(())
}

/// A pet out, or no free active slot whose Call Pet spell the hunter knows.
fn check_pet_slots(stable: &PetStableSlots, known_spells: &[u32]) -> Result<(), TameFailReason> {
    if stable.pet_out {
        return Err(TameFailReason::AlreadyHaveSummon);
    }
    let free_slot = stable
        .active
        .iter()
        .position(|occupied| !occupied)
        .ok_or(TameFailReason::TooManyPets)?;
    if !known_spells.contains(&CALL_PET_SPELLS[free_slot]) {
        return Err(TameFailReason::TooManyPets);
    }
    Ok(())
}

#[cfg(test)]
#[path = "tame_tests.rs"]
mod tests;
