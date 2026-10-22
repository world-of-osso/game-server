# Echo and shared rolling proof ledger — 2026-10-22

## Scope and authority

Native77485→77489 producer, then coordinated generic Attr10 ROLLING_PERIODIC
refresh for damage/healing. No Holy policy edits, canonical DB edits, invented
mastery/stat/proc amounts or Fire Ignite producer implementation. Shared follower
mastery92313031f landed first; actual Henry amount is asserted positive before
counting a producer RED. SimC76f8118 priest.hpp1464–1476/residual_action.hpp71–108
supplies named `HOLY_ECHO_RESIDUAL_ON_NATIVE_AMOUNT`; local70448 amounts/clock win.
Pinned TC a352b1fa SpellAuraEffects832–849 supplies generic carry. Native Echo is
four seconds/two-second period, not historical six seconds.

## Inputs and commands

S: `/home/osso/.worktrees/game-server-healer-ai`; separate T:
`/home/osso/.worktrees/game-server-healer-sim`; own protocol2abc7fa844f6e71d93203ce5421ae853e884854d.
Approved helper: `agent-run healer-ai python3 T/scripts/build-host.py --root T`.
Targeted test args: `--test -p server --bin game-server --profile test -- --nocapture --test-threads=1 <named filters>`.
Check args: `--check -p server`. Compiles use capacity admission, one owned job.
The paired frozen DB remains
`6b8b5e96b2a1f0caac05df74abce559516cacae9279d85e669d97edfd0dd7f2c`.
No T edits during jobs. Local sim-profile config is never committed.

Evidence root D: game-engine
`data/diagnostics/healer-ai-2026-10-21/echo-of-light/`.

## Proof ledger

| Code / scope | Observed result | Artifact |
|---|---|---|
| 62dd452a6 after bridge,4native producer/negative cases | 2PASS/2FAIL/1nextignored28.88s; both missing77489 only after native mastery/direct hit prerequisites | `native-producer-red.log` |
| a286b0383 producer, same4cases | 4PASS/1nextignored41.00s; actual pre-overheal/native clock/self effective healing/expiry, Renew/unlearned negatives | `native-producer-green-attempt1.log` |
| a286b0383 exact compiled ignored Echo rolling | Genuine RED21.06s: paid948 vs old unpaid473.88202 + fresh948.3356; original phase prerequisites passed | `native-rolling-red.log` |
| 968399598 generic71023 damage + unflagged139 + Echo refresh + two-Henry ownership | 2PASS/2FAIL32.35s: native damage5vs8; Echo lost pool. Unflagged/ownership controls passed | `generic-native-rolling-red.log` |
| 5622db488 first shared carry | 12PASS/1FAIL49.27s:335467 fractional snapshot oracle exposed unnecessary storage rounding; all generic/Echo/Barbed cases passed | `generic-native-rolling-green-attempt1.log` |
| 29e0d8a61 fix + clean master merge aeeaa1b87 | 19PASS/0FAIL/0ignored77.83s; compile7m36s. Preserves fractions/payment rounding, no test weakening | `generic-native-rolling-green-attempt2.log` |

Current19-case proof covers generic2, Echo6, Barbed4,3354671, Binding5, PoH1.
No unchanged broad suites rerun. Later docs-only commits do not invalidate code
proof. Check/fmt/readability receipts are recorded separately when terminal.

## Shared interface and limits

No new public helper: the ordinary child application path passes its actual
SpellInfo into `refresh::prepare_application(world,target,new,spell)` and reads
the flag. A consumer supplies sourced per-tick BP0 via the existing
`cast_triggered_with_effect0_amount`; the shared refresh handles due-tick
settlement and unpaid carry. Preserve native owner/recipient and ordinary
coefficient/eligibility rules.12654 producer proof is owned elsewhere.

Barbed's fresh five-amount normalization remains; shared carry owns its old pool.
335467's duplicated spell-ID carry was removed, retaining its fractional values.
The schedule counts retained native phase, preserving seven-initial/six-refreshed
Barbed behavior. No new generic periodic clock, estimator snapshot representation
or fractional-tick model was introduced. Passing these boundaries does not certify
every flagged spell producer or dynamic multiplier interaction.

Prismatic Echoes390967's separate mastery bonus remains open. Native mechanic
proof is not policy admission, simulator healing efficacy/survival, or live pixels.
