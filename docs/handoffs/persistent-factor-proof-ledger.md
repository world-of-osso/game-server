# Generic periodic persistent-factor facts

## Scope / ownership

Main authorized this slice after14860 confirmed no in-flight snapshot/shared-aura edit (a7f6f10fa changes attack_timing only). Feral eligibility remains14860-owned. Capture existing explicit native product separately from merged amount; stored `dot.<alias>.pmultiplier` and live proposed/action `persistent_multiplier` are separate accessors. No duration/tick-count fact expansion, new eligibility, numeric tuning, canonical/template/reference changes or DPS qualification. FIRST/cycle4334db5e3 is frozen for lead fc1078; composite helper0b1b9b79b is independent.

## Source / representation

Pinned76f8118 `dot.cpp568–573` reads existing dot state factor or0 without state. `action.cpp3465–3493` snapshots proposed action state and recomputes its live factor. Druid `sc_druid.cpp2829–2867` distinguishes periodic/direct persistent products. Native `AuraEffectState` currently stores merged amount only; `spell_mods::snapshot_periodic_amount` already has separate flat/product and source eligibility; scripts compose named Rake/SotF and other non-factor transforms. No amount-ratio inference is permitted.

`DotPersistent { resolve: ResolveDotSpell, effect: u8 }` and `Bindings.dot_persistent` only register exact native periodic effect metadata per existing aura alias. This initial scaffold has no new fact/capture behavior and may produce unused-field warnings until GREEN; warnings must not be suppressed.

## Initial build-only failure (not RED)

9525e06e8 merged master d675bf8fb/crate-split B before building. Ordinary-admitted native attempt produced E0063: an existing full Bindings initializer in simc_apl_native/tests.rs needed the new empty metadata map; no test executed. `persistent-native-red.{log,proof.json}` retained as INVALID, not RED. Correct only that fixture initializer before retry; no production behavior implemented.

Actual fresh SQLite read-only CURRENT canonical backup for this scope: `world.arms-persistent-current-cd7230ac1d62.db`, SHAcd7230ac1d62bf7d7ea8fef5fbb82faa9f36b61ec78108e591f1204c614a8ed6; canonical before/after SHA215d75ec02c0311831cf5d98a4432c985500345811471fcce6ed23fee7709557 unchanged. quick_check ok; private worktree alias points to this snapshot. `persistent-native-snapshot.proof.json`; no canonical write/DPS run.

## Strict native RED result

b5fa54c59: actual7genuineRED39.07s after22m25compile/0warnings. Real native Rake/SotF applications/qualifiers/ownership setup reaches each missing-feature assertion: five stored pmultiplier errors, two live/named-gap errors. `persistent-native-lib-red.{log,proof.json}` and `persistent-native-lib-red-binary.proof.json` pin source, snapshot and exact library SHA64e72516b09e4a0bc7c5ad8477b83a7c5a17e6b73f6b3e9362ff13d060b3d094. This is RED only; later-state/reapplication assertions after the initial missing getter are not GREEN proof.

Seven ignored native cases use real SpellStore rows/class11 and actual `apply_aura` applications, not fabricated AuraEffectState amounts. This scope proves the aura application/capture boundary, not full root Player cast ordering. Existing Rake native cast/child/tick regression is a later retained control.

| Case | Required observable behavior | Runtime proof |
|---|---|---|
| Rake stored application | Existing native rank+stealth bonus; later qualifier removal/AP change preserves stored product and amount, while live query changes | Stored fact missing: genuine RED |
| Rake reapplication | Real updated AP changes new amount; newly applied product reflects current qualifier absence | Stored fact missing: genuine RED |
| Live versus absent stored | Live proposed Rake bonus before any dot exists; stored0, explicit action alias equivalent, context-free query errors | Live fact missing: genuine RED |
| Actual ownership/target | Distinct own primary/secondary snapshots, foreign primary and foreign-only target cannot override own selected value | Stored fact missing: genuine RED |
| Unrelated HoT | Rake qualifiers cannot empower real Rejuvenation | Stored fact missing: genuine RED |
| Non-Feral snapshot | Existing native Soul buff/Rejuvenation snapshot persists after buff removal; live factor drops | Stored fact missing: genuine RED |
| Missing metadata | Named unavailable rather than fabricated product; unknown alias errors | Named missing-metadata contract missing: genuine RED |

Strict RED is now satisfied; production capture/accessors remain unimplemented. Main prioritized separate expected_combat_length RED/GREEN next, then the Arms148 literal route after fc1078/ta1068/ap1079 land. Preserve this RED branch for later implementation; do not relabel it ready. Setup/compilation/zero-test failures are excluded. Native jobs used ordinary arms-parity admission, one/-j2; no FIRST priority grant reuse. Lead owns full integration.
