# Shared APL proof ledger

Parent assigned shared literal SimC76f8118 expression/variable/list decisions for Ret and WW on2026-10-23. Ret facts/native adapter belongs to14752; WW remains this branch. Separate from generic weapon AP through55c47b72f.

| Revision / scope | Command | Result | Validity |
|---|---|---|---|
| c0cf6eed6, RED scaffold and11 ignored tests pushed first | `agent-run ww-monk-parity python3 scripts/build-host.py --build-host native --test -p server --lib simc_apl:: -- --ignored --nocapture --test-threads=1` | Genuine11FAIL/0PASS,30.68s compile,0.00s test,exit101; no compiler errors/warnings. Runtime not-implemented decisions and malformed-source acceptance | Baseline proof; production subsequently changed |
| aff61cbab, shared interpreter | Same targeted command | 11PASS/0FAIL,1m03 compile,0.00s test,exit0; zero compiler warnings | Current numeric/list implementation; tests enabled afterward. Additional recursion assertion passes focused1/1 at2d3179aaf,12.68s compile,0.00s test,exit0,zero warnings |

Engine `data/diagnostics/ww-parity-2026-10-23/shared-apl-{red,green}.proof.json` preserves binary identity. GREEN SHA256 `8ec773229fb6edc9c7f6cd207bf6aa580412f4510e908063f1a2758f3ae0b115`. Both actual runs follow formal `CAPACITY_ID=lead_ww_14754` ADD and `agent-run`; parent explicitly extended priority to APL.

## Action-context correction pending

The first11 cases used action-invariant fixtures. Exact source discovery: `sc_monk.cpp208–211` binds combo_strike to the proposed action; `action.cpp3284–3285` binds plain gcd to that action, and call controls at4648+ have trigger_gcd0. An actor-wide Facts dictionary cannot represent these. New mandatory `ActionFacts::value_for(&Action, operand)` is declared without a fallback; two ignored WW/Ret context REDs precede evaluator wiring. Runtime RED pending. No native literal-rotation claim follows from the earlier11-case library proof.

## Coverage and open boundaries

- Numeric/boolean/comparison/arithmetic semantics, `%` division/`%%` modulo; concrete WW Tiger Palm, Ret ES and trinket expressions.
- Explicit operand validation (including inactive branches), declared defaults/set/setif/reset and source-ordered conditional/empty call lists; native first-usable callback.
- Missing operand/list/control/variable operations fail, not false or a legacy rotation fallback.
- Native fact bindings, target_if/cancel_if/item-option execution, per-spell source matrices, native300s ST/3T DPS and Stockade remain open. Opaque action options are preserved for native adapters, never claimed interpreted by this library.

[Contract](../specs/simc-apl.md), [capability matrix](../wiki/systems/simc-apl.md), [WW inventory](ww-source-census.md).
