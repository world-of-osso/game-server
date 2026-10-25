# Native Unit form display power

## Authorized scope

Branch `form-display-power-units` starts at exact master `d9000b9cef8eb2441bb711c055da54ac206bb615`. Parent owns its separate push/gate. Reconcile display power for every native Unit; preserve separately justified player synchronization, identity, live resource contents and existing precedence. Do not patch the Feral adapter, change Ret HolyPower mapping or add resource grants/resets.

TC a352b1fa `SpellAuraEffects.cpp2049–2056` calls `UpdateDisplayPower` for nonplayers; `Unit.cpp5779–5827` selects form power before aura179 and native class defaults. Before correction, the early return without `Player` left authentic Ralia in Cat Form with Mana first. Native `EffectPowerBurn` also requires the target's primary power to match Energy, so this is not only a presentation issue.

## Regression boundary

Test-only `3d9352b0d0eddbe57033bfb60dab722606bebe7d` uses the production native follower initializer and real catalog/templates, not a manufactured Player or profile. Tests are in `crates/server/src/spell_cast/form_display_power_tests.rs`.

| Case | Observable requirement | Status |
|---|---|---|
| Authentic Ralia NPC Cat | Cat aura exists; actual Energy pool leads; NPC identity retained | Genuine RED→GREEN |
| Authentic Ralia native Bite burn | Actual22568 effect1 through production `resolve_effects` consumes its authored Energy cap | Genuine RED100→100/native25→GREEN |
| Player Cat | Ordinary player cast retains Energy display | Baseline and current control PASS |
| NPC Moonkin | Native197625 retains Mana for Crenna and restores it after Ralia Cat | Baseline and current control PASS |

The Bite case isolates the actual native burn effect, matching existing power-burn controls. It does not claim full finisher cost/damage, CP production, proc eligibility or numerical DPS parity. No test writes pool contents/order or invents burn amounts; the cap comes from native effect metadata.

Initial `form-display-power-red/` run at3d935 produced fixture-only failures before NPC spawn: bare spell Harness lacked WorldData. Player Cat control passed; NPC behavior was not reached. Build29m57s, cases10.67s/exit101. The fixture now loads real native creature data and model reaches, matching existing native_fixture callers. Moonkin control covers both caster Crenna and Ralia transitioning from Cat.

Corrected fixture atba6b016be7bb0d8745bf3af2fa573c142ac5f160 produces genuine2RED/2controlsPASS16.19s, clean12m48s compile/exit101 (`form-display-power-native-red/`). The candidate drops the Player gate and reads native class metadata for Player, class-follower and ordinary NPC units. Fixed-form/aura179/class precedence and existing pool values are unchanged; no client-sync code changes.

Runtime06b091df30aebaeeb5e76bdf67a813189a0394d3 qualifies all4 native cases in the retained scope. The once-only broad selection registered187 core/16 external cases:182 initially passed230.16s,5 quest/Azure cases stopped at missing native game_object_locks.json beside the isolated DB. Supplying the real companion artifact (SHA56170ba4f71671245deaeea97e4504dde66a34bef4dc711e6c0a0ef6360ca090) yielded ONLY those5 retryPASS84.50s; external16 ran once/PASS18.73s. All203 unique cases are covered, no ignored/waived case, no182-case repeat or source change. Existing test files remain unchanged.

The check ran once: `server` + `server-spell-tests --tests`, exit0/99.94s; `cargo fmt --all --check` exit0. No compiler warnings in these retained logs. Receipts are `form-display-power-green/` and `form-display-power-check/` under the same diagnostics root. All test_data loads resolve to the pre-existing private54e9bb0307e3fa7382e5a9cc30a6449694e970993e47dda47a03fe8113efee90 DB, not the later canonical4442 publish. No DB mutation/publication occurred. Parent integration gate remains separate. No previous Feral literal/reader ancestry is included.
