# Windwalker native DPS parity

Spec269 rotation spells and selected talents execute through native admission, resources, effects and damage. TC a352b1fa and native DB2 numeric data take precedence over SimC76f8118; record divergences rather than tuning coefficients.

## Required behavior

- Interpret the pinned MID2 Windwalker literal APL using action-bound facts, actual native spellbook/castability and explicit unknown-operand failures ([shared contract](simc-apl.md)).
- `WW_COMBO_STRIKES_SIMC_POLICY`: first eligible ordinary action activates mastery; repeating the last eligible action disables it. Utility, failed casts and triggered children do not advance ordinary combo order. Hit Combo is not required. Use native115636 effect0 coefficient times raw mastery points, only on source-eligible damage.
- `WW_SUNFIRE_SPIRAL_SIMC_POLICY`: selected native1272415 effect0 empowers Rising Sun Kick damage185099 only while ordinary Combo Strikes is active. Repeating RSK disables that benefit with mastery; no coefficient or class-mask invention. PinnedTC/TDB supply no Sunfire binding/mask; SimC76f8118 `sc_monk.cpp945–953` supplies the condition, DB2 supplies the percent.
- `MONK_MYSTIC_TOUCH_SIMC_PRODUCER` (native learned TP/white2RED, unlearned/unrelated2GREEN atd49; producer GREEN pending): the actual damage actor must know native8647 before its landed Monk impact/white hit produces native113746. Native aura metadata owns the physical5% target modifier; no class-only/nearby-Monk fallback or flat damage adjustment. Qualify first-impact/later-hit ordering and unrelated damage separately ([proof scope](../handoffs/ww-mystic-touch-proof-ledger.md)).
- Measure300s against native dummy31146 and the3target fixture with matched pinned gear/talents. Reference ST206443.72 and3T376559.68; each native result must be within±10%.
- Produce one native Stockade simulation receipt with the WW kit.

## Literal fixture driver boundary

Only explicit parity `Encounter::Fixture` activates the literal WW driver; it owns that actor's decisions without changing native follower identity, admission or combat handlers. Full pinned action lists and options remain literal, with shared native ActionFacts plus WW-specific combo/equipment/configuration facts. Unknown predicates/options are explicit errors or cited execution gaps, never a legacy-priority fallback. Unselected spells cannot leak from old generic kit rows. The WW fact adapter supplies actual fixture entity IDs in caller order to target_if; selected identities flow unchanged into the shared target-aware Prepared/Execution seam, never a first-target substitute. The fixed-time fixture's TTD remains its one global clock for every current target. Actual30s ST/3T cast/recipient tests and proposed-action combo tests precede implementation.

Stockade keeps its separately requested native kit driver. Loading a literal WW driver for a dungeon must reject `DUNGEON_TTD_UNAVAILABLE` before capturing its controller. A later health-trend TTD source packet is requested only after WW parity; live estimator implementation is not authorized.

## Qualification and exclusions

[Source census and current capability matrix](../handoffs/ww-source-census.md) owns per-ability proof and remaining gaps. `spell_cast/windwalker_mastery_tests.rs` tests actual TP→Provoke→TP without learned Hit Combo; genuine RED precedes implementation. This one boundary does not qualify all eligibility, channel snapshot or proc behavior.

No canonical world.db changes, fabricated IDs/coefficients, heuristic APL fallback or numerical tuning. Generic white-hit AP/6 versus TC3.5 is explicitly deferred by the lead ([weapon AP ledger](../handoffs/ww-weapon-ap-proof-ledger.md)).
