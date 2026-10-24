# Literal FIRST / cycle target selection

## Scope

Shared Program interprets bare/explicit FIRST and cycle_targets=1/max_cycle_targets while preserving native candidate order, ordinary min/max before-if semantics/ties, and corrected run non-inheritance. No per-spec evaluator, native target/range/data change, reference change or DPS qualification. API861/compositeb15 stay frozen for parent integration. Main extended lead_arms_14775 to this slice.

## Existing behavioral RED

Exact previously compiled library binary from7cfc2e4e6, SHA231876d16f98da6d7fc063f3c13d62a9d0b204c4651e0aab3296fd335a8920eb, contains unchanged shared interpreter and queued probes. At b15 source scope, filter `simc_apl_queued_ --ignored --nocapture --test-threads=1` gives3genuineRED0.01s: bare FIRST rejected as min/max-only, cycle chooses Slam instead of native-ready third target. No rebuild. Diagnostics `arms-parity-2026-10-23/first-cycle-red.{log,proof.json}`, `first-cycle-red-binary.proof.json`. Generic native snapshot field in the wrapper receipt is not an input to these engine-free tests.

Ten further ignored probes cover explicit FIRST, chosen-target if/native refusal, pinned negative-score asymmetry, positive cap versus successful-count distinction, zero cycle, exhaustion/default restoration, strict option literals, single-global-target shortcut and unsupported candidate adapters. At b2b464cde, actual expanded37case library run:27PASS/10genuineRED,0ignored,0.00s runtime;16m17compile/0warnings. All24 established regressions and three additional controls pass; ten failures are FIRST parsing/selection, cycle readiness/caps, missing target adapter and strict options. `first-cycle-expanded-red.{log,proof.json}` and `first-cycle-expanded-red-binary.proof.json` pin exact source/binary. Wrapper native snapshot metadata is not an input to this pure interpreter scope.

## Native pinned-binary oracle (custom probes, NOT DPS references)

Pinned76f8118 engine/simc + immutable MID2 Arms profile, actual native five-second one-iteration action sequences, local item DB, fixed initial rage100. Source commands/logs/JSON and wrapper receipts are under the same diagnostics root. `run_first_cycle_red.py` runs one admitted task at a time.

| Probe | Expected source behavior | Observed qualification |
|---|---|---|
| first:-1, single | single shortcut requires>0 |0MS, positive-control fixture otherwise valid |
| first:1, single | first positive match |2actual MS at0/4.327s on main |
| first:-1, actual adds | multi loop accepts!=0 |2actual MS at0/4.421s on main |
| cycle cap1, main fails condition | cap includes first native candidate |0MS |
| cycle cap2, first add qualifies | second candidate admitted, not a score shortcut |MS on Fluffy_Pillow_1 at2.078s |
| max selector + cycle cap1 | selector takes precedence, cap cannot trim candidates |MS on Fluffy_Pillow_1 at2.078s |

Sources: pinned `action.cpp2472–2508,2943–2964,5362–5453`; strict cycle bool0/1 and signed cap from `option.cpp314–370`, `action.cpp559–561`. Candidate order/eligibility is native-adapter supplied. The native cycle branch is conditional on global non-sleeping enemy count>1; only positive cap values limit iteration. Actual binary confirms FIRST single>0 versus multi!=0; preserve this asymmetry. `simc-selector-binary.proof.json` pins clean76f8118cf source, binary SHA6415c116d22e191d436eea7e5e4485fb49ac47c265fa8706f4e7b43b4dbabba4 and profile SHA847080503d43467467dc6102416f61397f9f2a9cbb2160da614b636a7e088afc. All six probes completed successfully; no reference changed.

## Implementation / pending GREEN

Shared Rule stores optional cycle limit; strict0/1 cycle and signed integer cap are parsed once. Ordinary actions check row condition/native admission once per selected candidate; controls retain their prior behavior. target_if binds before readiness and overrides cycling. Cycling snapshots the adapter's candidate order and caps traversal before condition/admission, with no target leakage. FIRST returns its first matching candidate without retrying after a row/native refusal. Existing min/max single shortcut/tie order remains unchanged.

Added native-binary-qualified selector/cap precedence control; targeted cohort becomes38. Source implementation is committed before GREEN; acceptance and automatic enabling of the engine-free probes await actual38case result. Lead still owns broad integration.
