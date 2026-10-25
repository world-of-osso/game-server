# Native action-permitting channel melee

## Scope and source

Own `ww-sck-channel-melee` branch from current master53d0e1ce1792e343e25d83adccf18a22f320ff05, after Xuen reconciliation handoff. Generic melee eligibility only: use actual channel/native allow-actions permission, not cast-bar presence. Preserve casting/channel schedules, both hand timers, intent, stats/powers, coefficients, class scripts, crowd control and victim avoidance. No core spell-ID whitelist, metadata overrides or300s rerun. Momentum is a separately approved next policy, not part of this fix.

TC a352b1fa `SharedDefines.h621` defines `SPELL_ATTR5_ALLOW_ACTIONS_DURING_CHANNEL=0x1`. `Unit.cpp3107–3120` sets UNIT_STATE_CASTING only for channels without it; `DoMeleeAttackIfReady2166–2180` checks that state. Native SCK101546 Attributes5=0x2201 allows actions; FoF113656/CJL1179520x8002200 do not. `combat.rs::SwingState` had `Has<CastState>` and `snapshot_attacker` blocked every active channel. Replace only that gate with actual CastType/native permissions; leave the separate victim-avoidance cast query unchanged.

## Strict native proof

|Phase|Exact source|Scope|Result|
|---|---|---|---|
|Initial setup|4848d6af05d8d2d90cff6b7297610f6425693f90|Test compilation|E0277: AutoAttack is Clone, not Copy. No native run/RED claim.|
|Initial native|1a677210aaae13f2a931af53be5e53b1f50fada6|Four native cases|One genuine SCK RED, FoF/noncast controlsPASS; Vivify control's instant-cast premise fails and is not another behavioral RED.|
|Complete baseline|1d196cead2666190105a6444abb8bd3f168a53ee|`literal_ww_native_channel_melee_ --ignored --nocapture --test-threads=1`|One genuine RED/three controlsPASS11.67s; compiler14.72s/zero warnings.|
|GREEN|db543b270d09fed2902df733b99d1b81c16cfb26|Same four native cases after generic production gate|4GREEN27.56s; actual active SCK white deliveries0→1, all three blocking/noncast controls retained. Compiler405.74s/zero warnings.|

The SCK RED uses real TP-earned Chi, normal native channel admission and the ordinary `start_attack` request. Initial attack timer0ns comes from the normal request; an existing timer is cloned/preserved, never assigned. Real CastState remains channel101546/duration1.5s; observed white deliveries while it remains active are0. Completion-frame deliveries are excluded. FoF and normal Vivify correctly block then resume; without a cast real white delivery occurs. No resource/aura/stack/stat/timer grants, synthetic CastState or fake RNG outcomes.

Native388812 casts earn392883: -100% Vivify cast time,20s duration/one charge. The normal-cast control now waits ordinary expiration without further casts. It never strips the legitimate proc or changes the trait profile.

## Evidence and completion boundary

Engine evidence root `data/diagnostics/ww-parity-2026-10-23/`: `channel-melee-proof-ledger.json`, `channel-melee-red-1d196cead.build.{log,proof.json}`, `native-channel-melee-red-1d196cead/` and retained earlier setup receipts. Complete baseline binary SHA256 `52d4d9d71d3d8bfadc968ccc1e590e6eefafeff5fe87f897f3e37c8bb1fe1e4f`; each runtime retains fresh canonical4442/private7bad backup, frozen269 inputs and exact command/binary receipt.

Implementation was committed before GREEN. Current binary SHA256 `444066a1b8e3d97d8784955aaacd84d7ba63708ebf2923ba85272004d6af5cc2`; `channel-melee-green-db543b270.build.{log,proof.json}` and `native-channel-melee-green-db543b270/` retain exact source/input/database/binary proof. Changed Rust readability has no findings. Formatter/diff and lead-owned broad/plain production --bins gates are recorded separately; no full-gate claim. Documentation-only follow-up does not invalidate native source proof. This boundary can affect natural Flurry earning but does not allocate the84-versus178 auto-attack gap or certify numerical DPS parity.

[Contract](../specs/dps-parity.md), [combat representation](../wiki/systems/combat.md#action-permitting-channels--2026-10-24).
