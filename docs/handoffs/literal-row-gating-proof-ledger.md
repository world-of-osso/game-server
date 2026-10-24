# Literal native moving/off-GCD gates

Verified2026-10-23: actual native4RED/1GREEN atd49f4951c; gating implemented afterward, GREEN pending. Parent approved generic Prepared movement/active-GCD snapshots plus shared Driver row gating, separately committed from14775's composite selection. Ele13400 reports82 source rows,7 moving rows and6 off-GCD item rows; no Ele adapter qualification is claimed here.

## Requirements and sources

[Spec](../specs/simc-apl.md#literal-native-row-gates). Pinned SimC76f8118 `action.cpp2616` compares configured moving against actual movement. `action.cpp2907` registers off-GCD scheduling for use_off_gcd plus zero trigger GCD;5213–5221 limits use during an active GCD to permitted rows. Native Prepared readiness remains authoritative; the literal option cannot waive a native cast/GCD refusal. No configured fixture movement plan exists, so `movement.distance` is a named unavailable operand, not0.

## Staged native controls

`literal_rotation/gating_tests.rs`, filter `literal_native_gating`, requires the real269 Player/profile/current private view. Native per-action admission is asserted before row-option assertions; setup failures are not RED.

- Stationary native actor: real ready instant TP admits moving0, rejects moving1.
- Native protocol Moving bit true: same real ready instant TP admits moving1, rejects moving0. No position or movement-route workaround.
- Real native TP starts an actual Player GCD; otherwise native-ready auto_attack cannot be selected without explicit row permission.
- Explicit use_off_gcd1 executes that actual auto attack without clearing the existing native GCD. This is a scheduling/control proof, not native item activation proof.
- Ready TP row with `if=movement.distance>6` fails `MOVEMENT_DISTANCE_UNAVAILABLE` and names its operand; no fabricated stationary distance.

Atd49f4951c the actual packet runs5 tests in28.52s: moving1/stationary, moving0/moving and unpermitted-GCD rows fail exactly at their behavioral assertions; named distance fails with the old generic unknown-operand error; explicit off-GCD native AA succeeds and preserves native GCD. No readiness/setup premise failed. Proof: engine `ww-parity-2026-10-23/native-integrated-red-d49f4951c/gating.{proof.json,output.log}`; database/binary/input receipts are adjacent.

Implementation reads actual native Moving and any active SpellHistory GCD into Prepared, then shared Driver applies literal row permission before selecting a native-ready execution. Native cast admission is unchanged. Invalid configured boolean values fail explicitly; movement.distance has its named unavailable error. Tests remain ignored; combined GREEN pending, no completion claim. No extra WW-only compile. Composite selection and movement routing stay with14775/14755. Native off-GCD item metadata/activation and Ele's full literal remain separate qualification scopes.
