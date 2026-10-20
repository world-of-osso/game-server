# Ordinary battleground preparation — CPU proof ledger

## Scope and source

[Contract](../specs/pvp-progression.md#ordinary-battleground-preparation); [implementation notes](../wiki/systems/battleground-preparation.md).

Approved source: TC a352b1fa Battleground.cpp100–103/334–445, first actual entrant/current-copy epoch and stages120/60/30/0; preserve authored roster minima separately. Per-map gate callbacks use authored world.db placements only. Cached source hashes, entry sets and TDB/runtime placement counts: canonical engine `data/diagnostics/pvp-bg-preparation-2026-10-19/gate-inventory.json`.

Original production change:0f224310f (later source-comment correction only). Historical receipts below retain their observed pre-rebase revisions. After bp732, Lead required a rebase onto master d331a8b53+; current tested rebase2a6ab113e205ab20017e07105d17ef63b9621958 is based on master fd1aebb202. The owned protocol checkout was explicitly approved to fast-forward from724ec71 to7f9bd7c5e8c4fd5d46c07e7df0b86fceb3605db4 for guild timestamp compatibility; no protocol source edits. PvP production/gate/loader files remain byte-identical to the original2b80f188d submission. Final broad integration/check/fmt remain Lead-owned.

## Proof receipts

All argv/stdout/stderr receipts live under canonical engine `data/diagnostics/pvp-bg-preparation-2026-10-19/`. These are separate runs, not one aggregate current-tip pass.

| Revision / receipt | Exact scope | Observed result | Later-change validity |
|---|---|---|---|
|6187bd839 / red.*|ignored `bg_preparation_`, concrete+Random20 native queue/port|0PASS/2FAIL: full roster InProgress instead of Waiting|Genuine behavioral RED before production|
|0f224310f / green.*|same ignored2|2PASS/0FAIL,0.39s|Initial consumer proof; stronger pending-transfer oracle follows|
|4a0f75080 / expanded.*|`bg_preparation_`4|gate lifecycle across10 actual maps and authored loader2PASS; countdown variant2FAIL,84.92s total|Gate proof remains valid: subsequent changes affect default fixture waits/observer, not production callbacks or preparing gate harness|
|063ea0699 / retained.*|`maps::tests::battleground_nodes::`, excluding already-proven `preparation::gates::`|114PASS/25FAIL,38.92s; corrected native concrete/Random clock2PASS|Partial retained evidence, not current-tip unified GREEN; six fixture waits and nineteen EDQUOT/I/O prerequisites then corrected|
|cfb4b413d / failed25.*|only exact25 previous failures,2threads|22PASS/3FAIL,269.47s|Six early-start fixture assumptions fixed through natural121s preparation; two Seething ground-root prerequisites and Silvershard frame-cadence oracle remain at this revision|
|bc3850ae0 / failed3.*|only exact3 previous failures,2threads; `GAME_SERVER_GROUND_DIR=/home/osso/Projects/world-of-osso/game-engine/data`|3PASS/0FAIL,53.66s, no warnings|Historical focused proof before bp732 integration|
|2a6ab113e205ab20017e07105d17ef63b9621958 / bp732-nine.*|exact3 bp732 failures + four preparation tests + two additional native flag-channel controls;2threads; engine ground root; protocol7f9bd7c|9PASS/0FAIL/0ignored,53.80s, no warnings|Current rebased relevant source/fixture proof before documentation-only update; Lead regate pending|

`failed-25.json` and `failed-3.json` preserve exact harness selections; respective command JSON files preserve all arguments and revisions. Do not rerun previous successful cohorts merely to create a single count.

## Prerequisites and oracle corrections

- `red-prerequisite.*`: first ignored fixture did not compile (missing GameObjectSpawns import). Corrected6187bd839 before genuine RED; never counted as behavioral failure.
- Pending-transfer variant originally timed from before ACK while using whole-second frames. Test observer now records the first actual InMap FixedUpdate frame, without reading preparation internals. Both corrected cases pass in retained.*.
- Nineteen retained failures were EDQUOT122/SQLite I/O during fixture creation. Lead established leaked killed-run tempdirs on24GiB/tmp and removed stale directories; bounded exact25 rerun cleared these prerequisites.
- `failed3-prerequisite.*`: TimeUpdateStrategy is not Clone. Fixture now removes/retains the owned strategy, then restores it, the prior fixed timestep and virtual maximum delta after the real preparation wait. No alternate behavior/default fallback.
- Seething needs the canonical engine data root, not server/data/ground. Silvershard cancellation requires restored original50ms fixture timing after preparation. Exact final3 pass with both qualifications.

## bp732 integration correction and test-driver audit

Lead bp732 on2b80f188d observed278PASS/3FAIL/1ignored, CHECK0/FMT0. No production fix was needed:

- Shared native flag-channel entry now naturally waits preparation before reconnect and physical captures. Two additional tests outside the prior gate filters use the same helper: `maps::tests::ten_warsong_players_accept_over_the_channel_then_ack_one_reserved_map` and `maps::tests::ten_twin_peaks_players_capture_three_source_flags_over_the_channel`.
- Repeated content268 settlements replace BgInstance. The fixture now also initializes its preparation state for each replacement, then advances naturally through each of100 matches. No assertion weakened.
- The honor fixture already passed its InProgress assertion. The actual0-vs60 resource failure came from retaining delta=1second after the natural preparation wait, advancing a newly installed NodeBattle before assault and shifting its source2-second tick. Shared native wait now preserves the caller's original delta without resetting elapsed time.
- Lead-requested grep covered14 BG-driving test modules plus all InProgress/start/objective-update and match-replacement assignments. Other matches use prepared helpers, model-only direct start, existing arena60-second waits or intentional Waiting/inactive eligibility controls. Audit details: `bp732-assumption-audit.md` in the diagnostic evidence directory.
- First rebased nine-test attempt failed compilation only because frozen protocol724 lacked guild `last_logout_unix_secs`; retained as `bp732-nine-prerequisite.*`. Explicitly approved7f9bd7c refresh resolved that prerequisite. The nine exact tests then passed together.

## Coverage and limits

Clock coverage: pending transfers alone remain Waiting even after130seconds; first actual arrival starts the clock; later full-roster arrival cannot reset it; Waiting refuses objective interaction; normal post-start interaction works. Concrete and Random32-selected Arathi use actual20-player parties. Existing undersized Random Arathi still retains its authored8/team minimum.

Gate coverage: actual placements, closed/open collision and animation, normal first-arrival clock, source delayed removal/retention and untouched same-map other copy for WSG2106, AB2107, Twin726, Gilneas761, Eye566, AV30, Isle628, Kotmogu998, Silvershard727 and Deephaul2656. Kotmogu early walls and Isle20-second doors are exercised separately from ordinary3-second removal. Loader checks concrete authored placement counts and source closed states.

Named gap: Seething1803 templates281224/281226 and OnStart exist, but zero imported placements; no fabricated gates. Strand607 remains unimplemented. Countdown UI/wire, Preparation44521, power/reset policy, rewards and arena timing are excluded.

Retained default entry fixtures now wait the actual clock, preserving their original frame settings afterward. Two direct native flag-criteria fixtures also advance the real clock; their unit family awaits Lead's PvP integration gate. Earlier retained cohorts are qualified partial evidence because the final frame-restoration change intersects their shared helper. No claim of one current-tip141-test pass, rendered/UDP behavior, deployment or player collision proof.

Changed Rust was formatted. Manual changed-line audit found no new warning suppressions, fallback paths or nested state-machine redesign. `update_gate` remains one cohesive source deadline/removal/open operation (slightly over30body lines); no unrelated readability refactor added. Whitespace diff check passed before this documentation-only commit.
