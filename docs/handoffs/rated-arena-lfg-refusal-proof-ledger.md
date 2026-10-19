# Rated arena roster LFG refusal

## Scope

Actual modeled rated2v2/3v3 leader QueueForRatedPvp refuses the complete roster if any member has existing LfgServerState queue or active role check. Use the existing is_queued_or_checking_roles(Entity) method before rated queue mutation. Preserve PvP/LFG. Native dequeue/decline permits normal admission despite retained selection history. Exclude skirmish/SoloShuffle, unconverted rated preferences, converse rated-to-LFG policy, new flags/lifecycles/rewards/penalties/wire/live. `LFG_MATCHED_DUNGEON_SESSION_ADMISSION` remains deferred.

Source TC a352b1fa Group.cpp1250–1253 unconditional isUsingLfg and BattleGroundHandler.cpp558–559 rated group-validator call. Pinned cached hashes in engine `data/diagnostics/rated-arena-lfg-refusal-2026-10-19/source-receipt.json`.

## Evidence

- Base `8d7bc723c` after Deserter de670 landed. Prior feature branches removed after ancestry verification.
- Ignored RED `f6417cd3d1d0bcf9d13492096155c6b6ff2104b5`, pushed before execution. Native `rated_lfg_ -- --include-ignored --nocapture`: **4 genuine failures / 4 controls PASS**. Actual max-level social2/3 roster, last member native LFG queued or 5-player rolecheck; leader admitted rated queue/errorNone instead of refusal. Idle2/3/nonleader/size controls PASS; no prerequisite failures.
- Test-only shared social-party helper accepts party_size; original five-player wrapper/behavior preserved. No source group registry mutation in tests.
- Production `ebfcbfac5`: existing optional LFG resource/activity method before rated insertion, no duplicated activity field.
- Merged current master as `2f1a11386`, enabled tests in `3776bf799a09703c94fa65c03e3d3d657d072ad1` before GREEN.
- Native GREEN on `3776bf799`: **58 PASS / 0 FAIL / 0 ignored**, one invocation of `maps::tests::battleground_nodes::deserter:: -- --nocapture`: eight new +50 retained. Four new refusal cases preserve every player's PvP/LFG state, authoritative queue/rolecheck projection; real DequeueFromLfg or RespondToLfgRoleCheck decline enables same rated roster. Role-check cancellation retains dungeon/role history. Native idle2/3, nonleader and incorrect-size controls pass.
- Changed Rust formatted; manual changed-line readability clear; whitespace diff clear. Later docs-only commits do not invalidate Rust proof. Lead owns broad/check/fmt acceptance. CPU-only, no deployed/live claim.

Full argv/logs and final proof ledger: engine `data/diagnostics/rated-arena-lfg-refusal-2026-10-19/`.
