# Voluntary early battleground exit: Deserter

## Scope and authority

TC a352b1fa Player.cpp24304–24328 grants aura26013 after removal/resurrection for ordinary BG Waiting/InProgress voluntary exits. worldserver.conf.dist2773–2778 defaults Battleground.CastDeserter to1. Lead explicitly approved adoption; no new flag. Reuse normal AuraApplication, authored SpellStore duration and expiry. Apply after successful return initiation/membership cleanup; no new deferred-operation or transfer state. Exclude completed BG, arena, declined invitation and refused combat-leave. No reward, wire, persistence or deployment change.

Pinned cached source hashes: engine `data/diagnostics/pvp-voluntary-exit-deserter-2026-10-19/source-receipt.json`.

## Proof ledger

- Base `94b24b70f` after lj649 landed (101/0/check0/fmt0). Previous converse tip ancestry verified before branching.
- Initial ignored tests `03a8efd8d` hit optional SpellMisc compile prerequisite; retained as `prerequisite-red.*`, NOT behavioral RED.
- Fixture `5b5e0dd89` incorrectly required ActiveAuras before any aura; retained as `oracle-red.*`, NOT a valid full RED. Missing component represents no auras; corrected negative assertion reads that representation directly.
- Corrected ignored/pushed RED `2d0be9111`: native `bg_exit_deserter_ -- --include-ignored --nocapture`: **5 genuine missing-aura failures / 4 controls PASS**. Concrete/Random Waiting/InProgress returned actors and a fully resurrected real-death actor lack26013. Completed BG, arena, declined invite and combat-refusal controls pass.
- Production `5a81ff0d1`: capture existing phase/arena before cleanup; grant sourced aura only after successful return boundary. Authored26013 duration is checked from SpellStore; AuraPlugin runs normal expiry without deadline edits/removal.
- GREEN development: initial46/4 exposed missing AuraPlugin; next lifecycle run exposed missing ToyCastingRequirements; one private fixture accessor compile failure and one ground-directory mismatch were retained separately. Lead approved existing public-crate authored loader, no visibility changes/full SpellCastPlugin. Requirements load beside server world.db, matching production world_data.rs1031–1033, not client ground assets.
- `green5`46/4 reached native expiration successfully; final assert incorrectly required a retained queue when existing matchmaking immediately issued an invitation. Corrected observable admission oracle permits matching queue or real unaccepted map2107 invitation; no producer change.
- `green6` on `4d25f41ddfb319215412e78df1c333cdfd37aea9`: **50 PASS / 0 FAIL / 0 ignored**, one scoped native invocation (`maps::tests::battleground_nodes::deserter::`). Nine new +41 retained cases. Concrete/Random Waiting/InProgress grant/block/normal-authored-expiry/readmission, dead exit after full resurrection, all four negative controls pass.
- Latest master merged as `0967af200` after GREEN6; merged aura/combat changes intersected proof scope. `green7` on `4a265f2a95e18877b0edf3ef63a5728548e8cabb`: **50 PASS / 0 FAIL / 0 ignored**, same scoped native filter, current merged code. Later docs-only commits do not invalidate this proof. Changed Rust formatted, manual changed-line readability clear; Lead owns final broad/check/fmt acceptance. No live/deployed claim.

Full argv, stdout/stderr and final ledger live under engine `data/diagnostics/pvp-voluntary-exit-deserter-2026-10-19/`. CPU-only native-channel proof; no live/deployed claim. Lead owns broad/check/fmt acceptance.
