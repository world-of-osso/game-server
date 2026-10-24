# Qualified Ret Sentence / Wake / Toll packet

## Scope and gate

Lead requested this isolated packet from master `d7e86cd0f679338ad32ae2b24b351d1d6454501b`. It contains only the qualified Sentence and Toll relationships, Wake qualification, their native tests and documentation. No literal adapter, Expurgation callback/tests, Rebuke readiness tests or Aura361 consumer tests are included. Those remain open on `ret-paladin-literal-development-d385`; exclusion from this packet does not narrow the full [Ret objective](ret-paladin-parity-proof-ledger.md#goal-and-authority).

Parent owns integration/final acceptance and remote operations. Requested packet integration command: `cargo check -p server -p server-simulation --tests -j2`, once through build-host/agent-run after admission. Ran exactly once on56da03776: exit0,2m04s, one inherited master warning for the now-unused simulator `navigation` import. Follow-up removes that alias only; no gameplay changes. Per lead's once-only instruction the compile command is not repeated after this deletion; parent owns final integration gate. Exact log/receipt: engine diagnostics `ret-qualified-packet-56da03776-check.{log,proof.json}`. Historical behavioral receipts below retain their exact tested revisions; the packet is not represented as a current full-suite pass.

## Source and behavior matrix

| Feature | Qualified behavior | RED → GREEN proof |
|---|---|---|
| Sentence343527 | Ordinary Judgment-funded cast, actual owned aura and real damage, native expiry emits387113; Cancelled removal never discharges | d15dae8f0:1genuineRED/1cancelPASS49.39s →9239a9092 unchanged2GREEN45.92s, zero warnings |
| Sentence percentage/ownership | Native effect1=20%; counter delta follows only actual owner's nonphysical damage; real white damage and a second normally spawned/equipped Ret actor's Holy Judgment are excluded | Test-only37f8763a2:2GREEN47.23s, zero warnings; runtime unchanged9239a9092 |
| Wake255937 | Ordinary selected cast deals actual31146 direct damage and grants native effect1 exactly3HolyPower; granted-only book does not admit purchased Wake | c17b9e565:2GREEN42.99s, zero warnings; no gameplay change |
| Toll375576 | Actual ST and each of3real combat-admitted31146 targets receives native406957; granted-only knowledge control remains rejected | f9622f8a1:2RED/1control48.15s; corrected native combat-ref setup b846564ea:2RED/1control69.13s →7361618a9 unchanged3GREEN56.82s, zero warnings |
| Toll cap/range/mode/percentage |7near combat candidates yield exactly5unique damaged targets including primary once; near idle/far combat targets excluded. Actual1277026 selects1279408 instead of406957; actual result-stage ratio equals native effect5 percentage | Test-onlyd0d6d07cf:2GREEN45.41s, zero warnings; runtime unchanged7361618a9 |

Eleven unique native cases. No300s DPS measurement, parity-band verdict or full natural-rotation claim. First Sentence implementation attempt d20ba037c failed compilation due to an owned-String error constructor; fixed by9239a9092, not runtime RED. Toll control attempt1475a6742 incorrectly expected full-mask children to emit CastSuccess; corrected to actual per-target Damage atd0d6d07cf without gameplay changes. That attempt is not a gameplay failure.

## Execution Sentence source policy

Lead approved `SIMC_RET_EXECUTION_SENTENCE_ACCUMULATION` on2026-10-24. Native343527 has no SpellAuraOptions row in any difficulty. Effective audited TDB has no343527/343721 row. TC a352b1fa `SpellMgr.cpp1758–1768` skips default generation without ProcFlags; no admission mask/chance is invented.

Pinned SimC76f8118 `sc_paladin.cpp3590–3612` installs owner/pet-list outgoing-damage assessors only for targets carrying that owner's gather; any Physical school bit, including hybrids, is excluded (`dbc/sc_const_data.cpp973–976`). `sc_paladin_retribution.cpp1048–1076` supplies accumulation. This packet implements direct-owner scope only. TC `spell_paladin.cpp739–804` owns native effect1 percentage and expiry-only387113 BP0 discharge through partial-trigger admission, not FULL_MASK.

[Class aura implementation](../../crates/server/src/auras/ret_sentence.rs) observes actual damage at `procs::push_event`, before queueing/same-tick expiry, changing no numeric/default proc semantics. Only matching caster/target/live aura instances accumulate. [Partial BP0 helper](../../crates/server/src/spell_cast/creature.rs) retains ordinary costs, cooldowns, aura/GCD checks and child proc behavior, ignoring in-progress only.

## Divine Toll source policy

Lead approved `SIMC_RET_DIVINE_TOLL_RELATIONSHIP` on2026-10-24. TC has no375576/DivineToll handler, native Dummy fields have Trigger0, and native/TDB contain no child dispatch edge. SimC76f8118 `sc_paladin.cpp1580–1590,1618–1631,1815–1935` and `sc_paladin_retribution.cpp1251–1253` supply native child406957 versus1279408 under actual1277026 mode.

[Relationship implementation](../../crates/server/src/spell_cast/scripts/ret_toll.rs) uses the existing generic area-filter/context-hit/triggered-child seams. Native effect0 cap, effect1 area geometry/combat admission and effect5 percentage remain authoritative; child coefficients stay native. The existing explicit primary counts once. No target/default-proc rewrite or fabricated resource producer.

## Proof artifacts and unqualified boundaries

Artifacts live under `/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/ret-parity-2026-10-23/`:

- Sentence: `ret-sentence-source-v1.json`, `ret-sentence-red-v1.log`, `ret-sentence-green-v2.{log,proof.json}`, `ret-sentence-native-controls-v1.{log,proof.json}`.
- Wake: `ret-wake-native-qualification-v1.log`.
- Toll: `ret-toll-source-v1.json`, `ret-toll-red-v{1,2}.log`, `ret-toll-green-v1.{log,proof.json}`, `ret-toll-native-controls-v{1,2}.{log,proof.json}`.

All are targeted qualification on the existing private snapshot, not DPS measurements. Snapshot SHA256 `52fa4d0ebec755f421b5678f0e83aa690a1d2422f33eae5208ff398848d4756c`; source/copy receipt is `hammer-policy-v1/database.proof.json`. No canonical database writes. TDB audit `/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/arms-parity-2026-10-23/tdb-spell-proc-source-audit.json`, SHA256 `4c7f2019aedc23baab7432fc3b639573ab854da23b433ecb393a9def77f28704`.

Sentence controlled-pet routing and a positive physical-hybrid fixture remain open. Wake hero/secondary-child relationships remain open. Toll mode production, child funding, Resonance, Exaction and Divine Hammer remain separate unqualified relationships. Shared proc metadata, Rebuke target-ready and Aura361 work remain with their assigned owners; this packet neither waives nor implements them.
