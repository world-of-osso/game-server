# Combat pet lifecycle

Verified: 2026-10-11, pet-lifecycle serverc4ba3ef/protocol203acd3; native runtime serverf771781/client16fdd7f18. Requirements live in [pet lifecycle](../../specs/pet-lifecycle.md); this is bounded implementation/proof, not complete Retail parity.

## Durable state

`pet_stables_v2` stores five active records plus200 stabled positions. Records retain identity/name/level/family/stance, current health and optional hunter specialization. `pet_stables` remains read-only migration input: both the stance-bearing and older pre-stance encodings convert on first read, then the new table takes precedence. Legacy/new records without saved health initialize from actual pet stats at spawn; subsequent spawns preserve saved health (bounded by current maximum), including zero.

Family default specialization comes from the local `creature_family`/`chr_specialization` join, excluding PetOverrideSpec0x20 as in pinned TrinityCore DB2Stores. Demon records do not receive hunter specs. Changed pet health saves after combat ticks; the queued player logout/despawn boundary takes an immediate snapshot before marking the owner pending removal. Dismiss snapshots health before removing the world pet.

## Missing-storage Call Pet — 2026-10-16

Native BM character47/Bhunbeastm repeatedly received883 CastSuccess without a pet. A unit probe against a copied actual private redb confirms no stored pet record or active callslot1. On login `load_pet_stables` therefore inserts no `PetStable`; `check_summon_pet` incorrectly admitted that absence, while `effect_summon_pet` found no stored slot and returned without spawning/replicating a pet. e016950c rejects it before success with "You do not have a pet to summon". Actual native-target no-storage RED→GREEN plus empty-slot/dismiss/re-call controls pass4; an actual-character47 snapshot probe separately proves the storage and failure/spawn result. Diagnostic probe source/log stay in bot-harness evidence, not permanent environment-dependent tests. Fresh native `native-bm-no-storage1` on built server0f04b64f confirms five exact no-pet errors from five requests in10s, zero883 CastSuccess and no pet in37samples. Native refusal is proven; a functioning pet/rotation is still blocked by the named source gap.

Starting-pet gap: pinned TDB1210.26091 creation-cast table31 rows has no Human-race-bit1/Hunter-class-bit4 match. The importer has no separate starter-pet mapping; `new-character-kit.md` already lists creation-cast pets as unmodelled. **HUMAN_HUNTER_STARTING_PET_SOURCE** remains explicit: no arbitrary pet grant, invented race mapping or changed bot kit. `bm-starter-pet-source-proof.json` retains the extracted SQL/hash and actual storage probe.

## Hunter Focus timing — 2026-10-18

The called wolf previously used the player `PowerType` Focus rate (native70448: 5/s), yielding 76 after a genuine 25-Focus Bite during several 50ms delivery frames. The unchanged Bite assertion reproduced on c67f8c863 (0PASS/1FAIL); a separate native called-wolf boundary regression reproduced raw76.25 instead of75 on bdd448d45. The cost is unchanged: native `SpellPower` row706, spell17253, Focus cost25.

Pinned TC a352b1fa separates pets: `CreatureData.h423` defines4000ms, `Pet.cpp64,671-687` starts/advances the timer even while full, and `Creature.cpp964-998` adds24 times the Focus multiplier then converts to integer. Native spawn now attaches `PetFocusRegen` only to Focus pets. The generic regeneration system advances that timer instead of applying continuous player Focus; authoritative discrete updates advertise zero continuous prediction. The timer carries phase and does not replay missed ticks after large lag. Player and demon paths remain unchanged. The normal world rate is1; configurable world rates and flat aura85 are not added by this bounded fix.

`pet_control_tests::called_wolf_focus_regenerates_in_four_second_steps_without_resetting_at_full` covers no fractional early refill, first24 tick, second tick clamping, and spending after a full-pool interval without resetting its phase. At merged3cc539edd (includes master8d88013ed), exact unchanged Bite75, the new interval fixture and existing playerFocus5/s control pass3/3 in62.27s with no warnings. Retained pet controls and independent lead gate pending; no UDP/rendered/full-class claim. Source/log receipts: `data/class-completeness6-2026-10-12/bite-focus-*` and `tc-a352b1fa/CreatureData.h.receipt.json` in the assigned s6 evidence tree. Requirements: [pet control](../../specs/pet-control.md).

## Stable operations

`PetStableRequest` runs after NPC hello/gossip/close validation. Only an in-world Hunter connection with the matching StableMaster role can list or mutate its own stable. Reliable ordered replies carry actual records, family/name/icon metadata, imported non-override specs and unlocked Call Pet slots.

