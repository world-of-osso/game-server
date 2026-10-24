# Feral source census (in progress)

Verified2026-10-23. Follower Ralia Dreamchaser: entry232800/spec103/class11/form768. Native DB2 and public TrinityCore a352b1fa precede SimC76f8118. No current parity claim.

## Frozen evidence

Private SQLite backup of engine `data/server-world/world.db`: SHA256 `51455391e7d4962589cc66d239bf77b2dbf3f4372ec8daee80c092f1e7989d9b`. Canonical database is never written. Evidence directory: engine `data/diagnostics/feral-parity-2026-10-23/`; `db2-spell-census.json` retains full35spell effects/auras/powers/PPM rows, `oracle-factor-census.json` retains printed factors and attribution, `tdb-druid-script-bindings.json` and `tdb-feral-proc-rows.json` retain matching SQL rows. This census is not proof of actual execution.

Authoritative references in engine `data/diagnostics/dps-parity-2026-10-21/`: `103.reference.single.*` ST191666.531726 DPS; `103.reference-fixed.three.*` three actual recipients247748.824618 DPS. Their ±10% bands are172499.878553–210833.184899 and222973.942157–272523.707080. Unsuffixed `103.reference.three.*` is explicitly invalid: Patchwerk clears the authored adds event. Both valid references use300s fixed time, same MID2 profile, optimal_raid=0 and seed70448. Both references reproduced exactly in this evidence directory. Read-only supplied-binary300s observation (**UNKNOWN_PRE_9D13_MISLABELED**, not current-master proof): ST56378.653333 (−70.5850%),3T62475.256667 (−74.7828%); each sustained-clock test PASS. `source-view.proof.json` records the passing startup-loader inode proof; `baseline-master.{1,3}.proof.json` records exact binary/hash/environment and bubblewrap mount view.

## Channel/source coverage

| Reference ST channel | Native data / source | Qualification / gap |
|---|---|---|
| Ferocious Bite22568,15.64% | Native effect0 AP1.452, effect1PowerBurn62/max25; base Energy25 then up to4optional CP beyond required1. TDB binds `spell_dru_ferocious_bite`; TC druid974–1018 scales effect0 by1+min(post-cost Energy,maxExtra)/maxExtra, then burns extra Energy. Incarnation102543 effect1 only adjusts maxExtra if an actual aura effect exists. | No native Bite damage hook found; generic PowerBurn consumes resources but cannot supply this damage factor. Exact ordinary launch→damage→burn genuine RED at a95c0df7a: initial37.5Energy yields147damage, same as25Energy147, rather than1.5×; resource-burn controls pass before the amount assertion. OneFAIL/8.36s/exit101, `feral-bite-red.{output.log,proof.json}`. This new case is temporarily ignored while the independently requested shared action-knowledge slice lands; retire the ignore with the sourced TC Bite port. No fabricated damage scalar. |
| Rip1079,13.94% | Native periodic effect0 AP0.3113, matching oracle printed a_mod. Native `auras/refresh.rs` already handles Rip pandemic. TC druid2080–2114 binds a legacy Player-only amount addition of1%AP×CP plus optional idol amounts. | Source divergence: Ralia retains Follower/NPC identity, not Player; do not port legacy Player-only addition onto follower or replace native coefficient. CP snapshot/duration and real periodic damage still need native proof. |
| Unseen Slash1263890,10.41% | Oracle actual formula1263890/AP0.657; parent Unseen Predator1263660 has real label218 modifiers and proc trigger1263962. | No native1263890 dispatch found. Follow trigger chain/DB2/TDB before any SimC-only dispatch policy; no unconditional fabricated proc. |
| Rake1822→155722,10.19% | Native initial AP0.291 matches oracle; effect2 triggers155722. TDB `spell_dru_rake`, TC druid2040–2077 captures stealth before impact. Existing `scripts/batch7.rs` and `auras/scripts.rs` freeze named bleed stealth bonus; `auras/spell_mods.rs` separately snapshots Tiger's Fury5217 periodic modifier. | Existing mechanism, not a new missing snapshot claim. `dummy12_rake_bleed_snapshots_stealth_bonus_across_later_rank_removal` already covers rank+stealth removal. Fresh matched-profile factors/periodic crit remain unqualified. Concrete source mismatch: `has_rake_stealth_bonus` tests aura231052/405834; native70448 names both ImprovedProwl, and405834 belongs to specs102/104/105. Selected Feral PouncingStrikes390772 is Dummy effect3, not an aura. TC captures stealth without that rank gate. Real selected-profile Prowl→Rake RED is pending; do not infer coverage from a fixture that manually grants the legacy rank aura. |
| Feral Frenzy274837→274838,7.83% | Native parent Dummy5/aura226 with no child trigger; child274838 directAP0.163 and periodicAP0.3267 match oracle. No TC Feral Frenzy script or TDB binding found. | Native parent has no dispatch script. A named SimC scheduling/child policy is needed only after proving no structured producer; DB2 retains coefficients/timing data. |

