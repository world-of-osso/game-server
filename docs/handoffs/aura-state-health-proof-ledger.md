# Generic health aura states — independent gate

## Authorized scope

Branch `aura-state-health` starts at exact master `461aa81024c2b1d444729fc0ba9961b213d51e7e`. Lead assigned only the seven TC health-threshold states beside existing Druid periodic-heal15. No Ret-local predicate, new spell metadata, other computed states, cached flags or timers. `aura_spell_restrictions.rs` and its Some(false)-only rejection rule remain untouched.

## Pinned source

TC a352b1fa actually sets the health-state siblings in `Unit::Update`, `Unit.cpp481–487`. `ModifyHealth8616–8655` and `SetHealth10039–10067` mutate the backing current health. `Unit.h804–809` uses strict `<`/`>` against uint64 `CountPctFromMaxHealth`; percentage results truncate. `SharedDefines.h2832–2861` supplies IDs/names. Native integer health counts are read from current `Health` each query; no cached aura-state component is introduced.

| ID / TC state | Predicate | Proof status |
|---|---|---|
|2 WOUNDED_20_PERCENT|current < count20%max|genuine core/native RED →core/native GREEN|
|6 WOUNDED_25_PERCENT|current < count25%max|genuine core RED →GREEN|
|13 WOUNDED_35_PERCENT|current < count35%max|genuine core RED →GREEN|
|21 WOUND_HEALTH_20_80|current < count20%max or current > count80%max|genuine core RED →GREEN|
|23 HEALTHY_75_PERCENT|current > count75%max|genuine core RED →GREEN|
|24 WOUND_HEALTH_35_80|current < count35%max or current > count80%max|genuine core RED →GREEN|
|25 WOUNDED_50_PERCENT|current < count50%max|genuine core RED →GREEN|

Tests include equality and neighboring values at every threshold, non-round maximums101/119, all other unknown state IDs and missing Health. Existing unknown-state Swiftmend control previously used state2; intentional new support requires using still-uncomputed state4/Frozen instead. Its contract remains unknown≠false. Existing state15 behaviors remain separate retained controls.

## Strict RED

Revision `fbdc5ee0d889f3db0c0fb78e71acaded10e9d5b4`:

- Core `health_aura_state`:8genuine missing-state failures/2unknown-or-missing-Health controlsPASS;0.00stest,11m43cold compile, zero warnings.
- Native `health_aura_state_native`: real31146 full-health ordinary24275 incorrectly returnsOk instead of TargetAuraState (genuine RED). Real31146 with current health prepared below20% accepts ordinary24275 and receives actual damage (controlPASS).1RED/1PASS32.34s;2m42compile; inherited unused simulator navigation import warning.

The native lower-threshold fixture changes only `Health.current` to19% of the real template's unchanged max; no spell, power, book, range, hit-chance or damage override. It is a health-state admission fixture, not a DPS measurement. Private snapshot SHA256 `52fa4d0ebec755f421b5678f0e83aa690a1d2422f33eae5208ff398848d4756c`; no canonical database writes.

## Implementation and pending proof

`auras/aura_state.rs` computes only these seven IDs, retaining exact15 logic and all otherNone results. Health reads are source-equivalent integer counts; thresholds truncate before strict comparisons. The actual cast pipeline consumes these values without a Ret exception. Existing Swiftmend unknown-state oracle uses4 after2 becomes computed; source15/exemption behavior remains unchanged. Remove the inherited unused simulator navigation alias rather than suppressing its warning.

At7f34b277e82a3b2fa5842fd6c881d8d21015057c, targeted core GREEN is17PASS22.56s:10new state/threshold/equality/integer/unknown cases plus7retained native Swiftmend/state15/owner/other-caster cases; zero warnings,3m12compile. Native original two cases are2PASS41.62s, zero warnings/1m39compile: full-health real31146 rejects24275; the below20threshold fixture accepts and receives actual24275 damage. Nineteen unique cases; no production changes after this revision. Requested `cargo check -p server -p server-simulation --tests -j2` ran exactly once on1a573b3ac33fd700545f371f81848c89f8b3379a:PASS0,2m34s, zero warnings. Two-package `cargo fmt -- --check`:PASS0. Later ledger-only updates do not invalidate this source proof; no source changes after7f34b277e. Parent owns broad/final gate. Native Ret Hammer consumer stays separate until this shared feature lands; no whole-rotation/DPS claim.

## Artifacts

Engine `/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/aura-state-health-2026-10-24/` contains `health-aura-state-source-v1.json` (full TC source paths/hashes), `health-aura-state-core-red-v1.log`, and `health-aura-state-native-red-v1.log`. GREEN artifacts: `health-aura-state-core-green-v1.log`, `health-aura-state-native-green-v1.{log,proof.json}`. Compiler/format artifacts: `health-aura-state-check-v1.{log,proof.json}`, `health-aura-state-fmt-v1.log`. `health-aura-state-native-consumers-v1.json` inventories85native target/exclude-target consumers, including Execute/KillShot; this is source coverage, not85runtime qualifications. Logs/proofs retain exact revision and scope; later source changes invalidate only intersecting evidence.

## ah1093 integration correction — 2026-10-24

Worktree `/home/osso/.worktrees/game-server-ah1093-fix` merges branch7782f0ed6 with exact master1466a4ca7 at62889bccee1340fb457aba53566dfcdcf6534103. Merge conflicts only retain both test-module/wiki additions.

