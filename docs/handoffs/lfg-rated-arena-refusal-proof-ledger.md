# Ordinary LFG refusal during actual rated arena queues

## Scope

Extend the existing ordinary non-follower QueueForLfg PvP guard only to PlayerPvpQueue::RatedBracket with Arena2v2/Arena3v3. These variants are written after real rated-team queue insertion; no new flags or policy for unconverted brackets. Preserve every member's PvP and LFG on refusal. Native DequeueFromPvp removes the actual team and reconciliation clears its remaining member queue projections, permitting normal LFG admission. Follower dispatch, RatedBattleground/SoloShuffle/other preferences, LFG group/session lifecycle, rewards, penalties, wire and live behavior are excluded. `LFG_MATCHED_DUNGEON_SESSION_ADMISSION` stays deferred.

Source: TC a352b1fa LFGMgr.cpp437–438 (`InBattlegroundQueue`) and group463–464. Pinned cached SHA receipt: engine `data/diagnostics/lfg-rated-arena-refusal-2026-10-19/source-receipt.json`.

## Evidence

- Base `c813d5725061e616de52922ff48f2080aacff950`, after ra674/60b0c4e6d ancestry verified; previous feature branches deleted.
- Ignored native RED `ea24399e2aff60a7c8b27f854e7be11e425256a4`, pushed before execution: `lfg_rated_ -- --include-ignored --nocapture`: **2 genuine failures / 2 controls PASS**. Real max-level social2/3 team admission first; member LFG request returned queued:true/errorNone. Idle ordinary admission and rated mixed-follower-selection dispatch passed. No prerequisite failures.
- Production `fbe490c49` changes only existing ordinary guard match arms. Merged current master as `a21ad291d` before enabling/running GREEN.
- GREEN `d7f50552123396fab07fc0f986defa0c166d384c`: **62 PASS / 0 FAIL / 0 ignored** in one native invocation of `maps::tests::battleground_nodes::deserter:: -- --nocapture` (four new +58 retained). Two actual rated2/3 cases preserve every member's complete PvP/LFG state on refusal, verify authoritative no-LFG queue/rolecheck, then native DequeueFromPvp clears all team queue projections and ordinary LFG admission succeeds. Idle admission and mixed-follower-selection dispatch controls pass; follower dungeon execution is not claimed.
- Changed Rust formatted, manual changed-line readability clear, whitespace diff clear. Later docs-only commits do not invalidate Rust proof. Lead owns broad/check/fmt acceptance; CPU-only proof, no live/deployed claim.

Full commands/logs/final ledger: engine `data/diagnostics/lfg-rated-arena-refusal-2026-10-19/`.
