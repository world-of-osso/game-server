# Stockade seed1 healer replay results

Generated2026-10-23 at `e2ff0eccb0c0ad6820f2c9a1e411e1f432571e40` from the [unchanged native recording](../../../content/dungeons/healer-benchmark/README.md). [Combined report](report.md), [exact JSON](report.json), and standalone spec-number JSON/Markdown retain native results and explicit gaps. No missing follower profile receives invented metrics.

## Proof

Capacity-admitted command, run through `agent-run healer-metrics`:

```text
python3 scripts/build-host.py --build-host native --test -p server healer_benchmark_ -- --ignored --nocapture --skip capture_stockade_and_report --test-threads=1
```

14PASS/0FAIL,179.73s native test runtime. Includes concrete200/150/50 fight extraction, consumed-only shield attribution, frozen denominator, native recipient armor/DR/shields, identical-intake positive-healing determinism, recorded source scaling, actual recorded spell-block replay, non-unit consumed-point attribution, standalone reports,15 direct-heal coefficient probes, and full persisted Stockade replay. No second autonomous route run; no broad workspace/check gate claimed.

Historical metrics RED at836428a95:0PASS/2FAIL. Full persisted replay RED atdc3879f04:13PASS/1FAIL overall; autonomous raw pressure blocked native rows. Fail-fast diagnostic atc00beba81 reproduced extra source swings at tick930. Removing replay-owned source CombatTarget projection fixes the second native swing clock without changing healer policy. Earlier recorded spell-block failure and child-only Penance fixture failure remain distinguished from coefficient mismatch.

Both native actors process all1,132 recorded hits; no autonomous raw receipts. Frozen eHPS denominator1,594.3s differs from observed native combat time2,014.4s, retained separately. Both consume zero healer shield points, so combined throughput equals effective HPS; all-unit intake makes the level-scaling absorb-unit gap moot. Alert/source-geometry/partial-resistance gaps remain explicit. These are constrained recorded-pressure results, not complete dungeon healer-policy or seven-spec parity certification.

[Coefficient ledger](../healer-coefficient-ledger.md) separates source coefficients/direct amount checks from these policy-performance results.
