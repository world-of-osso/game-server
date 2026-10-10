# Toy Box server foundation

## Contract

- Toy membership comes only from local Retail 12.1.0.69933 Toy.db2. Import every decrypted row, retain unresolved item/spell joins, and expose them as unavailable. No synthetic membership or spells.
- UseItem on a Retail toy learns it for the authenticated account, consuming one physical item in the same redb transaction. Duplicate learning is rejected without consumption. Unsupported activation does not prevent learning.
- Learned state and favourite flags belong to the account, survive database reopen, and reach every online character of that account. Other accounts do not inherit them.
- SetToyFavourite requires learned ownership. World entry and every change deliver a full ToyCollectionUpdate. Catalog entries include item/spell identity, name/icon, source, flags, expansion, learned/favourite state and an explicit unavailable reason.
- UseToy requires learned ownership and item class/race/level eligibility. It dispatches through the existing authoritative spell pipeline, including caster state, target validation, cast time, spell and category history. ItemEffect nonnegative recovery/category durations override SpellCooldowns; negative values retain spell durations. SpellCooldownUpdate reports cooldowns; ToyResult answers start/rejection. SpellGo/CastFailed retain later cast outcome.
- Unresolved spells, unsupported effect/aura types, script-only toy behavior and unsupported ItemEffect conditions/specialization restrictions fail explicitly before starting a cast or cooldown. Zero-entry transform special cases are unsupported, not successful cosmetic no-ops.
- Audited visual-only spells require a nonzero DifficultyID-0 SpellXSpellVisual binding and exclusively zero-valued dummy/script effects or dummy auras, with no trigger, misc payload, item creation or mechanic. They deliver SpellGo with caster/hit targets and apply/replicate/cancel their auras normally. A visual binding alone never authorizes a gameplay script or periodic dummy.
- Temporary wild summons preserve template faction unless SummonProperties overrides it; ally/guardian/companion summons inherit the caster faction. Companions follow their caster, are immune to player/NPC attacks and replace an occupied positive summon slot. Missing creature/property identities and puppet/vehicle/specialized categories remain explicit unavailable reasons.

## Exclusions

Toy Box UI, search/filter/pagination, action-bar persistence, scripted toy actions and rendered client visual proof are separate work. Existing mount/pet character storage is unchanged. Toy cooldowns follow existing character SpellHistory lifetime; persistence of cooldowns across server restart is not provided by this foundation.

## References

- Retail cached `Blizzard_ToyBox.lua`: 229–246 UseToy(itemID), 254–271 account favourites, 135–165 TOYS_UPDATED.
- TrinityCore a352b1fa `WorldSession::HandleUseItemOpcode`, `Player::CastItemUseSpell`, `SpellHistory::GetCooldownDurations`: item-use dispatch and ItemEffect overrides. Generic aura-56 transform handling excludes zero-entry spell-specific costume logic (`auras/unit_state.rs`).
- Implementation and preparation commands: [Toy Box system](../wiki/systems/toys.md).