Other major ST shares: Shred5221 7.14%, cat melee6.56%, Bloodseeker Vines439531 6.54%, Lunar Inspiration155625 5.51%, item Guillotine1306604 4.21%. Three-target top channels: Rip23.89%, Rake14.33%, Lunar Inspiration7.98%, Vines7.71%, Bursting Growth6.60%. Do not absorb missing channels into coefficients.

Mastery77493 already uses generic native Attr8/raw mastery scaling for followers (`auras/mastery.rs`); missing literal rotation does not establish a mastery defect. The35spell census has one TDB proc override,48484/Infected Wounds; newer Feral proc chains need an expanded child census rather than an assumption that absent rows mean zero probability.

## Historical observation decomposition

The lead withdrew the supplied binary's9d13ac19c label: it was an older petgate build of unknown revision. `native-baseline-provenance-correction.json` records the correction and hashes of preserved raw receipts; proof sidecars now carry a null revision and explicit qualification. Own current-build ST/3T replacement is pending. Independent SimC references and static DB2/TC mechanism findings are unaffected.

Despite its historical filename, `baseline-master-per-channel.json` is only that unknown-revision observation. It combines native Rake1822+155722 and reference same-ID Convoke variants without dropping damage. ST native/reference channel damage deltas: Bite−77.2%, Rip−68.5%, Rake−69.3%, Shred+53.7%, Frenzy/Unseen/Vines/Moonfire all−100%. Three-target Rip−89.1%, Rake−98.6%, Swipe+364.6%. Overperforming filler alongside absent channels rules out an undifferentiated global multiplier.

`per-slot-base-stat-diff.json` reconstructs the native source formula using ItemSparse, ItemBonus, explicit item levels, RandPropPoints and the actual generated multiplier tables. All10nonredirect slots match reference gear. Five ignored `redirected_base_stats` options explain base rating deltas(native−reference): head271528→271875 haste−62/vers+62; shoulders271526→268246 crit+5/mastery−5; chest271531→268235 crit+131/haste−69/mastery−62; hands271529→268234 vers+106/crit−48/mastery−58; legs271527→268225 haste+74/crit+69/mastery−143. Total crit+157/haste−57/mastery−268/vers+168. Shared equipment owner has the evidence; no Feral coefficient correction follows.

Native base ratings1076crit/738haste/959mastery/275vers plus7crit/7haste/32mastery gems exactly reconstruct the observed1083/745/991/275. Reference base919/795/1227/107 plus23/23/197/0 gives942/818/1424/107. Remaining consumable gap16crit/16haste/165mastery is routed to the shared consumable owner. Mastery rating mismatch is not a base-mastery defect.

The historical native observation casts Incarnation102543 three times despite its absence from the exact selected Feral reference traits. Own a95c0df7a reproduces actual unselected Incarnation admission; own c3934c31d shared-helper GREEN rejects it while preserving selected visible/parent actions and all990starter books. [Generic action-knowledge census](action-knowledge-census.md) records15/15GREEN, the successful empower control and the separately owned Ret generator-fixture blocker. Current native ST/3T and literal Feral qualification remain pending.

## Explicit TDB provenance boundary

`scripts/import_spell_proc_presence.py --manifest <audit.json> --db <private-world.db>` verifies every pinned base/update SHA-256 before publishing audited `rows[].SpellId` to `tdb_spell_proc(SpellId PRIMARY KEY)`. The supplied `tdb-spell-proc-source-audit.json` has1291effective rows from1283base rows plus7ordered public TC updates. No numeric proc value is imported;14775 owns later value columns. Retained IDs preserve those columns; a value-column insertion constraint rolls the entire refresh back. Use a private DB for qualification; canonical data is not mutated by this task.

