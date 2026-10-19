# Voluntary early battleground exit: Deserter

## Scope and authority

TC a352b1fa Player.cpp24304–24328 grants aura26013 after removal/resurrection for ordinary BG Waiting/InProgress voluntary exits. worldserver.conf.dist2773–2778 defaults Battleground.CastDeserter to1. Lead explicitly approved adoption; no new flag. Reuse normal AuraApplication, authored SpellStore duration and expiry. Apply after successful return initiation/membership cleanup; no new deferred-operation or transfer state. Exclude completed BG, arena, declined invitation and refused combat-leave. No reward, wire, persistence or deployment change.

Pinned cached source hashes: engine `data/diagnostics/pvp-voluntary-exit-deserter-2026-10-19/source-receipt.json`.

## Proof ledger

- Base `94b24b70f` after lj649 landed (101/0/check0/fmt0). Previous converse tip ancestry verified before branching.
- Initial ignored tests `03a8efd8d` hit optional SpellMisc compile prerequisite; retained as `prerequisite-red.*`, NOT behavioral RED.
- Fixture `5b5e0dd89` incorrectly required ActiveAuras before any aura; retained as `oracle-red.*`, NOT a valid full RED. Missing component represents no auras; corrected negative assertion reads that representation directly.
- Corrected ignored/pushed RED `2d0be9111`: native `bg_exit_deserter_ -- --include-ignored --nocapture`: **5 genuine missing-aura failures / 4 controls PASS**. Concrete/Random Waiting/InProgress returned actors and a fully resurrected real-death actor lack26013. Completed BG, arena, declined invite and combat-refusal controls pass.
- Production: capture existing phase/arena before cleanup; grant sourced aura only after successful return boundary. GREEN pending.

Full argv, stdout/stderr and final ledger live under engine `data/diagnostics/pvp-voluntary-exit-deserter-2026-10-19/`. CPU-only native-channel proof; no live/deployed claim. Lead owns broad/check/fmt acceptance.
