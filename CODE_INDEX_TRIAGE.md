# Code-Index Triage

Refreshed on 2026-04-08 from the repository root with:

- `code-index index`
- `code-index status`
- `code-index dead-code`
- `code-index untested`

## Summary

- Indexed files: `140`
- Symbols: `4432`
- References: `19594`
- Resolution health: `2131 / 17307` resolved, `3493` ambiguous, `11683` unresolved
- Raw `untested` output: `608` symbols
- Raw `dead-code` output: `2421` symbols

The `untested` report is usable for shortlist triage.

The `dead-code` report is not reliable enough for deletion work yet. The repo currently produces too many false positives because `code-index` is missing test and caller relationships across a large part of the Rust graph.

## Evidence That Dead-Code Counts Are Inflated

These symbols are reported as dead or untested even though the repo clearly references them:

- `shared::threat::should_evade`
  - Declared in [threat.rs](/syncthing/Sync/Projects/world-of-osso/game-server/crates/shared/src/threat.rs)
  - Called repeatedly by inline unit tests in the same file
  - `code-index callers should_evade` and `code-index tested-by should_evade` both return `[]`
- `shared::formulas::melee::resolve_melee_outcome`
  - Declared in [melee.rs](/syncthing/Sync/Projects/world-of-osso/game-server/crates/shared/src/formulas/melee.rs)
  - Called from [combat_tests.rs](/syncthing/Sync/Projects/world-of-osso/game-server/crates/shared/src/formulas/combat_tests.rs) and inline tests in [melee.rs](/syncthing/Sync/Projects/world-of-osso/game-server/crates/shared/src/formulas/melee.rs)
  - `code-index callers resolve_melee_outcome` and `code-index tested-by resolve_melee_outcome` both return `[]`
- `server::spell_db` shows up as a large dead-code cluster, but the entries sampled there are inline test functions, not production paths

This is enough to treat the current dead-code total as structurally noisy, not as a safe deletion queue.

## High-Value Untested Hotspots

These are the production modules most worth manual follow-up from the current `untested` output:

1. [duel.rs](/syncthing/Sync/Projects/world-of-osso/game-server/crates/server/src/duel.rs) — `23`
2. [pvp.rs](/syncthing/Sync/Projects/world-of-osso/game-server/crates/server/src/pvp.rs) — `19`
3. [lfg.rs](/syncthing/Sync/Projects/world-of-osso/game-server/crates/server/src/lfg.rs) — `18`
4. [auth_character.rs](/syncthing/Sync/Projects/world-of-osso/game-server/crates/server/src/auth_character.rs) — `18`
5. [admin_handlers.rs](/syncthing/Sync/Projects/world-of-osso/game-server/crates/server/src/admin_handlers.rs) — `18`
6. [friends.rs](/syncthing/Sync/Projects/world-of-osso/game-server/crates/server/src/friends.rs) — `17`
7. [handlers.rs](/syncthing/Sync/Projects/world-of-osso/game-server/crates/server/src/auction_house/handlers.rs) — `17`
8. [ignore_list.rs](/syncthing/Sync/Projects/world-of-osso/game-server/crates/server/src/ignore_list.rs) — `13`
9. [persistence.rs](/syncthing/Sync/Projects/world-of-osso/game-server/crates/server/src/persistence.rs) — `12`
10. [auth.rs](/syncthing/Sync/Projects/world-of-osso/game-server/crates/server/src/auth.rs) — `12`

These look like better next targets than broad dead-code deletion because they are runtime server workflows where missing coverage is plausible.

## Dead-Code Shortlist Worth Manual Review

These surfaced as public APIs with weaker evidence of obvious false positives than the test-heavy shared math and threat modules:

- [holiday.rs](/syncthing/Sync/Projects/world-of-osso/game-server/crates/shared/src/holiday.rs)
  - seasonal constructor helpers like `lunar_festival`, `love_is_in_the_air`, `noblegarden`
- [item_data.rs](/syncthing/Sync/Projects/world-of-osso/game-server/crates/shared/src/item_data.rs)
  - convenience helpers like `color_name`, `is_bound_to`, `dps`, `is_two_handed`, `is_broken`
- [messaging.rs](/syncthing/Sync/Projects/world-of-osso/game-server/crates/server/src/messaging.rs)
  - chat and guild message processors
- [profession.rs](/syncthing/Sync/Projects/world-of-osso/game-server/crates/shared/src/profession.rs)
  - accessor helpers like `required_level`, `required_skill`, `has_spec`, `spec_for_profession`
- [movement.rs](/syncthing/Sync/Projects/world-of-osso/game-server/crates/shared/src/movement.rs)
  - environment and movement utility helpers

These should still be reviewed manually before any deletion. The current index quality is too weak to delete directly from the report.

## Recommendation

1. Use the `untested` report for the next server-focused test pass.
2. Do not use the current `dead-code` report as a deletion list.
3. Take the existing `Improve code-index resolution quality for this repo` item before any broad dead-code cleanup.
4. Re-run dead-code triage only after resolution quality materially improves.
