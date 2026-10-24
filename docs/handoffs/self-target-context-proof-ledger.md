# Self-target action selection contexts

Verified2026-10-24; generic NativeAction readiness, not full spec parity.

## Native RED before behavior change

- WW same3d current-binary300s ST52567.95/3T72757.5867; ST FoF0,3T FoF19. Each receipt uses its own fresh CURRENT backup in engine `ww-parity-2026-10-23/native-after-fire-poll-3d32d68bf/`.
- Existing `literal_ww_native_st_executes_its_real_selected_kit_without_generic_starvation` genuinely fails FoF at3d (16.13s) and splitC3e4f0f727 (23.29s), exit101. No behavior fix preceded these runs.
- At3e,30 actual native Zenith samples: known/selected=true, resolved1249625, native self-cast admission=ready, patchwerk/Flurry/trinket1.has_use_buff/trinket1.cooldown.ready=1; Zenith up/full-recharge=0. A pinned Zenith target_if row is eligible but never selected. Artifacts, fresh CURRENT DB/input/binary receipts and archived RED executable: `native-ww-zenith-trace-3e4f0f727/`. Build47.88s/zero warnings; prior8aa diagnostic borrow compile failure preserved separately.
- 14781 reports exact Fire Breath native counterexample atbf82f7f07: default self-cast Prepared execution and actual enemy candidate controls pass, but unmodified fb_sc rank1 target_if row selects None. Its empower API/execution stack is not yet in the WW checkout; no independent Fire rerun claimed here.

## Cause and bounded fix

Prepared recorded admitted self-target NativeActions only under actor/default keys. target_if supplies an explicit enemy scoring context, so lookup misses before dispatch. Record those supplied context keys against the same admitted self-target Execution. No None-key fallback, foreign-target alias, enemy cast, native-permission bypass or Dev-specific shim. Ordinary hostile actions and composite planning unchanged.

## Native GREEN —404c053b4

Zero-warning targeted -j2 build39.58s; native4/4: pinned Zenith source selection/actual actor-only aura18.16s, prior30sSTFoF RED→GREEN13.84s,3T SCK/all3actualrecipients12.13s, actual FoF channel/117418damage23.83s. Same binary relevant25/25: native lifecycle9, composite selection7, composite lifecycle1, real coating bars1, row gates5 and Fire poll2. These29 current simulation cases exclude prior core Mystic4. Exact commands/env/current DB/input/binary receipts: `native-ww-self-context-green-404c053b4/`.

Fresh CURRENT-backup EACH actual300s: ST72147.08/−65.0524%,3T91571.0967/−75.6822%, both exit0. Source215d75ec02c0311831cf5d98a4432c985500345811471fcce6ed23fee7709557/copycd7230ac1d62bf7d7ea8fef5fbb82faa9f36b61ec78108e591f1204c614a8ed6; `native-after-self-context-404c053b4/`. Native Zen5casts/33.3333% uptime; ST FoF21casts/6791086 regular-child damage,3T FoF16/6121702. No numerical parity claim or isolated single-change DPS claim across integration.

14781 must still rerun its exact Fire Breath leaf on the shared fix; no local empower-stack/full Dev qualification. WW missing selected RWK/WDP/Flurry/DT and item/consumable/race qualification remain open.
