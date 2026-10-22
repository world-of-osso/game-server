# Binding Heals proof ledger — 2026-10-22

## Scope

Native368275→368276 transfer and enabling the already-fixed Prayer of Healing
cast-lifetime regression. No Holy decision-policy changes, mastery amounts,
canonical DB writes, selected-kit changes, or simulator effectiveness claim.

Source: TC a352b1fa `spell_priest.cpp:859–889` and
`sql/old/11.x/world/25101_2026_01_14/2025_12_30_01_world.sql`; local70448 aura/child
metadata. The SQL heal-only priest-family6/mask0x1800 restriction matters: the
native aura's effect class mask is zero. Preserve native amount, proc chance,
helpful-spell mask and ordinary child healing.

## Inputs

Server implementation a1f941b13; merged/final code
167612f2763e91dc55b155e6fb8aacf6850b7c27. Protocol
2abc7fa844f6e71d93203ce5421ae853e884854d. Test fixture uses the frozen existing
world DB SHA256
`6b8b5e96b2a1f0caac05df74abce559516cacae9279d85e669d97edfd0dd7f2c`.
Evidence directory: game-engine
`data/diagnostics/healer-ai-2026-10-21/binding-heals/`.
Six critical source hashes match S/T/helper-native root; native source paths alias
the checkout, not an independently frozen source copy (`post-master-inputs.json`).

## Commands and results

All compiles use capacity admission and `agent-run healer-ai`, then the approved
`python3 scripts/build-host.py --root T` helper. Native tests use
`--test -p server --bin game-server --profile test -- --nocapture --test-threads=1`
with the named libtest filters; checks use `--check -p server`.

| Revision / command scope | Result | Evidence |
|---|---|---|
| 16dc69af8/P03ceb compile | Environment failure only: stale protocol lacked landed empower/portrait APIs; not RED | `stale-protocol-compile-failure.log` |
| 16dc69af8/P2abc, `holy_binding_heals_native90` | Genuine missing-child RED:2PASS/1FAIL23.23s after native Flash Heal12778/overheal4958; all positive prerequisites passed | `native-red.log` |
| a1f941b13, Binding5 + enabled exact PoH1 | 6PASS34.76s; transfer/20% amount/one-child and self/periodic/family/unlearned controls | `native-green-attempt2.log` |
| 167612f27, post-master Binding5 + PoH1 + four shared-proc controls | 10PASS37.16s; positive additionally asserts Henry health recovery; compile3m52s | `post-master-native.log` |
| 167612f27, helper default server check | EXIT0,1m15s, no warnings emitted | `post-master-check.log` |
| 167612f27, `cargo fmt --all --check` | EXIT0 | `format-check.log` |
| Feature delta, manual main-thread readability | No in-scope changes required; not independent verification | `readability-review.md` |

The four retained native controls are Charred Warblades accumulation, Prayer of
Mending damage/heal/jump, Cenarion Ward ignoring healing/absorbed damage, and
Surge's actual proc/instant half-cost consumption. No unchanged broad suite rerun.
One earlier GREEN invocation failed Cargo argument parsing before compiling;
`native-green-attempt1.log` is retained, not treated as behavioral evidence.

## Capability boundary

Binding's ordinary native transfer is qualified; no combat-efficacy inference.
The shared healing payload preserves one hit event per planned recipient and
pre-overheal totals. Echo77485→77489 remains unimplemented pending the separate
shared follower-mastery bridge; do not fabricate a positive Henry mastery amount.
Native navigation, full selected Holy kit, and live pixel acceptance are separate.
