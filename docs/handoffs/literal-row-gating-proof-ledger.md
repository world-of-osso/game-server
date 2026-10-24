# Literal native moving/off-GCD gates

Verified2026-10-23: genuine4RED/1GREEN atd49f4951c →5GREEN19.50s at96b572797. Tests remain ignored/prepared-view-only. Ele13400 reports82 source rows,7 moving rows and6 off-GCD item rows; this is shared gate qualification, not full Ele/item activation proof.

## Requirements and implementation

[Spec](../specs/simc-apl.md#literal-native-row-gates). Pinned SimC76f8118 `action.cpp2616` compares configured moving with actual movement;2907 registers off-GCD scheduling for use_off_gcd plus zero trigger GCD;5213–5221 limits active-GCD scheduling to permitted rows.

Prepared reads actual native Moving and any active SpellHistory GCD; shared Driver filters literal row permissions before selecting an otherwise native-ready execution. Native cast admission is unchanged: the option cannot waive GCD/cast refusal. Invalid configured booleans fail explicitly. Configured `movement.distance` has no fixture plan and fails `MOVEMENT_DISTANCE_UNAVAILABLE`, never fabricated0. Snapshot/gating commits are separate from14775's composite selection; no extra WW-only compile.

## Native coverage

`literal_rotation/gating_tests.rs`, filter `literal_native_gating`, uses the real269 Player/profile/current private view. Every relevant native admission premise passes before the behavioral assertions.

- Stationary: real ready instant TP admits moving0, rejects moving1.
- Native protocol Moving true: same instant TP admits moving1, rejects moving0; no position workaround.
- Actual TP-started Player GCD: otherwise native-ready auto_attack cannot be selected without row permission.
- Explicit use_off_gcd1 executes that native AA without clearing the existing GCD.
- Ready TP with `if=movement.distance>6` returns the named unavailable operand error.

## Evidence and exclusions

Engine `ww-parity-2026-10-23/native-integrated-{red-d49f4951c,green-96b572797}/gating.{proof.json,output.log}` preserves the progression. Adjacent database/binary/input receipts retain full source/copy hashes and exact commands. Current fresh source215d75ec.../copycd7230ac...; binarySHA15509d9a... . No canonical mutation, resource grant or invented movement distance.

Native off-GCD item metadata/activation, Ele's complete literal and empower/chain plumbing are separate qualification scopes. The AA control proves scheduling, not item activation.
