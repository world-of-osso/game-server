# Retained battleground bracket maximum at invitation acceptance

CPU-only; no live/deployment claim. Server branch `pvp-bracket-port`, base `748a5ad78`, protocol `a2b07a6`; merged server master `fdd6ee34c`. Production commits `a77ed7247` and field correction `2b8959fbb`.

## Contract and source

[Specification](../specs/pvp-progression.md#ordinary-battleground-party-queue). TC a352b1fa `BattleGroundHandler.cpp359–365` converts a non-arena accept above `bg->GetMaxLevel()` into a decline. `Battleground.cpp1467–1470` retains the PVPDifficulty entry;1528–1533 returns its maximum rather than the broad template maximum.

`pvp_invites.rs` reads the current character Level and `matched.instance.bracket.max_level`, after validating the owned invitation ID. Above-maximum non-arena acceptance uses the existing decline/removal path. No new error/wire, scaling, minima, reward or penalty policy. Existing late-Deserter refusal remains intact.

Cached sources, hashes, exact commands and raw logs:
`game-engine/data/diagnostics/pvp-bracket-port-2026-10-18/` (`source-receipt.json`, `proof-ledger.json`).

## Evidence

| Revision | Invocation / result | Proof boundary |
|---|---|---|
| `cf5978d3a` | First RED attempt:4 harness panics | ExperiencePlugin registered after App finish; NOT behavioral RED. Preserved `red.*.log`. |
| `efcc8dc89` | Corrected RED:2 FAIL,2 PASS | Concrete1018 and Random32 emitted NewWorld after real XP29→30 following20–29 invitation. Within28→29 and arena29→30 controls already passed. `red2.*.log`. |
| `020ce169f` | First GREEN attempt: compile error | Incorrect `BgInstance.template` field; NOT GREEN. Preserved `green.*.log`; corrected to retained `BgInstance.bracket`. |
| `2b8959fbb` | Combined GREEN:13 PASS,0 FAIL/ignored | New4 cases plus retained9 Deserter admission/port cases. `green2.*.log`. |
| `2b8959fbb` | Workspace `cargo fmt --all -- --check`:0 | Main-thread focused format proof. Lead owns final check/fmt gate. |

Real level-up means queued `XpAward::quest` using authored `xp_to_next_level`, applied by the actual ExperiencePlugin after invitation. Tests assert resulting Level/UnitLevel, XP remainder, unchanged pending match before accept, no NewWorld on refusal, origin preservation, invitation removal and actual InstanceManager reservation release. Nineteen accepted support players remain in the same InProgress copy. Initial levels are fixture setup before queue admission, not a post-invite Level mutation.

Within-bracket28→29 still transfers/acks to the retained copy. Arena29→30 still emits its native NewWorld. These are CPU channel/map tests, not live transport/UI proof.

## Exact filters

RED command used `bg_bracket_port_ -- --include-ignored --nocapture` after `--test -p server --bin game-server`; ignored cases were pushed before production changes.

Combined GREEN filter: `maps::tests::battleground_nodes::deserter::` (13 cases, including4 `bg_bracket_port_` cases). Execute via `agent-run pvp`, the server `scripts/build-host.py --root <slot> --test -p server --bin game-server`, and the assigned sibling protocol/environment overrides. Do not infer a full server-suite result. Independent Lead gate/landing pending.
