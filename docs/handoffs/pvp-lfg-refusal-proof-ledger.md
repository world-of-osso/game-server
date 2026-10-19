# Ordinary BG admission during LFG activity

CPU-only; no live/deployment claim. Branch `pvp-lfg-refusal`, base `ba74ef1df`, frozen protocol `f9a9175`. Production `48b1ca3b5`; master `2e4dfb596` merged in `dbf270af3` before GREEN. Final Rust proof at `5d5edb2a9`.

[Specification](../specs/pvp-progression.md#ordinary-battleground-party-queue). TC a352b1fa Handler130–136, Group1250–1253 and Player.cpp26695–26698 bind LFG refusal. This approved slice covers only actual current queue/role-check activity. The matched-dungeon/session gap remains explicitly named in the specification.

Existing `LfgServerState` is crate-visible with one pure predicate over `queue.is_queued(player.to_bits())` and active `role_checks`. Ordinary BG solo/party validation reads it before any PvP enqueue. No selection-history predicate, new flags or lifecycle. Missing LFG subsystem has no queued/role-check activity. Arena/reward/penalty/wire/live policies unchanged.

## Evidence

Raw commands/revisions, cached source hashes and logs: `game-engine/data/diagnostics/pvp-lfg-refusal-2026-10-19/`.

| Revision | Result | Boundary |
|---|---|---|
| `e90ebd902` |6 harness prerequisite failures, NOT RED | Mandatory existing war-mode GameDatabase resource absent. Preserved `red.*.log`; fixture supplies a private temporary DB, no production fallback. |
| `13d3f3116` | Behavioral RED6 FAIL | Real queued LFG/active role check still allowed ordinary concrete1018/Random32 solo/party queueing, errorNone. `red2.*.log`. |
| `dbf270af3` | GREEN31 PASS/0 FAIL/ignored | New6 plus retained25 Deserter/bracket/entry/exit/combat controls. |
| `5d5edb2a9` | Current GREEN31 PASS/0 FAIL/ignored | Test-reader-only change explicitly checks earlier native LFG errors; same31-case invocation rerun. `green2.*.log`. |

The four queued cases use native QueueForLfg and real group invite/accept for parties, including an active last member. Refusal enqueues nobody and preserves LFG state; authoritative native query still reports queued. Normal DequeueFromLfg then permits the same BG request without residual partial queue entries.

Two actual five-role checks cover solo concrete and party Random admission. Native LFG snapshot proves the queue entries were consumed while the role check remains active. Normal decline removes active role-check membership; selected role/dungeon IDs remain, yet BG joining succeeds. No activity flags are fabricated, and no matched-dungeon/session semantics are inferred.

Focused filter `bg_lfg_`; combined `maps::tests::battleground_nodes::deserter::` (31 cases). RED adds `--include-ignored`; all commands use capacity-admitted `agent-run pvp` and server `scripts/build-host.py --root <slot> --test -p server --bin game-server <filter> -- --nocapture` with frozen sibling/environment overrides.

Changed Rust formatted; manual readability clear. Lead owns check/fmt and independent gate/landing; pending.
