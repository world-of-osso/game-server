# Consolidated healer RED baseline

Lead approved consolidating remaining ignored RED cases (2026-10-23) after genuine metric extraction RED at836428a95:0PASS/2FAIL,7.39s; cold build14m28s. Avoid repeated capacity admission/build cycles for each historical checkpoint.

This explicit negative baseline retains current test interfaces, source fixtures and native mitigation primitives but restores the original017da11d9 recorder output, disconnects raw-hit/consumed-point observation hooks, leaves replay unimplemented, source-snapshot application as a no-op, and per-spec Markdown writing unimplemented. No runtime switch or compatibility path is introduced. The subsequent implementation commit restores the c07d44ceb implementation tree before GREEN execution.

Target only genuinely missing business behavior: `healer_benchmark_ --ignored --nocapture --test-threads=1`, excluding coefficient probes, native spell/melee mitigation controls and explicit capture/report drivers. Those controls and probes run with the implementation GREEN batch. A compile/setup error is not behavioral RED.

Proof commands, exact revisions and results remain in `/var/tmp/osso/healer-metrics/proof-ledger.json`; final tracked evidence is added after native execution. No dungeon intake or native metrics are claimed before capture.
