# Shared native APL facts

Lead-assigned shared gameplay binding for WW/Ret. Engine-free `server::simc_apl` remains unchanged. `crate::simc_apl_native` implements its `Facts`/mandatory per-row `ActionFacts` using explicit alias registries and native gameplay state. Current seam is fixture-only (`cfg(test)`); literal drivers run only for explicit parity Fixture encounters. Stockade retains its separately requested native-kit driver; `DUNGEON_TTD_UNAVAILABLE` is a named blocker, never a silent skip/fallback. Live TTD implementation is not authorized.

## API

`Bindings { actions, buffs, talents, resources, global_cooldown_spell }` maps explicit aliases to native IDs/power types. `NativeFacts::new(&World, actor, &Bindings, Encounter, active_enemies)` returns a read-only view. `Encounter::Fixture { started_at, duration, style: FightStyle::Patchwerk }` uses the simulation clock and one shared target/fight deadline. `Encounter::Dungeon { started_at }` explicitly rejects target TTD/fight-remains. Spec adapters delegate shared facts and add only their own predicates.

Unknown aliases, properties, native spell definitions, selected talent allocation or required resource state fail explicitly. An absent declared buff is zero; an unknown buff alias is not. Talents read active `TraitLoadout`, not arbitrary KnownSpells. Plain GCD requires the proposed action; variable/call controls have zero GCD. `gcd.max` uses the actor's explicitly registered native global-cooldown spell. Native cooldown/charge/GCD readers live in `spell_cast/native_apl.rs`; shared full-duration calculation is extracted once from actual cast startup, retaining modifiers, haste, rates, category adjustments and Mobile Banking's authored exception. Reads clone cooldown state and cannot consume/retime live casts.

## Concrete coverage matrix

| Boundary | Proof | Still unqualified |
|---|---|---|
| Buff up/stack/remains; remove on repeated action | Actual native TP→BOK→BOK earns1/2 stacks then removes Hit Combo; shared values follow native state | Separate natural-expiry and permanent-buff sentinel cases; multiple-caster arbitration |
| Native resources/display units | Actual Charge grants200raw Rage→20display; actual TP earns Chi; native max/deficit | Energy time-to-max uses last published native regen rate; rate/projection edge cases not behavior-qualified here |
| Selected talents | Native starter purchases/config evaluation installs actual allocation; selected spell true, learned Provoke utility false | Rank operands unsupported explicitly; no `.rank` in current WW source census |
| Cooldowns/action GCD | Actual Provoke starts/expires native recovery; actual TP positive GCD, call control zero; actor-only plain GCD rejected | Cross-spec/category/rate/minimum-GCD integration gate remains lead-owned |
| Charges | Actual native Roll consumes one charge; remaining full recharge positive | Full natural recharge and fractional-charge arithmetic have source reuse, not separate native tests here |
| Encounter clock | Actual40native frames→2s; fight/target deadline300s global;3enemies; Patchwerk true; dungeon TTD/fight unavailable | Non-Patchwerk fixtures and live dungeon TTD not enabled |

No full literal rotation or DPS parity follows from these six cases.

## RED→GREEN ledger

| Revision | Command | Result | Validity |
|---|---|---|---|
|5924496f9 (docs tipab4a33e44)|`agent-run ww-monk-parity python3 scripts/build-host.py --build-host native --test -p server --bin game-server simc_apl_native:: -- --ignored --nocapture --test-threads=1`|Genuine6FAIL/0PASS,13.24s test,8m18compile,exit101. All real native fixture/cast/allocation setup reached unimplemented binding failures; three scaffold dead-field warnings.|Baseline proof; implementation subsequently changed.|
|dc1ee9781|Same targeted command after formal ADD|6PASS/0FAIL,17.40s test,7m37compile,exit0; zero compiler warnings.|Exact binary49562a93a94e52d1 SHA25618bcebff1ad78d91910ba647a8d65c186f89ace0c1cf174b7e99f34cdcabb790. Test activation changes no body; lead owns broad integration gate.|

Engine `data/diagnostics/ww-parity-2026-10-23/native-facts-{red,green}.{output.log,proof.json}` preserves exact invocation, admission and binary identity. First GREEN admission attempt beforedc1 was HOLD, no test started; actual GREEN atdc1 received ADD7.4/32CPU,33.2GiB free/6reserved,PSI1.45. Previous bounded RED worker01a168a8 completed; never duplicate an active run.

## Sources

- Native `ActiveAuras`, `TraitLoadout`, `PowerTypeTable`, `PowerPools` and native ordinary cast/charge/GCD helpers.
- SimC76f8118 `player/player.cpp:12359–12373` — bare talent/`.enabled` versus `.rank` semantics.
- SimC `action/action.cpp:3284–3285,3764–3793` — proposed-action GCD and actor-base maximum/minimum handling; native numeric timing remains authoritative.
- SimC `buff/buff.cpp:2018–2028`, `sim/expressions.hpp:135–138`, `util/timespan.hpp:39–40,216–219` — permanent-buff signed64 millisecond sentinel, not a fabricated finite timer.
- [Shared interpreter contract](../specs/simc-apl.md), [WW source/receipt census](ww-source-census.md).
