# Native resource fact integration packet

Status: capped native Focus/Energy/Mana facts reached genuine3RED at ecaf83d7b (22.15s):0 versus11.072727/16.84/2560. Receipt: `game-engine/data/diagnostics/dps-parity-fix-2026-10-21/power-haste-packet-red-ecaf83d7b-native.log`. Producer985afd5eb exposes the uncapped native rate;14781's adapter delegates once, converts to f64 and errors on None. DisplayModifier remains applied once by `resource()`. No adapter-side `powers.rs` change, grants or cached-rate fallback. Combined native fixtures and the explicitly requested0/25% Energy unit controls await GREEN qualification.

## First combined consumer run

At d5e39eee5, compilation passed without warnings; five cases ran in26.58s. BM Focus, Mana and both0/25% Energy controls passed. WW's initial capped potential facts passed, then ordinary Tiger Palm failed `OutOfRange`: the new natural-cap wait allowed `FollowOwner` to move the monk toward the quiet owner atX20. The fixture now removes that movement controller, sets Idle and asserts unchanged actual spawn geometry across the cap wait. It does not move units, change ranges or bypass native cast validation. Revised combined GREEN remains pending; do not count the partial run as completion.

## Integration

Apply only this packet's `power_haste_tests.rs`/documentation delta onto2ad5b4b31 (already present on the producer branch), not the entire Dev branch. The copied base is recorded locally as c7107c567. Run the three existing opt-in `dps_parity_power_haste_native_*` cases with prepared253/269/66 profile inputs and a private current-canonical SQLite backup, using the existing read-only startup-source wrapper.

The fixture extension preserves ordinary Cobra Shot193455, Tiger Palm100780 and Flash of Light19750 spends. Native ticks fill the initial pool and later recap it; no resource grants, stat/gear injection or synthetic25% haste control. Expected rates reuse the original fixture's native PowerType/base-mana, live derived haste and actual aura modifiers. The original tick-rate/gain assertions remain.

## Added observable contract

- At natural cap, current=max and applied/cache/wire rate=0; `resource.regen` is the positive displayed potential rate and `time_to_max`=0.
- Immediately after the ordinary instant Focus/Energy spend, the stale cache remains0 but facts expose current potential and correct time-to-cap before the next tick. Mana may pay at cast completion, so it does not pretend that its callback preceded a producer tick.
- The actual uncapped native tick matches that same expected rate; fact reads never change current/max/cache or the replicated `PowerEntry`.
- After natural recap, cache/wire returns0 while facts remain potential. Mana retains the original non-hasted control.

These assertions must reach genuine behavioral RED against the old cached fact adapter before its production replacement. Source evidence of cached0 is not a RED receipt.

## Adapter boundary after the producer exists

`NativeFacts::regen` must delegate to `spell_cast::powers::potential_power_regen(&World, Entity, PowerType) -> Option<f32>`, convert to f64 and fail explicitly on unsupported/missing continuous pools. Remove both old pool and Mana-cache reads; no cached-zero fallback. `resource()` already applies native DisplayModifier once. Pet discrete Focus and rune recharge remain separate producers.
