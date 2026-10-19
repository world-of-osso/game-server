# Rated arena roster LFG refusal

## Scope

Actual modeled rated2v2/3v3 leader QueueForRatedPvp refuses the complete roster if any member has existing LfgServerState queue or active role check. Use the existing is_queued_or_checking_roles(Entity) method before rated queue mutation. Preserve PvP/LFG. Native dequeue/decline permits normal admission despite retained selection history. Exclude skirmish/SoloShuffle, unconverted rated preferences, converse rated-to-LFG policy, new flags/lifecycles/rewards/penalties/wire/live. `LFG_MATCHED_DUNGEON_SESSION_ADMISSION` remains deferred.

Source TC a352b1fa Group.cpp1250–1253 unconditional isUsingLfg and BattleGroundHandler.cpp558–559 rated group-validator call. Pinned cached hashes in engine `data/diagnostics/rated-arena-lfg-refusal-2026-10-19/source-receipt.json`.

## Evidence

- Base `8d7bc723c` after Deserter de670 landed. Prior feature branches removed after ancestry verification.
- Ignored RED `f6417cd3d1d0bcf9d13492096155c6b6ff2104b5`, pushed before execution. Native `rated_lfg_ -- --include-ignored --nocapture`: **4 genuine failures / 4 controls PASS**. Actual max-level social2/3 roster, last member native LFG queued or 5-player rolecheck; leader admitted rated queue/errorNone instead of refusal. Idle2/3/nonleader/size controls PASS; no prerequisite failures.
- Test-only shared social-party helper accepts party_size; original five-player wrapper/behavior preserved. No source group registry mutation in tests.
- Production guard uses existing optional LFG resource and activity method before rated insertion. GREEN pending; Lead owns broad/check/fmt acceptance. CPU-only, no deployed/live claim.

Full argv/logs and final proof ledger: engine `data/diagnostics/rated-arena-lfg-refusal-2026-10-19/`.
