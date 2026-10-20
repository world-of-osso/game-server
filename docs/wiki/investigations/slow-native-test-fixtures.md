# Slow native test fixtures

## Scope and measurements (2026-10-16)

The lead's master gate reported eleven native gameplay tests above60s. Three isolated CPU profiles on server379c2c6b/protocoldc30250 retained every assertion:

| Test | Test-body duration | Runner duration including perf |
|---|---:|---:|
| All sixteen authored arena maps |469.15s|475.06s|
| Absent-player-aura marks/kills/reopen |55.11s|60.26s|
| Orb of Deception race/sex matrix |144.40s|148.25s|

Evidence: `game-engine/data/diagnostics/server-slow-tests-2026-10-16/` contains exact argv/revision/admission JSON, logs and49Hz user CPU perf recordings. Host contention varies; these are test-harness diagnostics, not release budget measurements. DWARF call-chain unwinding was incomplete; percentages are self samples, not complete loader attribution.

## Shared cause

Each arena builds a fresh app but reparses the same creature/achievement inputs; Solo Shuffle also reloads full class/spell data. The aura test loads the full achievement catalog five times. Orb reloads the complete spell catalog for every one of its twenty mapped race/sex cases and the unmapped refusal. Profiles show SQL VM/decoding and catalog construction (`criteria_conditions`, spell `find_mut`) rather than sleeping. Boss fixtures repeatedly load all creature templates before selecting their encounter subset. Pet fixtures repeatedly snapshot/import the same class summon source and reload its creatures/spells on relog.

## Change and authority boundaries

Test-only process caches parse canonical creature and achievement inputs once. The existing native spell fixture supplies complete spell/class copies; new clone implementations exist only under `cfg(test)`. Each app still owns mutable resources, entities and redb files. Achievement progress, dates, inventory, characters and protocol replies remain independent.

Class summon preparation imports one process-owned snapshot under `target/class-fixtures/` before sharing it read-only. The800MB process-lived file stays off `/tmp` tmpfs and is disposable with the slot's target cache. Its creature product is cloned before subset selection. Prepared encounter copies retain their own loaders: the canonical cache must never replace a copy's newly imported authority. Orb's producer changes creature/item/map tables, not spells; its spell input remains the unchanged canonical catalog.

The affected harnesses already use manual50ms Bevy time. No wall-clock wait or gameplay duration was shortened; all matches, casts, kills, race/sex combinations, cooldowns and durable-reopen assertions remain. No development profile or production loader changed.

## Verification

Atb2df8d8e atop master2dadf94e, all eleven named tests pass separately with unchanged gameplay assertions:

| Case | GREEN test-body duration |
|---|---:|
| Sixteen arena maps |124.81s|
| Absent-player-aura |22.65s|
| Orb race/sex matrix |54.77s|
| Six-round Solo Shuffle |56.28s|
| Gilneas Full Coverage |26.91s|
| Follower Felguard tap |27.74s|
| Warlock summon/dismiss/reopen |51.94s|
| Dungeon difficulty |29.01s|
| Legacy dungeon difficulty |44.15s|
| Kresh/Serpentis lifecycle |29.58s|
| Malygos full roster |57.32s|

The first three use matching49Hz CPU profiling on both sides. This is not an eleven-test combined invocation or proof that every test is below60s; arena remains above that threshold. The baseline used379c2c6b; current green also includes the intervening master2dadf94e source changes. Logs and `green-duration-ledger.json` preserve exact revisions, commands, admission snapshots and test-versus-runner durations. Full-suite timing/host contention remain lead-owned; no62→17minute claim.

Two isolation tests pass36.82s, proving app creature/catalog/class changes do not alter cached authority and the imported class snapshot rejects SQL writes. Follow-up45ff1598 changes only the snapshot's scratch location to disk-backed target. All5 prepared-source/isolation controls pass125.49s, including actual follower tap, six demon summon/dismiss/reopens and a separately imported Golden Serpent encounter; feature check passes2m43s without warnings. Unrelated canonical caches/profiles were not rerun for that filesystem-only change.

After merging master6403a988 (combat/aggro/script changes) atedd15fd6, the complete bounded set passes together: **14/14 in410.83s** (eleven named tests plus two isolation controls and the imported encounter-copy control). Compile/package-lock time is separately recorded3m13s; current feature check passes11.37s without warnings. This merge intersects gameplay consumers, so the bounded rerun is new integration proof, not a ceremonial repeat. Formatting/diff and manual changed-Rust readability checks pass. Full-server suite timing and independent verification remain lead-owned.

## Promoted battleground world reuse

`maps::tests::battleground_nodes::promoted_source_world` retains the fully decoded `WorldData` only after its copy-only promotion completes, keyed by the exact map ID. Alterac30 and Isle628 have separate entries; every fresh app receives its own clone. The temporary database is deleted after loading. No app, redb, entity, navigation query or canonical cache is shared, and the producer pipeline and all roster/frame/clock/assertion boundaries remain unchanged. A two-live-app test mutates both profiles' names, populations and expected-stat maps, then reloads each original profile to check isolation.

