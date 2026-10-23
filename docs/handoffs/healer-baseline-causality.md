# Healer baseline publication causality

Status: full-kit data drift causally explains105; identical fresh-process Holy replays are nondeterministic. Holy's cause remains open. Never substitute published snapshots for measured controls.

## Controlled input facts

The Holy experiment's measured master668844e83 baseline used immutable SQLite6b8b5e96b2a1f0caac05df74abce559516cacae9279d85e669d97edfd0dd7f2c. The publisher used the canonical dataset. Full source profiles and available105/257 kit rows through level32 match. Of61 compared native spell/class/trait/profile tables,60 match completely; only `follower_spec_kit` differs. Count increases1086→1090, with many changed existing tank/DPS rows—not merely four additions.

Party differences include Protection's53600 attacked/missing-aura conditions, Fire's Pyroblast/Arcane Explosion/Meteor conditions, Elemental's Voltaic refresh and BM's Barbed target rule. These are authored decisions, not DB2 amounts. Individual consequences require ablations.

Evidence root: `game-engine/data/diagnostics/healer-ai-2026-10-21/native-kits/`.

## Native recorded-pressure ablations

Code668844e83, unchanged1,132-hit intake, same protocol/config and every other old SQLite table held fixed. Private copies only; no canonical writes. Both ablations pass the existing persisted-intake driver. The second uses the same compiled executable in a fresh process. Exact input/DB/executable hashes and authentic row are in `baseline-causality-inputs.json`.

| Dataset change |105 overheal / idle / eHPS |257 overheal / idle / eHPS |
|---|---|---|
| Original frozen table, measured control |64.422641% /312 /91.136549 |78.143645% /546 /95.307031 |
| Only authentic canonical66/53600 row |64.422641% /312 /91.136549 |78.859801% /545.8 /93.181961 |
| Entire authentic canonical kit table |63.305949% /258.6 /98.476447 |78.424483% /545.733333 /105.764285 |
| Published snapshot |63.305949% /258.6 /98.476447 |78.731252% /549.733333 /105.392962 |

All runs have zero deaths. Every105 metric with current KIT exactly reproduces publication, including ending mana4360.44873046875; frozen control ends4430. Holy current-KIT ending mana4239.064453125 also matches publication.

The single-SotR-row hypothesis is rejected as the bulk105 cause. Full-KIT replacement causally restores105 exactly. Holy's observed changes are not isolated causal effect sizes: the subsequent same-binary repeat demonstrates uncontrolled process variation. Neither native coefficient drift nor complete Holy reconciliation is established.

## Exact published code and process boundary

Source `dce76186cb4d870e7d51fb1bbbf5afb66ce67440`, private current-KIT DB SHA256 `e884f7873446ab23b9723d2b725fbadd7c35d0669b093ecdf5a1e3d8e40d7473`, same compiled executable SHA256 `d9feeac079e5fc696080c2d0514eb2e3318a8f200031ad08c5550717501e4baa`, identical intake/config. The published14-test mode runs in another fresh process. Then the single replay runs again in a fresh process, with revision/binary guards and unchanged input. See `published-code-process-inputs.json` and the three report directories/logs.

| Fresh process | Proof |257 overheal / idle / eHPS |
|---|---|---|
| Single replay |1PASS,130.74s |78.158991% /544.5 /101.061281 |
| Published14-test selection |14PASS,148.31s |78.181888% /549.733333 /103.456689 |
| Single replay repeat |1PASS,153.89s |77.763709% /547.733333 /105.963746 |

All three retain zero deaths,978 native damage receipts,1,132 replayed unit-scale hits,2014.4 native combat seconds,1594.3 frozen denominator seconds and4239.064453125 ending mana. Single-process Holy effective healing changes161122→168938 despite the same executable/input.105 remains identical across every reported metric, including98.476447eHPS/258.6idle. Thus code epoch or14-versus1 ordering alone cannot explain the residual. This does not establish that either factor has zero effect.

## First-divergence probe, not a proven fix

Approved diagnostic branch `healer-pom-trace`, source5c673448c2a45b57a28697bd688e992cbe77ead7, adds only test logging to the PoM selector and a cloned seeded-RNG fingerprint. It does not consume the actual RNG stream or alter target selection. The two intended fresh-process replays compare actual candidate order/health/priority, RNG fingerprints and selected recipients. Capacity admission is pending; no trace result exists yet.

PoM retains the best injury/player-priority bucket then shuffles it. Its upstream area query sorts by distance without an equal-distance identity tie-breaker. This suggests an enumeration-order hypothesis, not proof that the measured replays actually differ there. Bevy EntityHash is deterministic; unordered MapUnits is not itself evidence, and this area query does not use MapUnits.

Separate source0af61cea864698638ace922f563740f466bb3727 prepares a seeded cyclic-permutation counterexample with authentic native level32 Garrick/Meredy/Austin and155793 metadata. It is committed but NOT RUN. Production canonicalization is absent. Accept the hypothesis only with observed process-boundary evidence; preserve TC priority and uniform selection if fixing it. No benchmark/capture changes, fabricated stats or canonical DB writes.

Native roots alias checkout; no independent-source-copy proof. Published reproducibility and whole-production equivalence remain open.
