# Shared resumable precombat polling

Verified2026-10-23: actual85 native cursor/while-casting2RED →3d32d68bf2GREEN27.11s, plus23 relevant native regressions GREEN. Parent prioritized14779's blocked Fire caller before the remaining WW gap. Previous package96b27GREEN is separate proof, not qualification of this new API.

## Crate split B integration

Masterd675bf8fb moves the fixture module into `crates/server-simulation/src/dungeon_simulator/`; test function names remain unchanged. Server core has no dungeon_simulator dependency. Shared literal fixture identity remains `server::test_support::LiteralParityFixture`; actual GCD observation is available under test/test-support. Native geometry reach is re-exported through test_support (navigation module stays private), and the actual server-only Moving component is public for native input/state observation.

First requested two-package --tests check atfb0bca6a6 exits101 in8m45 with six migration visibility/import errors (three navigation references, three Moving accesses), not a native behavioral RED. Integration fixes those references/visibility without changing movement behavior;6e3f31263 repeats the requested check successfully in24.23s, with one scaffold warning that Waiting is never constructed. The native boundary assertion now supplies an explicit expected public Waiting state rather than only a pattern; no wait behavior is implemented before RED and no warning is suppressed. Warning-clean85a3a1c20 repeat check passes both requested packages in2.68s; no suppression. This is compile proof before polling implementation, not native behavioral qualification.

## Caller and proposed contract

Forwarded origin/rotation-b-fire-literal543e7aca4 currently pins88rows/8lists and a native-order test/spec; its Rotation/FireFacts hookup remains14779-owned. Rotation owns Driver; dps_parity::advance_combat owns Run.app. Shared `Driver::poll_precombat(...) -> PrecombatPoll::{Waiting, Selected { action, execution }, Finished}` must distinguish native cast/GCD wait from exhausted source, preserving cursor, variables and original Action.options while waiting. Caller advances real App frames and resamples Prepared/facts, never sleeps or instant-loops casts.

The caller starts its300s fight epoch after setup. Finished does not imply no missile: Pyroblast11366's actual native flight is read through7826726dc (accepted here as86eba5bab), not a precast option, elapsed-time estimate or new timer. Actual Fire precombat rows are Arcane Intellect, variable value/reset/default/set rows, snapshot, Mirror Image, plain Pyroblast; there are no cast/travel options on those casts.

Combat Fire Blast/Combustion rows require declared use_off_gcd1/use_while_casting1 in addition to native permission. Neither flag can waive native cast/GCD refusal or interrupt an existing Pyroblast. Item names/potion/PI options remain attached to Action; native inventory/provider gaps must not become invented buffs or a Fire-only executor.

## Staged native controls

`literal_rotation/precombat_poll_tests.rs` uses actual prepared Player63/current private DB and real native admission before assertions. All stay ignored.

- Arcane Intellect starts native GCD; next poll must wait rather than consume remaining rows/variables or finish.
- Resume through source-plain Mirror Image→real hardcast11366, with constant/reset/default/set/snapshot fixture rows testing the shared cursor contract. No skipped/replayed native CastSuccess; actual cast duration and drained cast/GCD required. This focused boundary does not claim all88 FireFacts/source rows.
- After Finished, record actual782 flight/remains; a hardcast before launch is not flight. Caller creates the300s epoch afterward and observes time0.
- During actual Pyroblast, native-ready Fire Blast/Combustion require literal while-casting permission; permitted native instants preserve the real Pyroblast CastState.

## Actual native RED and implementation

Zero-warning server-simulation RED build at85a3a1c20 completes22m56s/23m03 process; binarySHA5906d6361b01e20a3d5cd6d00c9aed91a90efda0297b20d329a09bfb0c29ff14. Exact native cases pass all real Player63/spell/GCD/cast/readiness premises, then fail: cursor27.45s returns Finished during actual Arcane Intellect GCD; options23.73s selects Fire Blast during real Pyroblast without use_while_casting. Engine `ww-parity-2026-10-23/native-precombat-poll-red-85a3a1c20/{cursor-exact,while-casting-exact}.{proof.json,output.log}` retains exact commands, binary/current-source/copy/input hashes. Initial combined tool result became unavailable with no process/log/result; it is explicitly unknown, not counted as RED.

Prepared now samples actual CastState alongside existing native GCD. `poll_precombat` returns Waiting before touching Program/cursor/variables, including after the final harmful cast until native casting/GCD finish. Direct busy precombat `select` returns PRECOMBAT_WAIT_REQUIRED instead of silently consuming rows. Ready polling delegates unchanged one-pass source semantics; Action/Execution/options stay intact. Combat row permission additionally requires use_while_casting1 when a native cast is active, still subject to normal native cast admission. No wait behavior was implemented before these real failures; both native controls now pass at3d32d68bf.

## Current GREEN scope —3d32d68bf

Zero-warning server-simulation build28.76s/29.22s process; binarySHAe82cbc53935536e72f0618acd3f64a22c64d791f5fe9ba57c7e516e48e09e4bb. Actual Player63 two-case packet passes27.11s on a fresh current215d75ec.../cd7230ac... backup; full hashes/commands/input receipts at engine `ww-parity-2026-10-23/native-precombat-poll-green-3d32d68bf/`.

Native cast order1459→55342→11366, Pyroblast CastState duration2.3369999s, setup elapsed5.35s; Finished still has actual native flight1/remains0.142s. Epoch created afterward reads time0. Explicit Fire Blast/Combustion while-casting/off-GCD rows preserve the ongoing Pyroblast cast; missing while permission rejects the row. No synthetic precast/travel, timer, resource grant or cast interruption.

Relevant same-binary regressions pass23/23: native precombat9/51.15s, selection7/33.02s, lifecycle1/16.29s, manual poison bars1/14.85s and row gates5/23.15s. The old Mystic4 live in server core, were not rerun in this executable and are not counted in the current25; their prior proof remains separate. Full88-row FireFacts/source/adapter, item inventory/provider execution and all-class options remain14779/other-owner qualifications, not claims from this shared packet.

Caller pattern: sample Prepared/facts → poll; Waiting advances actual App frames, Selected releases facts then dispatches native execution, Finished releases setup and starts the caller's300s epoch. Query782's native Pyroblast flight at that point; Finished does not invent a no-flight condition. No fabricated stats/resources/travel, canonical writes, explicit engagement or Fire coefficients are introduced. Full source integration remains14779's proof, alongside these shared boundary controls.
