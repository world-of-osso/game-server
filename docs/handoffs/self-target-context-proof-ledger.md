# Self-target action selection contexts

Verified2026-10-24; generic NativeAction readiness, not full spec parity.

## Native RED before behavior change

- WW same3d current-binary300s ST52567.95/3T72757.5867; ST FoF0,3T FoF19. Each receipt uses its own fresh CURRENT backup in engine `ww-parity-2026-10-23/native-after-fire-poll-3d32d68bf/`.
- Existing `literal_ww_native_st_executes_its_real_selected_kit_without_generic_starvation` genuinely fails FoF at3d (16.13s) and splitC3e4f0f727 (23.29s), exit101. No behavior fix preceded these runs.
- At3e,30 actual native Zenith samples: known/selected=true, resolved1249625, native self-cast admission=ready, patchwerk/Flurry/trinket1.has_use_buff/trinket1.cooldown.ready=1; Zenith up/full-recharge=0. A pinned Zenith target_if row is eligible but never selected. Artifacts, fresh CURRENT DB/input/binary receipts and archived RED executable: `native-ww-zenith-trace-3e4f0f727/`. Build47.88s/zero warnings; prior8aa diagnostic borrow compile failure preserved separately.
- 14781 reports exact Fire Breath native counterexample atbf82f7f07: default self-cast Prepared execution and actual enemy candidate controls pass, but unmodified fb_sc rank1 target_if row selects None. Its empower API/execution stack is not yet in the WW checkout; no independent Fire rerun claimed here.

## Cause and bounded fix

Prepared recorded admitted self-target NativeActions only under actor/default keys. target_if supplies an explicit enemy scoring context, so lookup misses before dispatch. Record those supplied context keys against the same admitted self-target Execution. No None-key fallback, foreign-target alias, enemy cast, native-permission bypass or Dev-specific shim. Ordinary hostile actions and composite planning unchanged.

Native GREEN pending: pinned Zenith source selection plus actual actor-only aura, existing30sST FoF, relevant shared regressions and fresh300sST/3T. 14781 must rerun its exact Fire Breath leaf on the shared fix; full Dev/WW parity remains open.
