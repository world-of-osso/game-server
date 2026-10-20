# Ordinary battleground preparation — CPU proof ledger

## Scope and source

[Contract](../specs/pvp-progression.md#ordinary-battleground-preparation); [implementation notes](../wiki/systems/battleground-preparation.md).

Approved source: TC a352b1fa Battleground.cpp100–103/334–445, first actual entrant/current-copy epoch and stages120/60/30/0; preserve authored roster minima separately. Per-map gate callbacks use authored world.db placements only. Cached source hashes, entry sets and TDB/runtime placement counts: canonical engine `data/diagnostics/pvp-bg-preparation-2026-10-19/gate-inventory.json`.

Production change:0f224310f (later source-comment correction only). Protocol stays724ec7109d5261ded0a67753000ed40ad3f0ba17; no protocol edits. Final integration before this ledger:c33070b8d80256756505305678988a8154cc60cf, including latest server master (guild-last-online changes). Build/check/fmt and final integration gate belong to Lead.

## Proof receipts

All argv/stdout/stderr receipts live under canonical engine `data/diagnostics/pvp-bg-preparation-2026-10-19/`. These are separate runs, not one aggregate current-tip pass.

| Revision / receipt | Exact scope | Observed result | Later-change validity |
|---|---|---|---|
|6187bd839 / red.*|ignored `bg_preparation_`, concrete+Random20 native queue/port|0PASS/2FAIL: full roster InProgress instead of Waiting|Genuine behavioral RED before production|
|0f224310f / green.*|same ignored2|2PASS/0FAIL,0.39s|Initial consumer proof; stronger pending-transfer oracle follows|
|4a0f75080 / expanded.*|`bg_preparation_`4|gate lifecycle across10 actual maps and authored loader2PASS; countdown variant2FAIL,84.92s total|Gate proof remains valid: subsequent changes affect default fixture waits/observer, not production callbacks or preparing gate harness|
|063ea0699 / retained.*|`maps::tests::battleground_nodes::`, excluding already-proven `preparation::gates::`|114PASS/25FAIL,38.92s; corrected native concrete/Random clock2PASS|Partial retained evidence, not current-tip unified GREEN; six fixture waits and nineteen EDQUOT/I/O prerequisites then corrected|
|cfb4b413d / failed25.*|only exact25 previous failures,2threads|22PASS/3FAIL,269.47s|Six early-start fixture assumptions fixed through natural121s preparation; two Seething ground-root prerequisites and Silvershard frame-cadence oracle remain at this revision|
|bc3850ae0 / failed3.*|only exact3 previous failures,2threads; `GAME_SERVER_GROUND_DIR=/home/osso/Projects/world-of-osso/game-engine/data`|3PASS/0FAIL,53.66s, no warnings|Final relevant source/fixture proof before unrelated guild-master integration; broader current-tip integration remains Lead-owned|

`failed-25.json` and `failed-3.json` preserve exact harness selections; respective command JSON files preserve all arguments and revisions. Do not rerun previous successful cohorts merely to create a single count.

## Prerequisites and oracle corrections

- `red-prerequisite.*`: first ignored fixture did not compile (missing GameObjectSpawns import). Corrected6187bd839 before genuine RED; never counted as behavioral failure.
- Pending-transfer variant originally timed from before ACK while using whole-second frames. Test observer now records the first actual InMap FixedUpdate frame, without reading preparation internals. Both corrected cases pass in retained.*.
- Nineteen retained failures were EDQUOT122/SQLite I/O during fixture creation. Lead established leaked killed-run tempdirs on24GiB/tmp and removed stale directories; bounded exact25 rerun cleared these prerequisites.
- `failed3-prerequisite.*`: TimeUpdateStrategy is not Clone. Fixture now removes/retains the owned strategy, then restores it, the prior fixed timestep and virtual maximum delta after the real preparation wait. No alternate behavior/default fallback.
- Seething needs the canonical engine data root, not server/data/ground. Silvershard cancellation requires restored original50ms fixture timing after preparation. Exact final3 pass with both qualifications.

## Coverage and limits

Clock coverage: pending transfers alone remain Waiting even after130seconds; first actual arrival starts the clock; later full-roster arrival cannot reset it; Waiting refuses objective interaction; normal post-start interaction works. Concrete and Random32-selected Arathi use actual20-player parties. Existing undersized Random Arathi still retains its authored8/team minimum.

Gate coverage: actual placements, closed/open collision and animation, normal first-arrival clock, source delayed removal/retention and untouched same-map other copy for WSG2106, AB2107, Twin726, Gilneas761, Eye566, AV30, Isle628, Kotmogu998, Silvershard727 and Deephaul2656. Kotmogu early walls and Isle20-second doors are exercised separately from ordinary3-second removal. Loader checks concrete authored placement counts and source closed states.

Named gap: Seething1803 templates281224/281226 and OnStart exist, but zero imported placements; no fabricated gates. Strand607 remains unimplemented. Countdown UI/wire, Preparation44521, power/reset policy, rewards and arena timing are excluded.

Retained default entry fixtures now wait the actual clock, preserving their original frame settings afterward. Two direct native flag-criteria fixtures also advance the real clock; their unit family awaits Lead's PvP integration gate. Earlier retained cohorts are qualified partial evidence because the final frame-restoration change intersects their shared helper. No claim of one current-tip141-test pass, rendered/UDP behavior, deployment or player collision proof.

Changed Rust was formatted. Manual changed-line audit found no new warning suppressions, fallback paths or nested state-machine redesign. `update_gate` remains one cohesive source deadline/removal/open operation (slightly over30body lines); no unrelated readability refactor added. Whitespace diff check passed before this documentation-only commit.
