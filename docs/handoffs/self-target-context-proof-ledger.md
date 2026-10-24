# Self-target action selection contexts

Verified2026-10-24; generic NativeAction readiness, not full spec parity.

## Native RED before behavior change

- WW same3d current-binary300s ST52567.95/3T72757.5867; ST FoF0,3T FoF19. Each receipt uses its own fresh CURRENT backup in engine `ww-parity-2026-10-23/native-after-fire-poll-3d32d68bf/`.
- Existing `literal_ww_native_st_executes_its_real_selected_kit_without_generic_starvation` genuinely fails FoF at3d (16.13s) and splitC3e4f0f727 (23.29s), exit101. No behavior fix preceded these runs.
- At3e,30 actual native Zenith samples: known/selected=true, resolved1249625, native self-cast admission=ready, patchwerk/Flurry/trinket1.has_use_buff/trinket1.cooldown.ready=1; Zenith up/full-recharge=0. A pinned Zenith target_if row is eligible but never selected. Artifacts, fresh CURRENT DB/input/binary receipts and archived RED executable: `native-ww-zenith-trace-3e4f0f727/`. Build47.88s/zero warnings; prior8aa diagnostic borrow compile failure preserved separately.
- 14781 reports exact Fire Breath native counterexample atbf82f7f07: default self-cast Prepared execution and actual enemy candidate controls pass, but unmodified fb_sc rank1 target_if row selects None. Its empower API/execution stack is not yet in the WW checkout; no independent Fire rerun claimed here.

## Cause and bounded fix

Prepared recorded admitted self-target NativeActions only under actor/default keys. target_if supplies an explicit enemy scoring context, so lookup misses before dispatch. Record those supplied context keys against the same admitted self-target Execution. No None-key fallback, foreign-target alias, enemy cast, native-permission bypass or Dev-specific shim. Ordinary hostile actions and composite planning unchanged.

## Dev Fire Breath compatible-port GREEN

404c053b4 is compatibly cherry-picked as2605da9c0 on the Dev empower/replacement stack. Native `replace_current` admission and empowered/replacement Execution variants remain intact; only admitted self-target executions gain supplied enemy-context keys. The new WW case depends on later owner lifecycle APIs absent on this split-B base and remains owner-held, rather than importing unrelated code.

The unchanged bf82 Fire Breath counterexample executes exactly once under `--ignored --exact`:1/1PASS16.38s, compile22.11s, zero warnings. Default native self readiness and actual enemy candidate controls pass; the source-selected enemy receives the native stage1 DoT while actual dispatch remains self-targeted. Evidence root `game-engine/data/diagnostics/dev-evoker-parity-2026-10-22/`: `dev-fire-target-green-2605da9c0-state.json` records command/environment, paired inputs, binary and runtime log. BinarySHA2569a4b82cd6fcfc6128aa1cf3eb23d9c6fb065d72ed219e45136ca9ea7e1302be2; binary/input hashes unchanged.

WW GREEN remains owner-pending: pinned Zenith source selection plus actual actor-only aura, existing30sST FoF, relevant shared regressions and fresh300sST/3T. Full Dev/WW parity remains open.
