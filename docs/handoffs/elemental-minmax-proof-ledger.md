# Elemental-required shared numeric min/max

## Scope and authority

Only shared `Expression` minimum/maximum semantics, not an Elemental literal rewrite or native gameplay change. SimC76f8118 `expressions.cpp132–146` maps `<?` to maximum and `>?` to minimum;213–229 uses C++std::min/max;1104–1106 gives precedence5 between comparison4/add6. Native comparison/tie behavior is retained.

## Coverage

| Feature | Concrete behavior | Proof |
|---|---|---|
| Minimum `>?` | 3>?5=3,7>?5=5; Elemental target-count formula gives21 at3 targets/45 at7 | RED→GREEN |
| Maximum `<?` | 3<?5=5,7<?5=7 | RED→GREEN |
| Precedence | 2+3>?4*2=5;2+3<?4*2=8;10>?2+3<?8=8; comparisons remain lower precedence | GREEN |
| Existing expressions/lists/context/targets | All24 previous shared controls | GREEN in same26-case run |

## Exact evidence

- Base master8e7c7a685 merged before tests. Test-only170b82d9c:2 valid-expression parse RED (`unsupported syntax at ?5`),0compiler warnings. Productione34b6091f only adds operator variants/tokens, precedence and numeric evaluation.
- At735a60ac632c9b28b099f1a505ea4867196b1eda: `agent-run elemental-parity-shuja cargo test --locked -j2 -p server --lib simc_apl:: -- --include-ignored --nocapture --test-threads=1` gives26PASS/0FAIL/0ignored,15m59s compile/0warnings. New cases enabled afterward with unchanged bodies.
- Before/after server735a60ac6 and shared2abc7fa844f6e71d93203ce5421ae853e884854d identical. Library ELF SHA256 `89dc5ce973af666a4ad44d952f083bcd02de26004658bd6d5108ed750223e162`.
- Durable artifacts: `/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/elemental-parity-262-2026-10-23/minmax-red-170b82d9c/` and `minmax-green-735a60ac6/`; command logs/status, command/revision proofs and dependency/binary provenance retained.

No native coefficient/stat/RNG/book changes. Elemental fixture-driver integration and other shared extensions remain separate. Parent owns final integration/check/format/readability gates; no redundant proof replay requested.
