# Elemental Ascendance additional overload

## Revision and authority

- Base master: `bd5f3e00033d27d6a8e2a2b09b456b10d13782a9`.
- Tests: `7a6e14625bcc8021587c5f0ee44aa58be2cf1cda`; landed-GCD fixture correction: `b4a7f46b0b213fb8812a971eabf3325f2291e5b6`.
- Production: `a001f0541a6960489b721a54a8315d344f3b133e` (pushed `elemental-asc-overload`).
- Native70448 `1219480s9/effect8=1` supplies the additional count. TC a352b1fa `spell_shaman.cpp2124–2230` supplies ordinary mastery/SK/HIT+400ms. SimC76f8118 `sc_shaman.cpp3318–3348` corroborates once-per-cast Chain policy, not coefficients.

[Contract](../specs/spell-dummy-scripts.md#elemental-ascendance-additional-overload); [implementation](../wiki/systems/spell-casting.md#elemental-ascendance-additional-overload--2026-10-23).

## Concrete coverage

| Statement | RED b4a7f46b0 | GREEN a001f0541 | Proof level |
|---|---|---|---|
| Real SK-only188196 causes one45284 | PASS1 | PASS1 | Actual native cast, hit, delayed child damage |
| Real ASC+SK188196 causes ordinary+additional45284 | FAIL1≠2 | PASS2 | Actual native aura effect8=1, hit, ≥400ms child delay and damage |
| Real ASC+SK188443 against3 landed targets causes3 ordinary+1 additional45297 casts | FAIL3≠4 | PASS4 | Actual native3T casts/hits/delay/damage; native root speed0 |
| Lava Burst, Elemental Blast, Icefury additional overload | Not run | Not run | Shared production mapping implemented; no per-root native qualification |
| Failed ordinary roll, aura expiry/foreign aura, missed primary, DPS parity | Not run | Not run | Explicit remaining gaps; no claim |

## Commands and immutable inputs

Both revisions used `agent-run elemental-parity-shuja cargo test --locked --no-run -j 2 -p server --lib elemental_ascendance_additional_`, then the exact emitted ELF via `python3 scripts/dps_parity_native.py <ELF> elemental_ascendance_additional_ --ignored --nocapture --test-threads=1`. Parent extended capacity priority `lead_ele_13400`; one owned job only. Private read-only source view, native262 profile/active traits and the same immutable canonical-world backup were retained across RED/GREEN. No artificial chances, buff amounts, stats or coefficient edits.

Evidence under game-engine `data/diagnostics/elemental-parity-262-2026-10-23/`:

- `asc-additional-red-b4a7f46b0/`: compile21m14/zero warnings; actual1PASS2FAIL27.40s. ELF SHA256 `5d961fc820189b39a0cb4d0d8f7152f9fbcb4a8b26d82882ee7f33533e92414c`.
- `asc-additional-green-a001f0541/`: compile17m03/zero warnings; actual3PASS20.23s. ELF SHA256 `12153369bd51db038c992335d4bd2cf10d033fd4e0546f569c98509d3433a21f`.
- Each directory preserves input hashes, stable source/dependency revision provenance, command/status logs and native proof JSON. Earlier7a6 compile-onlyE0599 was not feature RED; no native runs occurred there.

## Split-B integration

Parent-confirmed `origin/master@d675bf8fb` merged at `c32f187ca85786a88809e1e3a25523380807b0dc`. Resolve the directory-move conflict by retaining `elemental_ascendance_tests.rs` under `crates/server-simulation/src/dungeon_simulator/dps_parity/`; its existing `crate::` paths use the new simulation crate's authoritative server re-exports. Runtime overload code remains in `server`. No pending equipment/target/precombat API merged; parked adapter unchanged.

Requested integration check: `cargo check --locked -j 2 -p server -p server-simulation --tests` (pending). Native acquisition tests now belong to `-p server-simulation --lib`; retained child-effectiveness tests stay in `-p server --lib`. Above native results remain exact pre-split a001f0541 evidence, not post-split execution claims. Parent owns final functional gate; do not rebuild/replay unchanged native proofs merely for this merge.

## Retained child-effectiveness controls

The shared aura-reader refactor also touches existing mastery damage scaling. On the retained a001f0541 ELF (same SHA before/after; cb82a5ffe changes docs only), run `overload_uses_mastery_effect1_and_removal_restores_child_base --nocapture --test-threads=1` through the same private source view: all5PASS16.88s. Real LB/CL/Lava/EB/Icefury child casts retain native effect1 effectiveness, aura removal restores child base, and Echo Chamber applies through ordinary aura calculation. These existing controlled-SP/zero-crit harness cases qualify effectiveness/removal, not additional-overload acquisition or DPS. No rebuild. Logs/proof: `asc-child-controls-a001f0541/` beside the RED/GREEN artifacts.

Implementation changes only native extra-count scheduling and the first harmful target-plan HIT marker. Ordinary proc RNG consumption, Chain chance division and queued400ms child damage/energize remain. Parent owns final checks/readability/integration. Docs-only follow-ups do not invalidate runtime proof. Full literal adapter remains separately parked at `elemental-parity-shuja@b4a269fd97f6ffad5783ed2d8f38edd1520e9fe3`; do not remove it or merge pending APIs.
