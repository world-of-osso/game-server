# Shared expected_combat_length fact

## Contract / scope

Main/14779 requested one shared NativeFacts getter: Fixture returns existing configured Encounter::Fixture.duration only when positive+finite; Dungeon/invalid configuration returns EXPECTED_COMBAT_LENGTH_UNAVAILABLE. No300literal, default, elapsed/remaining substitute or duplicate stored field. Ordinary/proposed action access must agree. Source: pinned76f8118 sim.cpp3579–3580 reads expected_iteration_time; parity caller derives configured duration from TICKS/20, not the getter. This slice is independent of persistent snapshot capture and the later Arms adapter.

Branch arms-expected-combat-length starts from target API66a991847 (master d675bf8fb merged, checked code dc525f641). Persistent branch2bc981067 retains seven genuine native REDs; no capture implementation here. FIRST/target integration remains lead-owned.

## RED / GREEN ledger

88095009c: actual4genuineRED0.02s after15m38compile/0warnings, all fail on the missing expected_combat_length operand/named contract after elapsed/remaining setup passes. `expected-length-red.{log,proof.json}` and `expected-length-red-binary.proof.json` pin exact source/binary; no DB/profile input.

Merged landed924638aba/crate-split C asfdd191725 before implementation. NativeFacts and the four test bodies are byte-identical across that merge, so RED scope remains valid; no redundant RED rerun. Minimal getter reads existing enum duration, validates positive/finite and returns named errors, without a new field or default. GREEN pending.

Four test-first cases use NativeFacts with a minimal Bevy World, an actual configured Encounter and advancing native Time. They require no private DB/profile/native cast; getter validation is their exact boundary. No build/setup/zero-test failure may count as RED.

| Case | Concrete input / expected output | Proof |
|---|---|---|
| Configured not elapsed/remaining |147.5/300/600.25 with native time83/start11, elapsed72; expected length stays configured | Genuine missing-operand RED |
| Invalid fixture |0,-1,NaN,+Inf,-Inf; named error, no default | Genuine missing-contract RED |
| Dungeon |configured start only; named unavailable, no inferred duration | Genuine missing-contract RED |
| Action forwarding |Fireball proposed context returns configured215.75 without an actor-wide default or spell lookup | Genuine missing-operand RED |

Getter implemented only after runtime RED. Commit/format precedes targeted GREEN; one admitted job/-j2. No DPS run or broad-suite rerun. Main owns final integration.
