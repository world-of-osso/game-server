# BM owner cooldown return proof

## Coverage matrix

| Boundary | Proof |
|---|---|
| Actual Austin BW restores selected active193532 native count1 after real Barbed spend | RED5b/e164: full2→spent1, after1 vs2 expected, elapsed1.25s, no intervening Barbed; GREEN9a03: after2 |
| Actual Scent removal suppresses return | PASS before/after; after1, same actual successful BW/clock |
| FULL_MASK free Cobra +active469880 reduces pending Barbed by native2000ms at cast | Genuine RED5b/e164:14.609 remains14.609 versus12.609 expected, before next frame; real free Focus unchanged; shared-launch implementation pendingGREEN |
| Actual Scales removal suppresses free-Cobra reduction | PASS; same native free launch, pending recovery unchanged |
| Normal Cobra Scales and absence control | Earlier NPC geometry failures are not gameplay RED. Actual controlled native Player18505: pending13.359 unchanged vs11.359 expected; inactive controlPASS, geometry(0,0,0) unchanged |

Scent9a03bcd845f219da5454e12fe4e89981858a4c9c:2GREEN19.78s; one `server-simulation --lib` build12m51. Actual native selected amount1, not assumed older2. Existing RestoreCharge owns cap and partial recovery; independent pet strike retained after the owner return. [Contract](../specs/spell-dummy-scripts.md#native-bm-owner-cooldown-returns).

## Exact artifacts

Diagnostics root: `game-engine/data/diagnostics/dps-parity-fix-2026-10-21/`.

- `owner-cooldown-red-5b7cb789f-{compile.log,native.log,executable.json}`: six ran42.45s, Scent/freeScales genuine RED and absence controlsPASS, normal Scales range setup failures.
- `owner-cooldown-geometry-probe-140fd1a00-native.log`: measured preparedactor(22.03,-7.57,-0.27)→(66.72,-22.94,-0.82), target(1.5,0,0), actual maxRange40.
- `owner-cooldown-red-e16492ffa-native.log`: removing FollowOwner/Idle once still fails exact geometry guard; native regroup reinstalls that controller each tick without owner-selected target.
- `owner-cooldown-scent-green-9a03bcd84-{compile.log,native.log,executable.json}`: Scent2GREEN; ELF `server_simulation-c8dd41cff95cf6e8`, SHA256 `1a5691708dcaae9b34ce43935277657e2e9bb1e47a3d0dc276686aafd4c71012`.
- `owner-cooldown-inputs/manifest.json`: fresh canonical read-only snapshot/normal class closure0 additions; world SHA256 `cd7230ac1d62bf7d7ea8fef5fbb82faa9f36b61ec78108e591f1204c614a8ed6`.

Native runs use `agent-run dps-parity-fix`, one process/thread and the existing frozen startup-source wrapper; `proof-ledger.jsonl` records exact argv, revision, environment and digests. Initial d648 private cooldown-reader compile failure is not RED;5b uses public NativeFacts, with core formulas kept private. The single-test geometry probe’s output was valid geometry evidence, but its runner incorrectly demanded plural `1 tests`; that diagnostic-only count guard is corrected.

## Remaining scope

The one-time FollowOwner removal hypothesis was falsified; actual native NPC regroup reinstalls it each tick. Tests18505c7e0 instead construct a genuine controlled native Dwarf Player with pinned253 gear/traits/KnownSpells for single-Cobra mechanics, retaining actual Austin+pet/AI for Scent. No NPC-role conversion, controller stripping, teleport or native admission change. Normal/free Scales positives now both reach genuine RED with two absence controlsPASS21.30s. The specific reduction is bound at existing shared `missiles::launch` before flight for both cast kinds, reusing native pending-cooldown handling; GREEN pending. Scent progress did not wait for the earlier blocked fixture.

Additional exact receipt: `owner-cooldown-scales-red-18505c7e0-{native.log,executable.json}`. The original free-cast RED remains valid, but the new four-case receipt replaces normal fixture failures as the Scales qualification boundary.

Native descriptions and SimC76f8118 hunter6741–6742/4963–4964 source the returns. Selected193532/469880 rank1 have no trait effect-point overrides. KillerCobra199532 is not selected; no unearned reset. PackMentality depends on real Beast lifecycle; no fake summon callback. No stats, coefficients, rank points, timers or rotation have been tuned. Broad parent gate and whole-DPS parity remain open.