At62889bcce, one `cargo test --locked -p server --lib -j2 class_completeness_native80_shadowfrost -- --nocapture` invocation reproduces both Shadowfrost failures (0PASS/2FAIL,7.11s; cold build47m15s). Both return `SPELL_FAILED_TARGET_AURASTATE` before rune spending or immunity resolution. Artifact: `/home/osso/.worktrees/logs/ah1093fix-shadowfrost-red.log`. Capacity ADD preceded the invocation; private target only.

Root cause is fixture admission, not incorrect health-state computation: native343294 row23284 requires TargetAuraState13; harness NPC starts5000/5000. TC a352b1fa `Unit.cpp483` computes WOUNDED_35_PERCENT, `Unit.h804/809` requires current below truncated35%maximum, and `SpellInfo.cpp2499–2500` refuses its false required target state. `SharedDefines.h2848` assigns ID13. TC exemption checks (`SpellInfo.cpp2495–2497`, `Unit.cpp6163–6185`) do not apply: fixture target is neither owner nor vehicle, and active native immunity/dodge/class passives provide no aura-state bypass. TC source root is recorded above.

Lead explicitly authorized correcting only these two fixture setups after this contradiction was reported. Set maximum health20000 while preserving current5000, making current below7000 and keeping every existing assertion unchanged (especially combined-immunity final5000). Added TC citation at each setup. No health-state, immunity, rune, or production spell changes.

### Current proof scope

All corrected-source proof below covers33d0d484dc650ac65dd80e22698aa0c1811524bb. Later ledger-only commits do not invalidate it. Each Cargo invocation followed capacity admission and used `agent-run ah1093fix`, private checkout target and `-j2` for builds/tests/check. Runtime tests use the supplied gate environment and `TMPDIR=/var/tmp/osso` after agent-run. Formatting/check commands were not rerun.

| Scope | Command / revision | Result | Artifact under `/home/osso/.worktrees/logs/` |
|---|---|---|---|
| Talondras isolated, before fixture correction | Original server binary built at62889bcce; exact `scripted_ai::tests::data_bosses::talondras::encounter_coverage_talondras_native_stomp_damage_qualifies` | 1PASS/0FAIL,16.95s; both difficulty1/205 inside test | `ah1093fix-talondras-repro.log` |
| Both Shadowfrost cases, ten health-state core cases, Talondras | `cargo test --locked -p server --lib -j2 -- class_completeness_native80_shadowfrost health_aura_state encounter_coverage_talondras_native_stomp_damage_qualifies --nocapture` | 13PASS/0FAIL,17.30s; compile2m30s, zero warnings | `ah1093fix-server-green.log` |
| Two native health-state cases | `cargo test --locked -p server-simulation --lib -j2 health_aura_state_native -- --ignored --nocapture --test-threads=1`, with the native runner configuration described below | 2PASS/0FAIL/0ignored,30.96s; compile1.26s, zero warnings | `ah1093fix-native-prepared.{log,json}` |
| Requested three-package test-target check | `cargo check --locked -p server -p server-simulation -p server-spell-tests --tests -j2` | PASS0,6m33s, zero warnings; exactly one invocation | `ah1093fix-check.{log,json}` |
| Three-package formatting | `cargo fmt -p server -p server-simulation -p server-spell-tests -- --check` | PASS0 | `ah1093fix-fmt.{log,json}` |

Talondras did not reproduce in isolation and also passed in the corrected-source13-case run. Its prior gate failure was missing damage at difficulty205, not cast rejection. Native372701 has no aura-state restriction row and no triggered spell effects. This does not establish the original cause; nondeterministic avoidance is only a hypothesis. No Talondras test or production changes were made. The four baseline follower failures and a new full-suite gate are outside this bounded proof.

### Native fixture and output recovery

The native cases are opt-in ignored tests. Their prepared inputs are `DPS_PARITY_OUTPUT=/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/ret-parity-2026-10-23/hammer-policy-v1` and `DPS_PARITY_WORLD_DB` pointing to its `world.ret-policy.db`. Recomputed SHA256 matches the branch receipt: `52fa4d0ebec755f421b5678f0e83aa690a1d2422f33eae5208ff398848d4756c`. Cargo uses `--config 'target.x86_64-unknown-linux-gnu.runner=["python3","/home/osso/.worktrees/game-server-ah1093-fix/scripts/dps_parity_native.py"]'`; that existing runner mounts the prepared database read-only over startup world.db inside bubblewrap. No canonical database writes or fixture metadata changes.

Two foreground native build attempts lost their Pyrun runners during external Continue/resume; their Cargo children exited without recoverable command output. Journal/slice and runtime artifacts were inspected, and lead explicitly approved lost-output retries. A durable child-owned stdout/stderr wrapper (`ah1093fix-proof-run.py`, outside Git) now writes logs and completion JSON independently of the foreground runner. A warm-up invocation selected2ignored tests and executed0; adding `--ignored` first exposed missing prepared-input environment before either health assertion. These are not GREEN evidence. The final prepared invocation above is the valid2-case proof. No check was launched by the obsolete waiting wrapper; it was stopped before the single durable check. Manual readability inspection of the four inserted fixture lines found no issues.