Matching serial `maps::tests::battleground_nodes::alterac_channel_` runs compare base17847d172 with code def21aaad on frozen protocol5660814. Both use Cargo's package working directory (`crates/server`) for the existing relative importer path and the same canonical ground root. All nine test names/statuses match:9 pass, no failures/ignores, with separate start/finish observations.

| Metric | Before | After |
|---|---:|---:|
| Process wall |292.732s|35.023s|
| User+system CPU |285.464s|27.368s|
| Peak RSS |616380KiB|713468KiB|

The cohort saves257.709s wall with +97088KiB peak RSS. First profile use still loads/imports/decodes; subsequent cases retain every native frame and assertion. These serial observations do not guarantee a parallel gate's reduction. One decoded world remains resident per requested profile; this Alterac-only peak does not bound a process also retaining Isle628. The separate two-profile isolation characterization passes before75.01s and after27.05s; it is not a full Isle gameplay-suite result.

Evidence: `/home/osso/.worktrees/logs/wow_formats-gate-timing/alterac-{before,after}/`, aggregate `wow_formats-alterac-comparison.json` and `wow_formats-alterac-isolation-{before,after}.log`. No repeated benchmark was used to replace an outcome.

### Sources

- [Fixture loader and isolation test](../../../crates/server/src/maps/tests_battleground_nodes.rs), [copy-only promotion producer](../../../scripts/promote_battlegrounds.py).

## Arena residual: bounded independent fixtures (2026-10-16)

The sixteen-map test runs a fresh App and redb for every map. LEAD confirmed no cross-map serial-order contract and authorized at most four workers. Scoped workers now execute all sixteen source cases, join every worker and propagate every fixture panic; source census, gate/orb/combat/return/reopen assertions, frame counts and50ms steps are unchanged. No production catalog experiment or scheduler override is included in this follow-up.

Onbccfa428 (requestedbda55ef5+c86bb318, merged currentmaster4b4e017f), ordinary no-feature native test passes **52.70s**, with sixteen distinct source-map PASS lines. Sampled `/proc/<pid>/stat` CPU is105.41s user+8.29s system over52.90s runner wall time (20ms samples; final CPU counters may undercount the exit tail). This uses normal capacity admission, not a quiet window. `/usr/bin/time` was unavailable; the first timer-wrapper attempt exited127 before starting the test and is preserved as setup failure, not runtime RED.

The serial instrumented diagnostic at400adbd3 was117.06s; flags, master revision and host load differ, so it is not a controlled speedup ratio. The earlier executor/query/catalog experiments remain separately parked and are not merged into this fixture-only candidate. Evidence: `arena-four-workers-{build,native-green}.log/.json` and `arena-four-workers-cpu-samples.json`. Full-suite integration and timing under other contention remain lead-owned.

## Selected native follower metadata (2026-10-19)

`spell_cast::test_harness::load_native_creature_world` previously built every template's level arrays before retaining requested entries. It now clones only requested templates from the existing `test_data::world_data()` view, retaining their immutable `Arc<LevelStats>` and cloning `ExpectedStats`. All other returned world fields still use their original defaults. `followers::profile_behavior_tests` clones complete classes from the existing spell fixture instead of reloading them. No new canonical cache, app sharing or navigation sharing is introduced; prepared encounter inputs still require their own authority.

Matching serial cohorts compare base7cd9667a8 with code0fc15ec61, both on protocol03580cc. Canonical protocol moved during implementation, so the after build uses an authorized detached03580cc checkout, not the newer canonical source. One app-isolation characterization passes before and after: tuned Kobold40 plus Garrick209057/Crenna209072 remain selected, while edits to one live app's name/models/tuning/template set/ExpectedStats leave its sibling and canonical input unchanged.

| Cohort | Before/after test wall | Before/after process CPU | Before/after peak RSS KiB | Preserved coverage |
|---|---:|---:|---:|---|
| Baked corners |84.169/70.698s|83.641/69.518s|1128656/1118916|1353 corners, zero qualified, no path errors|
| Authored origins, two tests |379.821/37.465s|339.822/37.195s|1135952/1087204|144 origins, same four qualified, no violations/errors|
| `followers::kit_*` |515.198/68.450s|475.463/66.884s|1022284/1146772|36 pass, three unchanged pre-existing ignores|

CPU/RSS cover the entire child process, including captured diagnostics; corner process wall additionally contains output emission (109.001/101.030s), excluded from its test wall. Origin process wall is380.274/37.565s; kit process wall515.352/68.536s. Host contention and cold canonical initialization vary: these observations are not a guaranteed parallel gate reduction. Kits retain more canonical input in memory (peak +124488KiB); the fixture CPU reduction is not free memory reuse. Full raw output, serial start/finish events, `/proc` samples and resource receipts are under `/home/osso/.worktrees/logs/wow_formats-gate-timing/phase-a-{before,after}-{corners,origins,kits}/`. No assertions,50ms ticks, difficulties, native routes or events changed.

