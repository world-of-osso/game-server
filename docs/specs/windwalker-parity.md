# Windwalker native DPS parity

Spec269 rotation spells and selected talents execute through native admission, resources, effects and damage. TC a352b1fa and native DB2 numeric data take precedence over SimC76f8118; record divergences rather than tuning coefficients.

## Required behavior

- Interpret the pinned MID2 Windwalker literal APL using action-bound facts, actual native spellbook/castability and explicit unknown-operand failures ([shared contract](simc-apl.md)).
- `WW_COMBO_STRIKES_SIMC_POLICY`: first eligible ordinary action activates mastery; repeating the last eligible action disables it. Utility, failed casts and triggered children do not advance ordinary combo order. Hit Combo is not required. Use native115636 effect0 coefficient times raw mastery points, only on source-eligible damage.
- Measure300s against native dummy31146 and the3target fixture with matched pinned gear/talents. Reference ST206443.72 and3T376559.68; each native result must be within±10%.
- Produce one native Stockade simulation receipt with the WW kit.

## Qualification and exclusions

[Source census and current capability matrix](../handoffs/ww-source-census.md) owns per-ability proof and remaining gaps. `spell_cast/windwalker_mastery_tests.rs` tests actual TP→Provoke→TP without learned Hit Combo; genuine RED precedes implementation. This one boundary does not qualify all eligibility, channel snapshot or proc behavior.

No canonical world.db changes, fabricated IDs/coefficients, heuristic APL fallback or numerical tuning. Generic white-hit AP/6 versus TC3.5 is explicitly deferred by the lead ([weapon AP ledger](../handoffs/ww-weapon-ap-proof-ledger.md)).
