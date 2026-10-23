# Healer baseline publication causality

Status: kit-table drift is proven; the small remaining Holy difference is still open. Do not substitute a published snapshot for a measured control.

## Controlled input facts

The Holy experiment's actual master668844e83 baseline used immutable SQLite6b8b5e96b2a1f0caac05df74abce559516cacae9279d85e669d97edfd0dd7f2c. The publisher used the canonical dataset. Full source profiles and all available105/257 kit rows through level32 match. Of61 compared native spell/class/trait/profile tables,60 match completely; only `follower_spec_kit` differs. Its count increases1086→1090, but many existing tank/DPS rows also change—not merely four added rows.

Observed party differences include Protection's53600 attacked/missing-aura conditions, Fire's Pyroblast/Arcane Explosion/Meteor conditions, Elemental's Voltaic refresh and BM's Barbed target rule. These are authored decision data, not DB2 amount changes. Do not infer their individual consequences without an ablation.

Evidence root: `game-engine/data/diagnostics/healer-ai-2026-10-21/native-kits/`.

## Native recorded-pressure ablations

Code668844e83, unchanged1,132-hit intake, same protocol/config and every other old SQLite table held fixed. Private copies only; no canonical writes. Both ablations pass the existing persisted-intake driver. The second uses the exact already-compiled test executable in a fresh process. Input/DB/executable hashes and exact authentic row are in `baseline-causality-inputs.json`.

| Dataset change |105 overheal / idle / eHPS |257 overheal / idle / eHPS |
|---|---|---|
| Original frozen table, measured control |64.422641% /312 /91.136549 |78.143645% /546 /95.307031 |
| Only authentic canonical66/53600 row |64.422641% /312 /91.136549 |78.859801% /545.8 /93.181961 |
| Entire authentic canonical kit table |63.305949% /258.6 /98.476447 |78.424483% /545.733333 /105.764285 |
| Published snapshot |63.305949% /258.6 /98.476447 |78.731252% /549.733333 /105.392962 |

All ablations have zero party deaths. Every105 metric field with the current kit table exactly reproduces the published snapshot, including ending mana4360.44873046875; original frozen control ends4430. Holy current-kit ending mana4239.064453125 also matches its published value.

The single-SotR-row hypothesis does not explain the bulk drift and is rejected as the sole cause. Replacing only the full authored kit table causally restores the105 snapshot and most of257's difference. This is not evidence that healer-native coefficients drifted.

## Remaining Holy boundary

Current-kit257 effective healing168620 versus published168028; idle818.6 versus824.6seconds (four GCD equivalents); raw781534 versus790023. Source epochs also differ: published source dce76186c versus measured668844e83 includes runtime/pet and Ignite changes. Published qualification ran14 selected tests in one process; the measured control ran one explicit replay. Code/process-order effects remain hypotheses until an exact-source/input comparison isolates them.

`causality-sotr-only.log` and `causality-current-kits.log`, plus their generated JSON/Markdown directories, retain actual observations. `baseline-native-table-hashes.json` retains the61-table comparison. Native roots alias checkout; no independent-source-copy claim. Do not close the remaining baseline-causality goal or claim whole-production equivalence from this partial result.
