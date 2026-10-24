# Shared resumable precombat polling

Verified2026-10-23: RED scaffold/tests staged, unexecuted. Parent prioritized14779's blocked Fire caller before the remaining WW gap. Previous package96b27GREEN is separate proof, not qualification of this new API.

## Caller and proposed contract

Forwarded origin/rotation-b-fire-literal543e7aca4 currently pins88rows/8lists and a native-order test/spec; its Rotation/FireFacts hookup remains14779-owned. Rotation owns Driver; dps_parity::advance_combat owns Run.app. Shared `Driver::poll_precombat(...) -> PrecombatPoll::{Waiting, Selected { action, execution }, Finished}` must distinguish native cast/GCD wait from exhausted source, preserving cursor, variables and original Action.options while waiting. Caller advances real App frames and resamples Prepared/facts, never sleeps or instant-loops casts.

The caller starts its300s fight epoch after setup. Finished does not imply no missile: Pyroblast11366's actual native flight is read through7826726dc (accepted here as86eba5bab), not a precast option, elapsed-time estimate or new timer. Actual Fire precombat rows are Arcane Intellect, variable value/reset/default/set rows, snapshot, Mirror Image, plain Pyroblast; there are no cast/travel options on those casts.

Combat Fire Blast/Combustion rows require declared use_off_gcd1/use_while_casting1 in addition to native permission. Neither flag can waive native cast/GCD refusal or interrupt an existing Pyroblast. Item names/potion/PI options remain attached to Action; native inventory/provider gaps must not become invented buffs or a Fire-only executor.

## Staged native controls

`literal_rotation/precombat_poll_tests.rs` uses actual prepared Player63/current private DB and real native admission before assertions. All stay ignored.

- Arcane Intellect starts native GCD; next poll must wait rather than consume remaining rows/variables or finish.
- Resume through source-plain Mirror Image→real hardcast11366, with constant/reset/default/set/snapshot fixture rows testing the shared cursor contract. No skipped/replayed native CastSuccess; actual cast duration and drained cast/GCD required. This focused boundary does not claim all88 FireFacts/source rows.
- After Finished, record actual782 flight/remains; a hardcast before launch is not flight. Caller creates the300s epoch afterward and observes time0.
- During actual Pyroblast, native-ready Fire Blast/Combustion require literal while-casting permission; permitted native instants preserve the real Pyroblast CastState.

Current poll method deliberately delegates old select behavior: no busy-state implementation before observed native RED. No fabricated stats/resources/travel, canonical writes, explicit engagement or Fire coefficients are introduced. Full source integration remains14779's proof, alongside these shared boundary controls.
