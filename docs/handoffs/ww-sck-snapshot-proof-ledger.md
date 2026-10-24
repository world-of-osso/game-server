# Windwalker Unbroken Rhythm SCK snapshot

Approved2026-10-24: `SIMC_WW_UNBROKEN_RHYTHM_SCK_SNAPSHOT_CONSUMPTION`. Created `ww-sck-snapshot` from origin/master13d2f9633 after correcting stale local mastera5a5af6d3 before edits. Requested origin/master4996e5967 merge is b2aeff202; only wiki-index conflict, both entries retained. Cone gatecb1099 is landed in that base; Flurry889cfbf6c remains separate/frozen.

## Source contract

SimC76f8118cf3f7f41dad20708e8328e1f4ab2deebc `engine/class_modules/monk/sc_monk.cpp1526–1532` uses the set buff's effect2 percent and stack count as a persistent SCK-tick multiplier;1641–1660 creates the SCK event before expiring that buff. Native zero-based1297033 effect1=20%, max6 stacks, supplies the value. Native107270 retains AP coefficient0.100000001, variance0.0500000007 and its ordinary geometry/target counts.

Native1296624 aura42 already produces1297033; it is not replaced or narrowed. In particular107270's Attributes3=0x200 (`NOT_A_PROC`) allows its triggered hits through existing `auras/procs.rs296–326`. Do not mislabel1296624's0x100000 as `CAN_PROC_FROM_PROCS`; that0x04000000 flag is on1297033 itself. A newly earned live buff during a channel belongs to later parents, not the current snapshot.

Machine-readable exact excerpts/rows: `game-engine/data/diagnostics/ww-parity-2026-10-23/sck-snapshot-source-packet.json`.

## Native coverage matrix

| Boundary | Baseline b288c1abd | Current qualification |
|---|---|---|
| Earned1297033 consumed once at parent | Genuine RED: buff remains before the first tick | GREEN atb2aeff202; one ordinary removal |
| Actual damage on every real107270 tick | Genuine RED: native60% earned bonus requires38167 before armor, actual23854 | GREEN atb2aeff202; all4 actual ticks retain1.6 effectiveness |
| No-earned parent while native ticks earn new live stacks | PASS, real Chi and native20s expiry | Retained GREEN; all4 ticks stay unboosted |
| Later parent must not reuse earlier earned amount | PASS, real first channel then native expiry and real second cast | Retained GREEN; all4 later ticks stay unboosted |
| Producer remains effective after consumption | First case continues to real tick-driven re-earning after the failed consumption assertion | GREEN: new live native stacks return during real ticks |
| Other107270 parent322729 | Native source edge verified | Emission-scoped implementation; no separate runtime certificate claimed |

Baseline native2RED/2controlsPASS takes14.65s; build470.32s/zero warnings. Tests earn resources/stacks from actual Tiger Palm, observe native expiry, and compare resolved damage against observed native inputs/unaffected source/target products. Crit, variance, armor and legal misses are not rerolled. No fixture aura, stack, power, stat or coefficient grants.

At7331708ee all4 unchanged native cases pass in17.43s; compile778.79s/zero warnings. First earned tick now resolves38167 before armor and27714 actual damage, with the requested1.6 scalar. All four earned tick observations are1.6 within native rounding; no-earned/later-parent observations remain1.0 despite native re-earning. Proof summary: `sck-7331708ee-native-proof-summary.json`; binary SHA-256 `4b39f55dc6fb0f1e2c41ba3520fd4ff0832e2f645dcccea088320fd2d2f05bbf`. This is the pre-merge qualification.

After the requested4996e5967 merge, b2aeff202 retains all4 unchanged native cases GREEN in19.23s; build580.25s/zero warnings, fmt check0. All12 observed tick results retain the same earned1.6/no-earned1.0/later-parent1.0 factors and actual damage as before merge. Current binary SHA-256 `9d2c0d4063fe9a4a4c91a630e1f5d31f0d7f2b512df878c1eaf7b37d64563523`; `sck-b2aeff202-native-proof-summary.json` links exact factors. Only proof documentation follows this qualification; the full lead gate remains separate.

## Implementation representation

`spell_cast/scripts/unbroken_rhythm.rs` stores the native parent aura instance ID and one scalar in a class component. The existing single parent self-hit occurs after aura application and before the first native tick, so it captures the owned buff and consumes it once; native refresh/triggered cast paths use the same hit seam. Root removal clears only the matching instance.

A generic optional periodic-trigger script lookup replaces only a registered emission. Native additive OnEffectPeriodic still runs first; all unregistered parents retain their existing trigger path. Only101546/effect0 registers the new emission. It clones the native child through the existing triggered-spell loader and sets the existing per-cast `cast_damage_multiplier`; no new common snapshot field/protocol or caster-wide107270 damage override is introduced. This matters because both101546 and322729 natively emit107270. Unknown snapshot context is diagnosed, not replaced with guessed damage.

Native producer, flags, periods, coefficients, AP and target counts are unchanged. This slice does not implement recovery, Flurry variants or natural Dual Threat production. Full lead integration gate and broader WW±10% numerical parity remain open.

## Proof artifacts

`game-engine/data/diagnostics/ww-parity-2026-10-23/`: `sck-red-b288c1abd.build*`, `native-ww-sck-red-b288c1abd/`, `sck-green-7331708ee.build*`, `native-ww-sck-green-7331708ee/`, `sck-7331708ee-native-proof-summary.json`, `sck-merged-b2aeff202.build*`, `native-ww-sck-merged-b2aeff202/`, `sck-b2aeff202-native-proof-summary.json`, merged format/readability receipts, source packet and task-local `sck_native_ops.py`. Each native runtime has its own current read-only SQLite backup, exact inputs/binary hashes, argv/environment and complete combined logs. Initial c69 admission exhausted ten HOLD samples before compilation; no native execution occurred at that revision. Builds/tests use admitted `agent-run ww-sck-snapshot` with-j2. No worker push or spawned agents.