## Fully prepared encounter inputs (2026-10-19)

`scripted_ai::tests::data_bosses::prepared_inputs` retains only decoded immutable resources after a complete copy-only producer finishes. Its process-local key includes producer identity, selected entry/map and population mode; difficulty-specific spawning still runs separately in each app against the complete difficulty-template catalog. The authored single-actor row is decoded from the same prepared copy, cloned and assigned the requesting difficulty before normal spawning. Every installation clones resources; existing immutable `Arc` level/path/script data remains read-only or uses existing copy-on-write mutation. No Bevy app, world, entity, SQLite connection, temporary database or navigation query is cached. Canonical fixture inputs are not added to this cache.

The type-erased resource-copy list keeps the existing private resource types private: construction records each original loader explicitly, and installation only clones its captured resource into the app. Metadata types gain test-only `Clone`, including initially empty mutable area memberships; mutable app edits cannot reach retained inputs. Herald INTRO2 has a distinct completed composite key: base Herald preparation, Nazan summon preparation and server-side spell import all finish before world/spell resources are retained. `prepare_encounter_database` itself remains a fresh-copy API for callers that need to mutate their own database. No existing assertions, event order, ticks, difficulty variants or censuses are removed.

### Phase B matched observations

Base09cbaf988 and implementationf504ae535 use the same frozen protocol0399f8b. All six serial cohorts retain identical test/status lists:28 pass plus the original authenticated-bake Ragefire ignore on each side. A separate new test keeps Normal Herald, Heroic Omor and Heroic Herald apps alive together, mutates the Normal app's world/spell resources and reloads Normal inputs; sibling and cached authority remain unchanged. This characterization passes both before125.17s and after190.43s; those characterization invocations are excluded from the matched timing cohorts.

| Cohort | Before/after process wall | Before/after process CPU | Before/after peak RSS KiB |
|---|---:|---:|---:|
| Omor slay6 |171.555/44.840s|143.406/36.703s|1065500/1419560|
| Ragefire completion3 |85.536/143.534s|70.505/58.755s|1052560/1316864|
| Azure Rune8 |250.486/74.411s|197.079/53.306s|1007612/1384572|
| Telash return4 |186.337/64.391s|115.250/52.713s|1043944/1369008|
| Herald intro/damage3 |159.393/53.812s|136.679/46.852s|1385400/1806788|
| Herald min1 four cases |212.425/65.007s|164.643/60.311s|1404984/1799132|

Ragefire isolated cold wall regressed despite lower CPU: the cohort uses each prepared profile for the first time, so no repeated-profile savings are claimed there. Samples/receipts retain the measured I/O and contention rather than replacing that result with a retry. Every cohort's peak RSS increases because decoded resources live until process exit; these isolated peaks are not a bound for a gate retaining every profile. First-load imports, per-app clones and native simulation still run. Serial wall savings are not guaranteed parallel-gate savings.

Evidence: `/home/osso/.worktrees/logs/wow_formats-gate-timing/phase-b-{before,after}-{omor,ragefire,azure-rune,azure-return,herald-intro,herald-min1}/`; aggregate `wow_formats-phase-b-measured-comparison.json`. Per-test start/finish flushes are separately observed for every enabled case. Package check passes90s with no warnings; format/diff checks pass. Broader merged-master integration remains lead-owned.

## Canonical toy spell reuse

The transmitted-object fixtures and shared facing fixture read the unchanged canonical spell database. They now clone the complete store already loaded by `spell_cast::test_harness::world_db()`, instead of rebuilding it for each app. Prepared-copy spell loaders are unchanged. Every app still owns its store, redb, cases and ordinary50ms lifecycle observations; this adds no cache and removes no tick or assertion.

Matching full `toys:: --test-threads=1 --format=pretty` runs compare base337355397 with code c9feb45a2 on frozen protocol da007a6. Both pass the exact same213 cases with no failures or ignores; every test has separate start/finish output events. Full-family wall **regressed2246.693→2677.801s**, process CPU2046.590→2346.632s and peak RSS1655476→1756852KiB. No full-family or gate-wall improvement is claimed.

The five directly affected tests improve244.738→113.634s combined (131.103s): transmitted-object lifecycles113.962→64.136s; four facing tests130.776→49.498s. The208 unaffected tests grow2001.814→2563.669s, overwhelming that local saving. Lead accepted that unaffected growth as host contention, not the two-site change; these receipts do not quantify its separate phases. Results were not rerun to select a better outcome. Evidence: `/home/osso/.worktrees/logs/wow_formats-gate-timing/toys-{before,after}/` and aggregate `wow_formats-toys-comparison.json`.

## Related

- [Server tick lag](server-tick-lag.md) — distinct runtime/FIFO work; no release acceptance follows from faster tests.