Swap preserves the summoned entity when moving among unlocked active positions; moving it into storage unsummons it while retaining health. Rename updates stored and live NPC identity; specialization updates the record. Abandon removes the selected record, not other owned pets. Lists do not rewrite unchanged storage. Same-slot moves are harmless. Closed interaction, invalid/locked positions, invalid names and unsupported specs fail without mutating records.

The real Erma menu9821 includes StableMaster option0 plus separate battle-pet healing options. Tests route the authored combat-pet option instead of assuming a single-entry gossip menu. HostClient replies are collected in First before Lightyear's Last clear.

## Revive and Mend

Local Revive982 effect109 is a once-per-cast handler. Its cast check rejects a living current pet; without one it requires the first dead active record, not a stabled record. Completion recalls/spawns that dead pet near the owner, clears corpse/death/combat state, restores the authored percentage of maximum health, resends its bar and persists the health. This matches pinned TC EffectResurrectPet/CheckCast rather than inventing a last-used-slot rule.

Mend136 already uses aura20's imported10% maximum-health tick every2s. A numerical fixture now proves whole-HP healing and the owner's periodic combat-log event; no Mend implementation change was needed.

## Proof and remaining scope

Baseline632f3aea restored both wounded23 and dead0 pets at full378. At8358c09 all11 hunter tests pass, including reopened-redb wounded/dead health and health-change→queued logout→flush without a fixed tick. Earlier live audit confirms hunter tame/relog/dismiss/call and native Imp summon/relog/menu-dismiss, but only the baseline live binary was exercised.

At5af60fe hunter15/15 pass without warnings: real NPC hello/gossip selection, stable→rename→spec→redb reopen→active→Call→abandon→reopen, active-slot ownership preservation, same-slot move and non-mutating refusals. Protocol203acd3 concrete operation/name/spec/dead-health wire roundtrip passes1/1. Revive source5150fd4 passes lethal-damage/clear-corpse/reopen, first-dead-active selection and living-pet refusal (18 hunter cases pass; the initially fractional Mend assertion was wrong). Focused Mend passes1/1 at9c3e7d7 after matching whole-HP/log contracts. Atc4ba3ef, each authored Call Pet1–5 spell loads its own occupied active position (one real-data fixture covering all five); existing Fel Domination333889 cast-time modifier/next-summon charge also passes1/1. Demon persistence proof lives in [[warlock-pets]]. No shared realm or world.db writes.

## Native hunter proof

Private serverf771781/protocol203acd3/client16fdd7f18: real Erma gossip opens authoritative records in both skins. Native mouse controls store at slot6, rename Ash/Coal, choose Cunning79/Ferocity74, then move to active0. Rename initially reproduced unresolved StableRenameNineSlice; client16fdd7f18 registers the missing Plain metal style. Call883 and native PetFrame Dismiss restore/clear the current bar; connection-reopen captures retain both states.

Real private admin damage3000 invokes the production damage path on the owned wolf. Modern rendered health fraction0.82857→0.92857 and Forever0.82895→0.92763 prove one Mend tick (pixel widths are quantized; server fixture proves exact whole HP). Lethal999999 produces a dead pet; native Log Out confirms CharacterSelect before reconnect. Death persists, Revive982 restores full health, then a second confirmed Log Out preserves revived health in both skins.

Early diagnostic reconnect freed GameClient before the old netcode connection finished disconnecting; intermittent casts after overlapping login had no completion. This is not certified as product failure or success. Final fixtures use the actual user logout boundary, not an arbitrary disconnect delay. Feed food-classification provenance and absent/out-of-coverage demon content remain explicit data boundaries.

## Sources

- [Storage](../../../crates/server/src/persistence_pets.rs), [runtime](../../../crates/server/src/hunter_pet.rs), [stable operations](../../../crates/server/src/pet_stable.rs), [logout](../../../crates/server/src/networking.rs), [behavioral tests](../../../crates/server/src/hunter_pet_tests.rs).
- Installed Retail `Blizzard_APIDocumentationGenerated/PetConstantsDocumentation.lua`:5 active,200 stabled,205 total.
- Local world.db: Wolf family1→Ferocity74; override specs535/536/537 have flag0x20.
- Pinned TrinityCore a352b1fa DBCEnums/DB2Stores reference under game-engine/data/diagnostics/pet-lifecycle-2026-10-11/reference/.
- Proof ledger/logs: game-engine/data/diagnostics/pet-lifecycle-2026-10-11/; handoff `/home/osso/.worktrees/handoff-pet-lifecycle.md`.

## See Also

- [[warlock-pets]] — controlled demon lifecycle and power model.
- [[spell-casting]] — cast/effect ownership.
