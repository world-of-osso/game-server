# Selected RWK landed-BOK producer

Policy: `SIMC_WW_RWK_BOK_LANDED_PRODUCER`. Verified2026-10-24. Native7/7 GREEN atf2c66e0a7; parent ww1081 now points there. Numerical parity remains open.

## Authority

- SimC76f8118cf3f7f41dad20708e8328e1f4ab2deebc `engine/class_modules/monk/sc_monk.cpp1387–1424`: reject non-landed BOK, inspect active Combo Breaker, roll selected RWK effect1 percentage, reset RSK with `reset(true)`, trigger RWK override, then decrement Combo Breaker. Producer lines1408–1414.
- TC a352b1fa entire `src/` search `1250566|1250554|467307|rushing_wind_kick` has no match/binding. This is a named SimC eligibility policy, not a claimed TC script.
- CURRENT native copy:1250566 effect0 is aura4/60 with no TriggerSpell and no SpellAuraOptions row. `tdb_spell_proc` has no1250566/1250554/467307 rows. Actual query receipt: engine `ww-parity-2026-10-23/native-ww-rwk-producer-red-404c053b4/rwk-producer-authority.json`.
- DB2 supplies60%,1250554's aura332→467307 and the native RSK/RWK charge category1518 (MaxCharges1/ChargeRecoveryTime10000ms); RSK GCD1500ms. No fixed reduction constant is copied from SimC. RWK success is a full-reset operation, implemented by reducing the actual complete native recovery through existing gameplay timers/owner notifications. Separate Sharp Reflexes/Zenith BOK numeric reductions at1392–1405 are not implemented by this bounded producer.

## Scope

Actual caster TraitLoadout must select1250566; only its owned native passive supplies the amount. Active116768 plus native landed100784 is required. Instant native BOK resolves its OnHit before `spell_mods::consume_charges`, so the real Combo Breaker state remains visible; misses never run the hook. Successful proc triggers native1250554 and the source-earned native RSK/shared-charge reset, never fixture resources/GCD/position changes. Damage coefficients, target arithmetic, other talents and WDP/Flurry/DT excluded. The separately observed consumer boundary below adds only source1199–1203 one-use expiration.

## Proof

Existing `literal_ww_native_combo_breaker_blackout_earns_rwk_and_delivers_its_child`: actual404 natural Combo Breaker + BOK damage controls pass,1250554 assertion fails31.63s/exit101. Fresh CURRENT read-only backup, exact command/env/profile/binary receipt and archived executable under `native-ww-rwk-producer-red-404c053b4/`. Canonical215d75ec02c0311831cf5d98a4432c985500345811471fcce6ed23fee7709557/copycd7230ac1d62bf7d7ea8fef5fbb82faa9f36b61ec78108e591f1204c614a8ed6.

206a first admission remained HOLD; no build started there. At9604fae2d, normal14754 ADD and -j2 compile409.49s/zero warnings. Actual300s producer case RED→GREEN18.91s proves1250554,467307 and468179 damage; native declared unselected1250566 control GREEN16.59s after natural Combo Breaker/BOK. Exact receipts in `native-ww-rwk-producer-9604fae2d/`.

Same binary fresh CURRENT-backup EACH300s: ST79001.5133/−61.7322%,3T101185.7333/−73.1289%, both exit0. ST RWK21casts/21childhits/1927409damage;3T14/14/970655. Receipts `native-after-rwk-producer-9604fae2d/`. These qualify producer/basic child only; three-target child arithmetic/recipients are not qualified. Actual reset/one-use override restoration test added next; no consumer shim before observing that boundary. Numerical parity, full producer package and ww1081 advancement remain open.

## Actual consumer RED —8a5ff975d

Exact native natural Combo Breaker→RSK spent shared charge→landed BOK earned1250554→actual full-recharge zero→Prepared-admitted467307→468179 damage premises all pass. The actual use leaves1250554 active, failing one-use/RSK-restoration assertion9.89s/exit101; build40.45s/zero warnings. Fresh CURRENT/input/binary/archive receipts at `native-ww-rwk-consumption-8a5ff975d/`.

This is not a generic proc event fix: TC SpellMgr.cpp1660+ leaves OVERRIDE_ACTIONBAR_SPELLS332 outside isTriggerAura; native handlers::can_proc/proc_matches agree. DB2's charge1/cast mask alone cannot make a non-trigger aura consume, and no TDB override exists. SimC sc_monk.cpp1199–1203 explicitly expires the RWK buff after action execution. Add that selected-owner AfterCast consumer and remove only owned1250554 with Consumed reason; leave generic332/proc rules unchanged. Atf2c66e0a7 exact reset/consumer case RED→GREEN11.82s; same binary producer25.49s, native unselected-owner13.75s, Zenith12.85s,30sSTFoF19.77s,3T SCK/all3recipients14.24s and actual FoFchannel9.83s all GREEN (7 unique native cases). Normal14754/-j2 build429.36s,zero warnings. Exact receipts: `native-ww-rwk-green-f2c66e0a7/`.

Fresh CURRENT-backup EACH same-binary300s: ST77850.54 (−62.2897%),3T92544.2467 (−75.4238%), both exit0; `native-after-rwk-consumption-f2c66e0a7/`. ST RWK10casts/10childhits/921373damage/4.7%override uptime;3T6/6/410922/2%. Actual one-use behavior replaces the unqualified persistent override. AoE arithmetic/recipients, missed-hit/unrelated-action/owner-isolation controls, WDP/Flurry/DT and all-spec numerical parity are not claimed.
