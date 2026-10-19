# Ordinary LFG admission during PvP

## Scope

Ordinary non-follower QueueForLfg only. Existing PlayerPvpState Battleground/Arena queue or invitation/entered projection refuses admission without changing either subsystem. Native PvP dequeue/decline/leave permits an otherwise eligible join. No follower-route change, unconverted RatedBracket preference policy, new activity flags, LFG group/session lifecycle, wire, rewards or penalties. `LFG_MATCHED_DUNGEON_SESSION_ADMISSION` remains deferred.

Source: pinned TC a352b1fa LFGMgr.cpp437–438 (group analogue463–464); cached source and SHA receipt in engine `data/diagnostics/pvp-lfg-pvp-join-source-2026-10-19/`.

## Evidence

- Base `017a3304f` includes lp641/dde6566cb; no converse implementation before landing.
- Ignored RED `08124c39f`, pushed before execution: `lfg_pvp_ -- --include-ignored --nocapture`, native helper, game-server binary. **9 behavioral failures / 1 follower-selection routing control PASS.** Each real concrete/Random/arena queued/invited/entered actor received LFG queued:true, error:None. No prerequisite failure.
- Production: read existing PlayerPvpState in ordinary dispatch before handle_queue_request mutates LFG; follower branch remains outside this guard. Missing PvP component means no represented activity, as in existing LFG-only fixtures.
- GREEN pending. Lead owns broad/check/fmt acceptance. CPU-only; no live/deployed claim.

Exact argv and full logs: engine `data/diagnostics/lfg-pvp-join-refusal-2026-10-19/{red-command.json,red.stdout.log,red.stderr.log}`. Later docs-only commits do not invalidate Rust proof.
