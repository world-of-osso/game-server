# Selected RWK landed-BOK producer

Policy: `SIMC_WW_RWK_BOK_LANDED_PRODUCER`. Verified2026-10-24. Native GREEN pending; ww1081 remains404c.

## Authority

- SimC76f8118cf3f7f41dad20708e8328e1f4ab2deebc `engine/class_modules/monk/sc_monk.cpp1387–1424`: reject non-landed BOK, inspect active Combo Breaker, roll selected RWK effect1 percentage, reset RSK with `reset(true)`, trigger RWK override, then decrement Combo Breaker. Producer lines1408–1414.
- TC a352b1fa entire `src/` search `1250566|1250554|467307|rushing_wind_kick` has no match/binding. This is a named SimC eligibility policy, not a claimed TC script.
- CURRENT native copy:1250566 effect0 is aura4/60 with no TriggerSpell and no SpellAuraOptions row. `tdb_spell_proc` has no1250566/1250554/467307 rows. Actual query receipt: engine `ww-parity-2026-10-23/native-ww-rwk-producer-red-404c053b4/rwk-producer-authority.json`.
- DB2 supplies60%,1250554's aura332→467307 and the native RSK/RWK charge category1518 (MaxCharges1/ChargeRecoveryTime10000ms); RSK GCD1500ms. No fixed reduction constant is copied from SimC. RWK success is a full-reset operation, implemented by reducing the actual complete native recovery through existing gameplay timers/owner notifications. Separate Sharp Reflexes/Zenith BOK numeric reductions at1392–1405 are not implemented by this bounded producer.

## Scope

Actual caster TraitLoadout must select1250566; only its owned native passive supplies the amount. Active116768 plus ordinary landed100784 is required. Instant native BOK resolves its OnHit before `spell_mods::consume_charges`, so the real Combo Breaker state remains visible; misses never run the hook. Successful proc triggers native1250554 and the source-earned native RSK/shared-charge reset, never fixture resources/GCD/position changes. Damage coefficients, target arithmetic, other talents, WDP/Flurry/DT and override-consumer changes excluded until independently qualified.

## Proof

Existing `literal_ww_native_combo_breaker_blackout_earns_rwk_and_delivers_its_child`: actual404 natural Combo Breaker + BOK damage controls pass,1250554 assertion fails31.63s/exit101. Fresh CURRENT read-only backup, exact command/env/profile/binary receipt and archived executable under `native-ww-rwk-producer-red-404c053b4/`. Canonical215d75ec02c0311831cf5d98a4432c985500345811471fcce6ed23fee7709557/copycd7230ac1d62bf7d7ea8fef5fbb82faa9f36b61ec78108e591f1204c614a8ed6.

206a implementation committed before verification. Five normal14754 admission checks HOLD (PSI6.25–16.03%, last RAM6.3GiB); no build/job started. Explicit selected-owner gate and policy ledger follow; native467307/468179, actual override consumption, negative eligibility controls and fresh300s receipts remain required. No GREEN/parity claim.
