# Native Unit form display power

## Authorized scope

Branch `form-display-power-units` starts at exact master `d9000b9cef8eb2441bb711c055da54ac206bb615`. Parent owns its separate push/gate. Reconcile display power for every native Unit; preserve separately justified player synchronization, identity, live resource contents and existing precedence. Do not patch the Feral adapter, change Ret HolyPower mapping or add resource grants/resets.

TC a352b1fa `SpellAuraEffects.cpp2049–2056` calls `UpdateDisplayPower` for nonplayers; `Unit.cpp5779–5827` selects form power before aura179 and native class defaults. Current early return without `Player` leaves authentic Ralia in Cat Form with Mana first. Native `EffectPowerBurn` also requires the target's primary power to match Energy, so this is not only a presentation issue.

## Regression boundary

Test-only `3d9352b0d0eddbe57033bfb60dab722606bebe7d` uses the production native follower initializer and real catalog/templates, not a manufactured Player or profile. Tests are in `crates/server/src/spell_cast/form_display_power_tests.rs`.

| Case | Observable requirement | Status |
|---|---|---|
| Authentic Ralia NPC Cat | Cat aura exists; actual Energy pool leads; NPC identity retained | Genuine RED: Mana instead of Energy |
| Authentic Ralia native Bite burn | Actual22568 effect1 through production `resolve_effects` consumes its authored Energy cap | Genuine RED:100→100 instead of consuming native25 |
| Player Cat | Ordinary player cast retains Energy display | Baseline control PASS |
| NPC Moonkin | Native197625 retains Mana for Crenna and restores it after Ralia Cat | Baseline control PASS |

The Bite case isolates the actual native burn effect, matching existing power-burn controls. It does not claim full finisher cost/damage, CP production, proc eligibility or numerical DPS parity. No test writes pool contents/order or invents burn amounts; the cap comes from native effect metadata.

Initial `form-display-power-red/` run at3d935 produced fixture-only failures before NPC spawn: bare spell Harness lacked WorldData. Player Cat control passed; NPC behavior was not reached. Build29m57s, cases10.67s/exit101. The fixture now loads real native creature data and model reaches, matching existing native_fixture callers. Moonkin control covers both caster Crenna and Ralia transitioning from Cat. Corrected fixture atba6b016be7bb0d8745bf3af2fa573c142ac5f160 produces genuine2RED/2controlsPASS16.19s, clean12m48s compile/exit101 (`form-display-power-native-red/`). The candidate drops the Player gate and reads native class metadata for Player, class-follower and ordinary NPC units. Fixed-form/aura179/class precedence and existing pool values are unchanged; no client-sync code changes. GREEN is pending. Existing shapeshift/power suites and the requested two-crate check have not run and must run once after the committed fix. No previous Feral literal/reader ancestry is included.