The317candidate default-refusal census still splits268explicit/49missing against this effective manifest. WW MomentumBoost451294 is **not** a candidate: native Attributes3=0 lacks TC's required CAN_PROC_FROM_PROCS, despite its unrestricted aura42→451297/0x40010 mask. Do not broaden the guard to force a SimC match. Importer source qualification: genuine CLI RED→5/5GREEN1.31s at87b0d684c, exact1291private IDs, eight hashes verified; `proc-presence-{red,green,private-import}` receipts. Generic refusal behavior remains a separate gate.

Native92d89f8c1 proc run: InfectedWounds48484→58180 passes. Unsafe Savage449645 already produces449646 during real actor bootstrap, before the test's Tiger's Fury cast: genuine refusal RED at that production boundary. Frantic391875 also legitimately produces391876 during bootstrap; its original initial-absence assertion was an incorrect control, not a proc defect. Corrected control requires that actual native bootstrap proc and verifies the child is not directly learned. Full run1PASS/2FAIL16.77s,20m57compile,zero warnings; `proc-definition-red.{proof.json,output.log}`. No damage/flags/probabilities are fabricated.

`SpellAuraOptions::proc_definition_source: Option<ProcDefinitionSource>` now distinguishes Generated/ExplicitTdb/Scripted; None disables proc dispatch without deleting raw options. Complete assembly and ordered effects precede provenance/refusal. Existing scripted mask policies are shared with the classifier, not duplicated as new spell exceptions. `tdb_spell_proc` must exist before loading this candidate; use the provenance CLI on the gate's private data copy before runtime qualification. Canonical data is not changed by this task. Numeric TDB overlays remain14775-owned. At682b31e1c: native3/3GREEN10.34s, standing Dance1PASS6.98s and Inundate3PASS9.50s;21m31compile/1304.13swall/zero compiler warnings. All controls use the same library binary SHA256ea620d7c1709fa96e02685e0a2c58c19acdbfeff856219eed00a56d8dc327e57 and private54e9bb0307e3fa7382e5a9cc30a6449694e970993e47dda47a03fe8113efee90. Receipts `proc-definition-green`, `proc-scripted-dance-control`, `proc-scripted-inundate-control`. Parent published1291presence rows canonically through backup/copy/atomic rename and queued separate gatepv1065; parent owns its result. No Feral parity verdict follows from this narrow mechanism proof.

## Native form combat timing

TC a352b1fa `Player.cpp23407–23413` sets both melee base times from nonzero CombatRoundTime, restores ranged base2s, else uses regular weapons. `StatSystem.cpp453–471` separately clamps AP time to0.25s and scales weapon min/max by formtime/APtime; `Unit.cpp11100–11132` and `9557–9560` select form AP time for non-Players or Feral Player forms (Cat/Bear/DireBear/GhostWolf), otherwise the weapon's ordinary/normalized time. Keep the project's already documented RetailAP/DPS6 instead of changing coefficients to TC3.5. `DamageVariance` occurs only in TC DB2 declarations, not runtime code; no new scalar is justified.

Native70448 nonzero rows: Cat1=1000ms, Bear5/DireBear8=2500ms, Xuen44=1ms, Yulon45/Chiji46/Niuzao49=2000ms. Preserve authored1ms rather than guessing1s. Class-stat follower clocks consume this generic data; ordinary template-only creatures and Spirit Wolf clocks remain unchanged. An in-flight melee timer keeps its phase; the next reset uses current form timing.

Genuine timed-swing RED at564caae3b: Cat actual1.85s versus source/haste0.909s, Bear actual1.85s versus2.272s; unshifted equipment control passes.2FAIL/1PASS10.82s,19m18compile,1170.75swall,zero warnings. Actual delivered melee events, native stats/slow and authored rows are measured without invented weapons or powers. `form-combat-red.{proof.json,output.log}`; prior tuple inference error is separately archived, not counted as RED.

