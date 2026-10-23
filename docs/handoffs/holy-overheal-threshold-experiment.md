# Holy routine-health threshold experiment — reject

**Do not land the production policy change in this branch.** It improves an overheal percentage by suppressing useful healer activity. Both deaths and raw throughput matter; zero deaths in this one constrained replay do not establish acceptable healing quality.

## Exact experiment

- Current-master baseline:668844e83f6463fa118d1c794011b43953c1bcf3, including landed14737 replay.
- Isolated measured candidate:9131f6ed22efeb636cc5208763f0de93fabbffcf. Its only change from baseline is `followers/holy_policy.rs` copied from68f9f8bf2a9c44d53a907c8795f921dbea6c68ef.
- Implementation branch integration:ff5214227737ab4ca2f6530014d52638d4c11f22. Additional files are native tests and the proposed policy spec, not benchmark/capture changes.
- Identical1,132-hit Stockade intake, BLAKE3`541a4dedf0e1114307fd9972cd21642ac5ad62f6646b7a62f61c677839229eb8`; SHA256`c08b6c91551aa6f3e1697b9837c6ffd1ea66787b857622c9247eb53eda9809c2`. Frozen recorded-combat denominator1,594.3s; native observed combat remains a distinct field.
- Same protocol2abc7fa844f6e71d93203ce5421ae853e884854d, test profile, local config and frozen native SQLite6b8b5e96b2a1f0caac05df74abce559516cacae9279d85e669d97edfd0dd7f2c. Input/config hashes unchanged after measurement; helper native roots alias the checkout, not an independent source copy.

Evidence root: `game-engine/data/diagnostics/healer-ai-2026-10-21/overheal-idle/`. `master-replay-inputs.json` pins code/input/config; `master-{control,holy-candidate}/` retain unmodified generated JSON/Markdown; `master-holy-comparison.json` holds exact paired metrics.

## Actual paired results

| Spec / metric | Baseline | Holy overlay |
|---|---:|---:|
|105 deaths |0|0|
|105 overheal |64.422641%|64.422641%|
|105 idle GCD equivalents |312.000000|312.000000|
|105 effective HPS |91.136549|91.136549|
|257 deaths |0|0|
|257 overheal |78.143645%|26.987854%|
|257 idle GCD equivalents |546.000000|1178.233333|
|257 effective HPS |95.307031|5.655774|
|257 raw / effective healing |695212 /151948|12350 /9017|
|257 ending mana |4430|4430|

All105 metric fields are identical, an unchanged-spec control. Holy effective healing falls94.065733% while ready-frame idle increases632.233333GCD equivalents. The lower overheal ratio is not an overall-quality improvement. Idle counts all living combat-ready frames, not hypothetical missed healing opportunities; no policy should spam useless heals merely to lower it.

The in-tree published snapshot has different metrics (10598.476447eHPS,257105.392962eHPS). It does not reproduce under this frozen local scope. `master-control-v-published.json` preserves exact differences; their cause is not isolated. The table above uses actual current-master replay results on both sides, not substituted published values. No whole-current-production acceptance is claimed.

## Diagnosis and proof levels

| Boundary | Proof |
|---|---|
| Oversized chip-injury heal | Native RED at74924de25:78missingHP produces Serenity2360/raw,2282overheal (96.7%). `holy-chip-native-red.log`,1FAIL16.30s. |
| Proposed hard routine gates | Holy owns2050/2061 and requires authored40%/80% health thresholds outside emergency/Surge choices. Shared ranking, resources, spell amounts and benchmark files unchanged. |
| Bounded native decisions |68f9f8bf2:8PASS39.19s, including chip injury, moderate Flash, emergency Serenity, naturally earned Surge, PoH2, unsafe offense veto and native Purify. No existing expectation changed. |
| Frozen replay | Baseline and candidate each return0; both native specs process all1,132 recorded hits. No second autonomous route/capture. Candidate admission exhausted once without compiling; bounded retry ran only candidate, never repeated baseline. |
| Formatting/readability | Integratedff521 format checkPASS; changed production has explicit ownership, native readiness and authored thresholds, no warning suppressions. |
| Final shipping gate | Not run: the measured behavioral regression already rejects this experiment. Not ready for landing. |

The original large-Serenity counterexample is genuine. The proposed blanket direct-heal health gates are too coarse for the measured workload. Preserve this failed experiment rather than weakening the native oracle or claiming improved efficiency from the lower percentage. A narrower selection/forecast diagnosis needs its own native RED and paired replay before any future policy change.

## Next authorized scope

After this packet, implement minimal native decision kits for65/Garrick,256/Henry,264/Shuja,270/Bo and1468/Kybel using14737's alternate-spec bridge. Coordinate row/GCD-reference format through the lead, source native book amounts from DB2/TC, and establish one genuine native RED per spec. No invented profiles, stats, resources or casts.
