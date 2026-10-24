# Native Rake actual-stealth eligibility

## Scope and source

Clean gate branch `feral-rake-stealth` starts at exact master1fb0230adaced22b170796e37271ea4e45a6ebfb. It carries only the Rake predicate, core regression assertion/name, relocated actual-profile case and focused documentation. No literal-development ancestry, Bite/form/proc-policy changes, passive removal observer, Shred predicate change or persistent multiplier fields.

TC a352b1fa `spell_druid.cpp2040–2078` records actual caster stealth and applies native1822 effect3's amount plus native163505 stun. It does not require legacy231052/405834 aura presence. The shared Rake predicate now checks actual stealth only; native coefficients, existing bleed snapshot path and ordinary nonstealth behavior stay unchanged.

The obsolete no-rank unit is renamed `dummy7_rake_stealth_without_legacy_rank_gets_authored_bonus` per explicit parent approval. Its test and predicate commit cite TC2040–2078.

## Existing behavior proof

Engine diagnostics root: `data/diagnostics/feral-parity-2026-10-23/`.

- An observer-only run established why the initial fixture was invalid: native1126 Mark of the Wild cast success interrupted5215 Prowl at1.55s with requested/matched flag0x4 and `RemoveReason::Interrupted`; AutoAttack/CombatStatus were absent. `rake-prowl-removal-observer/{proof.json,stderr.log,removal-receipt.json}` preserves the receipt. The engine's interrupt behavior is correct. No observer code is needed by this clean branch's acceptance case.
- Approved fixture ordering captures the existing fixture controller, applies real required Mark buffs before Prowl, waits actual native GCD and leaves stats/interrupt flags untouched.
- Test-onlyda45833b1 reaches genuine damage RED: ordinary Rake7077, actual Prowl Rake7077, authored factor1.6; both real casts land without either legacy aura. OneFAIL14.86s/clean48.81s build, `rake-selected-owned-precombat-red/`.
- Predicatea5b2fce9f on the development branch passed seven requested cases: four initial/rank/nonstealth14.68s, two existing dummy12 snapshot/removal/reapplication10.51s, one actual selected-profile12.46s. Both packages compiled36m09s/exit0/zero warnings; `rake-selected-green/` contains per-command receipts and compiler stderr. Zero filtered simulation cases in the core invocation are compile-only evidence, not runtime coverage.

## Clean-branch gate

The relocated test uses the same prepared native profile, controller capture, ordinary casts/GCD, target geometry and damage assertions; diagnostic-only removal recording is omitted. It has no dependency on a Feral literal adapter or new observer API.

At code1775772177f040f054f4c2edab5fa6b643d3c73f, evidence root `rake-clean-gate/`:

- [x] Relocated actual-profile case ran once:1PASS31.65s/exit0, `native-case.{proof.json,stdout.log}`;611.03s wrapper includes compilation.
- [x] Original requested two-crate check finished exit0/374.58s; it had already started before the package-scope expansion and is not represented as spell-test coverage.
- [x] Expanded check ran once with `-p server -p server-simulation -p server-spell-tests --tests`: exit0/2.27s wrapper, cached server/simulation metadata reused, `three-crate-check.proof.json`.
- [x] Four relocated `dummy7_rake_` cases ran once in `server-spell-tests`:4PASS16.03s/exit0, `relocated-core-cases.{proof.json,stdout.log}`.
- [ ] Parent owns push and full `--workspace` gate; no full-suite or300s DPS parity claim.

The native predicate is byte-identical to qualifieda5b2fce9f. Changes from exactbase1fb0230ad are Rake-only; no merge or literal-development ancestor enters this branch. Focused docs updates do not invalidate these code-scoped receipts. The historical spell-test migration mapping retains its original pre-rename inventory; this fix's explicit semantic rename is recorded above.