The generic form clock and shared weapon terms are now implemented. A source-backed matrix covers all7rows, Player/non-Player selection, normalized/ordinary melee plus the real native73000rifle, zero-time Ghost Wolf and unshifted controls; GREEN pending. No AP coefficient, native damage bonus, item value or fixture verdict band is tuned.

Pre-crateB a7f6f10fa native KIT300s remeasurement is now independently qualified: ST81105.573333, corrected3T90069.816667. Sustained-clock and ten-wave controls pass36.19s/52.59s;324/322actual melee hits. `current-a7f6f10fa/` preserves profile/trait hashes, private-inode proof and exact binaryfe58515b0ddeda9dc7c6ed65c3406f242bc9bc99a7d659c6f0daa0da58357216/data54e9 receipts. This supersedes the historical unknown-revision observation for current-code evidence, not as a causal comparison or literal-APL parity result.

CrateB integration keeps controller policy in public server knowledge SPI and simulation as its consumer. The native form timing type and weapon-range entry point are public server data/SPI so the simulation dependency direction stays valid. Original test names and semantics are retained; new two-package qualification is separate from the pre-split31case evidence.

Unified7701fc1b3 targeted qualification:33/33PASS, privateDB54e9… unchanged (1291proc IDs/1408trait points), shared-protocol2abc7fa844f6e71d93203ce5421ae853e884854d clean. Evidence root `crate-b-unified/`:

| Package / behavior | Tests | Result / test seconds |
|---|---:|---|
| server: resolved action admission and passive endpoints |16|PASS49.88|
| server: generated refusal, explicit provenance, Frantic retention |3|PASS22.65|
| server: Cat/Bear/unshifted clocks and all-form weapon terms |4|PASS30.98|
| server-simulation: target/resource-preserving literal execution |9|PASS49.02|
| server-simulation: native Feral unselected-talent exclusion |1|PASS7.37|

Core binary SHA1f9a6f5e6ab2fca005eb20623af28f4be56a4a80a455e23ffbc9290804e632b2; simulation SHAab828e815d992172086f560d4ed54ecc281dc7b90cb96c6e325cc59b3b99584f. The original combined command's runner exited without an exit/output receipt after emitting the core test binary; its exit remainsUNKNOWN in `interruption.proof.json`. No heavy core build was repeated. A separately completed simulation-only `--no-run` built both server library and server-simulation with exit0,1193.87s, no warnings/errors; `simulation-build.stderr.log` preserves the full compiler output. Direct core/simulation runs above have individual exit/count/hash receipts. Parent ft1083 pins770; final integration gate remains parent-owned. This qualifies only the named behaviors, not full-suite success or Feral literal/DPS parity.

## Literal boundary

The exact pinned profile requires target_if Boolean selectors and action-relative dot/snapshot facts, active_dot, hero_tree, previous-GCD facts, pool_resource/for_next, max_energy and use_items/off-GCD options. Shared owners receive these requests through the lead; Feral owns identity bindings, Berserk/Incarnation aliases and snapshot source attribution. Dungeon rotation is unchanged; only explicit parity Fixture encounters may capture native follower decisions.

SimC `sc_druid.cpp11216–11323` maps clearcasting to native135700, Apex proc to391882 and Sudden Ambush to1244483 through selected talent triggers; `13595–13617` maps bs_inc to Incarnation when selected, otherwise Berserk. Native TraitDefinition overrides, not guessed action replacement IDs, remain authoritative.

## Sources

- Public pinned SimC `engine/class_modules/sc_druid.cpp`, `profiles/MID2/MID2_Druid_Feral.simc`; profile SHA2569f419caddee5e8cae94856a05b27bd276b390c6ac6c83d4d6449b1eccd093797.
- Public pinned TC `/home/osso/.worktrees/logs/achievements-criteria-2026-10-12/tc-source-a352b1fa/TrinityCore-a352b1fa/src/server/scripts/Spells/spell_druid.cpp`.
- TDB `spell_proc.sql`/`spell_script_names.sql` from engine `data/diagnostics/spell-dummy7-2026-10-10/`.
- [Spell oracle](../wiki/reference/simc-spell-oracle.md), [shared literal machinery](../wiki/systems/simc-apl.md), [DPS parity contract](../specs/dps-parity.md).
