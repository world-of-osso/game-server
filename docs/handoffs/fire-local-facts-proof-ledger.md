# Fire-local native facts

## Scope

`firestarter.active`, `scorch_execute.active`, and caster-owned `hot_streak_spells_in_flight`. Generic duration/react/cast/history/equipment facts remain with their shared owners. Caller qualification remains separate.

## Source

- Pinned76f8118 `sc_mage.cpp6925–6957`: selected talent plus inclusive recipient health threshold; >=Firestarter, <=Scorch.
- Native70448 canonical `spell_effect`:205026 Effect0=90;2948 Effect1=30. Implementation reads the native SpellStore effect, not literal numeric thresholds.
- Pinned `sc_mage.cpp6988–7010`: sum eligible actual travel events. Native query counts caster-owned PendingSpellHits through the same Hot Streak eligibility predicate as native hit handling; hardcasts/ongoing burns are not pending events. No new timer or queue.

## Genuine RED

Revision `3014664cf92d1ff7edd98ebf9a3384fd649240c9`, maintained build-host `--test -j2 -p server-simulation --lib --profile sim` (server/server-simulation opt-level1), native runner, Player90 current profile, seed70448. Three behavioral failures: unknown firestarter.active, unknown scorch_execute.active and unknown hot_streak_spells_in_flight. Startup loader inode case passed. One additional overlap case failed setup (Pyro already impacted before Fireball launch), **not feature RED**.

- Compile17m21s, zero warnings; cases26.53s; startup1PASS +3behaviorFAIL +1setupFAIL.
- Log `game-engine/data/diagnostics/rotation-fidelity-2026-10-21/fire-local-3014-current-red.log`.
- Fresh CURRENT canonical SQLite.backup20261024T084253Z; canonical SHA256 `215d75ec02c0311831cf5d98a4432c985500345811471fcce6ed23fee7709557` unchanged across backup. Private SHA256 `cd7230ac1d62bf7d7ea8fef5fbb82faa9f36b61ec78108e591f1204c614a8ed6`.
- Private path/receipt `game-engine/data/diagnostics/rotation-fidelity-2026-10-21/world-fire-local-current-20261024T084253Z.db` / `.receipt.json`.

## Scoped GREEN, overlap still open

Revision `d8a30fa61a132c6303afa490d25b903d44acd743`: Firestarter and Scorch recipient boundary/explicit-second-target cases passed; hardcast→one pending projectile→impact zero plus caster isolation passed; startup inode passed. Compile8m21s, zero warnings; cases22.51s. Overlap alone failed native Pyro admission with OnGlobalCooldown: cast completion did not imply GCD completion. No count failure was observed. Test repair waits the actual native cast window and verifies one pending event before the earned-Hot-Streak Pyro.

Log `game-engine/data/diagnostics/rotation-fidelity-2026-10-21/fire-local-d8a-current-green.log`; fresh current backup20261024T090919Z, canonical/private SHA256 unchanged from RED's logical source but a separate new backup/receipt.

## Completed overlap proof —2936366fe

The corrected overlap case and startup inode case pass2/2 in9.76s; compile15.98s, zero warnings. Ordinary nearby Firestarter Fireballs earn actual Hot Streak; after a distant Fireball launches, the caller waits its real GCD, observes one pending event, then the ordinary instant Pyro consumes the earned proc and raises the count to two. No cast bypass, injected pending event, grant or synthetic timer.

Log `game-engine/data/diagnostics/rotation-fidelity-2026-10-21/fire-local-293-overlap-green.log`; separate fresh current backup/receipt20261024T092952Z, same canonical/private hashes. The three unchanged native cases retain their d8a proof; test-helper-only changes do not invalidate it. Combined four Fire-local cases plus startup qualification are GREEN, not a claim that the failing d8a invocation passed as a whole. `rustfmt --check` also passed on293's changed Rust files; manual readability audit found no required changes.

Caller300s ST/3T, generic facts and dependency qualification remain open; no full literal or DPS acceptance claimed.
