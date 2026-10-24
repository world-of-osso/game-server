# Native Pack/resource proof ledger

## Revision and inputs

- RED `fe855f75b`: Pack2FAIL/1PASS18.24s; real Player launch has no1282660 event, actual Austin300s kit has admitted Barbed casts without the reward. Removed-passive control passes. Compiler/setup failures are not counted.
- GREEN `ccfb44c1977dd76eafb6ef21ba23ade0d5ebee20`: Pack3PASS32.01s; native Rage-center1PASS14.48s. One `cargo test --locked -p server --lib ... --no-run --message-format=json-render-diagnostics` compile26m55s; compiler JSON selects `server-3270869ea8fa4b0c`.
- ELF SHA256 `da3987fdb00ba83886d392b7c86f7acfaa72b21dfc2dd64a6fd4c2d14c5b3ce5`; private native world SHA256 `dd1de231e32c26291182dff050a8de5a5132908723ed21454eb128e45e4b05ae`.
- Adapter `d5e39eee5` is included in the tested source. Subsequent merge `f9f240c02` adds only7216b1dc9's separate geared resource cap-wait geometry fixture/documentation; Pack and ordinary Rage test paths are unchanged. Corrected WW requires the parent gate; no duplicate local run.

## Behavioral coverage

| Contract | Exact proof |
|---|---|
| Real geared Dwarf Player normal Barbed launch grants native1282660 before any next frame | PASS; first actual full-pool overflow then real Cobra spend/second partly realized reward |
| Actual321014 removal suppresses reward | PASS; no instant child event |
| Actual Austin300s native kit grants once per admitted Barbed | PASS; event count equals casts, total equals native child amount ×casts, effective gain positive |
| Native unflagged Rage at center has no phantom positive potential | PASS after real combat-reference termination; peace phase/current center/cache0 asserted |
| Shared Focus/Energy/Mana facts lifecycle | Separate [consumer packet](native-resource-facts-packet.md); four consumer cases pass atd5e39, WW initial facts pass but later spend exposed FollowOwner range fixture failure;7216 correction awaits parent full gate |

The first Rage probe atfe855 failed the fixture's peace-state guard because AttackStop retains combat status. It is not gameplay RED;74f4b8ce7 uses native `combat_refs::end_all_combat`, not direct stats/pools/timers.

## Commands and receipts

All native execution uses `agent-run dps-parity-fix`, one process/thread, capacity `lead_pcrit_13843`, private snapshot through `scripts/dps_parity_native.py`. Filters:

```text
dps_parity_pack_tactics_native_ --ignored --nocapture --test-threads=1
dps_parity_power_haste_unflagged_rage_at_center_has_no_phantom_gain --nocapture --test-threads=1
```

Diagnostics root: `game-engine/data/diagnostics/dps-parity-fix-2026-10-21/`.

- `power-haste-pack-red-fe855f75b-{compile.log,native.log,executable.json}`
- `power-haste-pack-adapter-green-ccfb44c19-{compile.log,native.log,rage-center.log,executable.json}`
- `proof-ledger.jsonl` records exact argv, revision, environment, protocol boundaries and input digest.

The superseded unmerged74f GREEN worker was stopped during admission-HOLD before any Cargo/rustc child existed. Its runtime-loss notice is expected, not a cancelled compile or failed proof.

## Boundaries

Native321014 description explicitly names1282660; native child effect30 supplies25 Focus/self. SimC76f8118 `sc_hunter.cpp5091–5092` corroborates execute timing. The existing Barbed callback still starts periodic246152 independently. No stats, coefficient, timer, rotation or canonical DB is changed. [Contract](../specs/spell-dummy-scripts.md#twentieth-batch-authored-totals-and-next-spell-uses).

These receipts establish mechanics, not whole-BM DPS parity or broad integration readiness. Snakeskin and generic APL-derived enemy-count classification remain later authorized work.
