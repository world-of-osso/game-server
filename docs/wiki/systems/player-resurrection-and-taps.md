# Player resurrection and creature taps

`DeathPlugin` accepts owner resurrection responses; `CreatureTap` publishes stable tap membership. [Contract](../../specs/player-resurrection-and-taps.md). Verified source audit: 2026-10-06; CPU proof is recorded below separately.

## Resurrection

Effects 18 (RESURRECT) and 172 (RESURRECT_WITH_AURA) enter the normal spell pipeline. Retail 7328/2006/50769 have implicit targets `[0, 0]`; resurrection explicitly requires a dead ally despite those empty target selectors, with existing map/range/LOS checks. A valid offer does not restore health. It stores caster entity/name, spell, cast-time position/map, expiration and the effect's percentage of the target's max health/mana. One unexpired offer exists per target; another cast cannot replace it.

`ResurrectionResponse` echoes caster/spell and acceptance. The authenticated in-world connection selects the target player, never a client-supplied target GUID. Decline/expiry consumes or expires the offer without revival. Accept clears corpse/graveyard/death state, restores health/mana, moves to the held caster position, bumps movement epoch and marks interest/replication. Effect 172 casts its triggered aura after restoration. Every response returns authoritative death state, including expired/missing offers, preventing a client acceptance from waiting forever. Offers are process-local, not persisted.

## Tap design and bandwidth

`UnitTap(Vec<u64>)` contains `CreatureTap.tappers`' stable character IDs, not transient entities or damage amounts. `GroupMemberSnapshot.character_id` permits comparison with the current group even when a tapper is offline/out of interest. The client knows its own selected character ID. Untapped/absent state is available to everyone; otherwise the viewer or any current group member must be a tapper. Existing server kill eligibility and subgroup tap sharing are unchanged.

The list is inserted only when membership changes; repeated damage does not publish the damage budget. Evade/respawn publishes an empty list. No per-viewer flag, polling message or per-tick tap payload is added. Raw identity data is 8N bytes; standard bincode encodes one vector-length varint plus N ID varints (1/3/5/9 bytes each), plus transport/component envelopes. IDs `[42,43]` occupy 3 payload bytes, empty occupies 1. The roster gains one ID varint per member on its existing infrequent updates. A single owner/group ID would lose the existing multi-tapper/subgroup eligibility; entity IDs would fail across reconnects.

## Retail sources

- TrinityCore `a352b1fa`, `src/server/game/Spells/SpellEffects.cpp`, `EffectResurrect` and `EffectResurrectWithAura`: percentage restoration and consent before revival; 113 is CANCEL_CONVERSATION in this Retail source, not historical RESURRECT_NEW.
- Cached Retail `Blizzard_StaticPopup_Game/GameDialogDefs.lua:1138-1172`: RESURRECT Accept/Decline/timeout; its no-sickness variant follows it. The realm's ordinary player spell resurrection does not add healer sickness.
- Cached Retail `Blizzard_UnitFrame/Shared/CompactUnitFrame.lua:561-563,675-677`: non-player-controlled tap check and health RGB 0.9.
- Cached Retail `Blizzard_UnitFrame/Mainline/TargetFrame.lua:307-312`: tap-denied faction/portrait RGB 0.5. This source does not itself turn the target health fill grey; the requested client health behavior uses that tint explicitly.

## Proof

Targeted `rezrtap` tests use the normal spell and host message path, recording exact restored pools before later mana-regeneration ticks. Native GPU equivalence and a live realm are not claimed. Final command/revision/results are maintained in `/tmp/claude/rezrtap-proof.md`.

## Sources

- [Offer handler](../../../crates/server/src/death/resurrection.rs)
- [Tap projection](../../../crates/server/src/creature_tap.rs)
- [Roster identity](../../../crates/server/src/group/registry.rs)
- [Behavioral tests](../../../crates/server/src/spell_cast/rezrtap_tests.rs)

## See Also

- [[networking]] — reliable channels and replicated component boundaries.
- [[player-damage]] — damage records taps before kill rewards.
