# Windwalker Flurry core proof ledger

Approved2026-10-24: `SIMC_WW_FLURRY_CHARGE_RELEASE`, core earning/release only, on `ww-flurry` from masterd9000b9ce. Cone8aa2465f2 remains frozen separately forcb1099; no source change or rebase there.

## Contract and sources

- Pinned SimC76f8118cf3f7f41dad20708e8328e1f4ab2deebc `engine/class_modules/monk/sc_monk.cpp5148–5176`: selected450615/non-miss; native1H/2H charge counts and One Versus Many critical multiplier. Actual white impacts2256–2271 and Dual Threat2059–2066 own the earning boundary; DT uses the main-hand group.
- FoF execute1817–1826 releases live451021;747–784 reads the stack, expires it, and schedules450617 plus451250 once per charge. **The150ms spacing is a named SimC policy**, specifically779–781 (`i * 150_ms`), not a native-data timing claim.
- Source mappings5948–5957; native450615 effect0/1=1/2, One Versus Many450988 effect0=1 and451021 cap30. Children retain native AP coefficients0.600000024/0.519999981, targeting, damage and AoE handling. No data mutation or coefficient copy into runtime policy.
- Machine-readable excerpts/native rows: `game-engine/data/diagnostics/ww-parity-2026-10-23/flurry-charge-release-source-packet.json`.

## Native capability/proof matrix

| Boundary | Proof before implementation | Current status |
|---|---|---|
| Actual selected white impact → real451021 | bf7551dd7 andec4b89338: actual0 versus native2 | Genuine RED; GREEN pending |
| Actual ordinary451839 impact → real451021 | bf7551dd7 andec4b89338: actual0 versus native2 | Genuine RED; GREEN pending |
| Natural literal FoF →450617/451250 | ec4b89338: actual root executes after real whites, children absent | Genuine RED; GREEN pending |
| Actual avoided white, unselected no-hero input, selected no-charge FoF | ec4b89338:3PASS | Retained controls; no aura/stack/power/stat grants |
| Critical count, native cap and150ms child timing | Native tests continue to these assertions after initial failures | Not yet GREEN |
| Fresh300s ST/3T | Not run for this slice yet | Pending after final GREEN |
| Natural Dual Threat generation | Native451823 DUMMY30%/451835 aura361 lacks a producer consumer | Explicitly unqualified/excluded; the impact test is an ordinary native451839 cast, not proof of automatic DT generation |

Initial bf7551dd7 native batch has2 genuine earning RED, the selected no-charge controlPASS and3 setup failures, **not five feature failures**. Existing FixtureTarget lifetime/role metadata was missing from the literal release/miss cases. Dropping only a granted hero-root purchase also did not unselect its active subtree. ec4b89338 initializes the existing lifetime model and declares the frozen class/spec-only input before native progression. Its actual3RED/3controlsPASS takes41.86s; compile43.98s/zero warnings. No fixture aura, stack, power or stat mutation makes those prerequisites pass.

## Implementation boundary

`spell_cast/scripts/windwalker_flurry.rs` owns every class ID and source policy. Generic committed-white and landed-direct-damage callbacks retain actual hand/critical provenance; no expanded proc protocol or damage-derived crit guess. Native aura application owns real451021 stack growth/capping. The existing FoF AfterCast and generic queued native-trigger seams own release, target lifetime and execution. Children use the retained cast-request primary rather than mutable combat target state; a missing primary is diagnosed after charge consumption, not replaced with an invented recipient.

Stand Ready, Wisdom, High Impact, energy heuristics, APL rows, natural Dual Threat production, SCK snapshot and392993 recovery are not implemented in this slice. SCK and recovery have separate approvals, branches and gates. Full lead integration gate and overall±10% WW parity remain open.

## Evidence

All artifacts live in `game-engine/data/diagnostics/ww-parity-2026-10-23/`: `flurry-red-{bf7551dd7,ec4b89338}.build*`, `native-ww-flurry-red-{bf7551dd7,ec4b89338}/`, pinned source packet and task-local `flurry_native_ops.py`. Each runtime has its own current read-only SQLite backup, exact input hashes, binary SHA-256, argv/environment and full output. Builds/tests use admitted `agent-run ww-flurry` with-j2; no worker push or spawned agents.
