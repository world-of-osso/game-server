# Native resource fact integration packet

Status: test-only packet extending13843's2ad5b4b31 fixtures; not yet compiled or executed here. No `powers.rs` change and no new grant build.14781 owns the `NativeFacts::regen` adapter;13843 owns the uncapped native rate producer;13120 owns the Energy counterexample contract.

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
