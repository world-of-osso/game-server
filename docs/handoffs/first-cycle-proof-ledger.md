# Literal FIRST / cycle target selection

## Scope

Shared Program interprets bare/explicit FIRST and cycle_targets=1/max_cycle_targets while preserving native candidate order, ordinary min/max before-if semantics/ties, and corrected run non-inheritance. No per-spec evaluator, native target/range/data change, reference change or DPS qualification. API861/compositeb15 stay frozen for parent integration. Main extended lead_arms_14775 to this slice.

## Existing behavioral RED

Exact previously compiled library binary from7cfc2e4e6, SHA231876d16f98da6d7fc063f3c13d62a9d0b204c4651e0aab3296fd335a8920eb, contains unchanged shared interpreter and queued probes. At b15 source scope, filter `simc_apl_queued_ --ignored --nocapture --test-threads=1` gives3genuineRED0.01s: bare FIRST rejected as min/max-only, cycle chooses Slam instead of native-ready third target. No rebuild. Diagnostics `arms-parity-2026-10-23/first-cycle-red.{log,proof.json}`, `first-cycle-red-binary.proof.json`. Generic native snapshot field in the wrapper receipt is not an input to these engine-free tests.

Ten further ignored probes cover explicit FIRST, chosen-target if/native refusal, pinned negative-score asymmetry, positive cap versus successful-count distinction, zero cycle, exhaustion/default restoration, strict option literals, single-global-target shortcut and unsupported candidate adapters. Production not changed yet; expanded runtime RED pending.

## Native pinned-binary oracle (custom probes, NOT DPS references)

Pinned76f8118 engine/simc + immutable MID2 Arms profile, actual native five-second one-iteration action sequences, local item DB, fixed initial rage100. Source commands/logs/JSON and wrapper receipts are under the same diagnostics root. `run_first_cycle_red.py` runs one admitted task at a time.

| Probe | Expected source behavior | Observed qualification |
|---|---|---|
| first:-1, single | single shortcut requires>0 |0MS, positive-control fixture otherwise valid |
| first:1, single | first positive match |2actual MS at0/4.327s on main |
| first:-1, actual adds | multi loop accepts!=0 | Pending actual binary |
| cycle cap1, main fails condition | cap includes first native candidate | Pending actual binary |
| cycle cap2, first add qualifies | second candidate admitted, not a score shortcut | Pending actual binary |

Sources: pinned `action.cpp2472–2508,2943–2964,5362–5453`; strict cycle bool0/1 and signed cap from `option.cpp314–370`, `action.cpp559–561`. Candidate order/eligibility is native-adapter supplied. The native cycle branch is conditional on global non-sleeping enemy count>1; only positive cap values limit iteration. Do not normalize FIRST single>0 versus multi!=0 without actual binary proof. Preserve every historical receipt; no ready claim until expanded RED/GREEN and native probes settle.
