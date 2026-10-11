# Class spell batch13 — 2026-10-10

[Contract](specs/spell-dummy-scripts.md#thirteenth-batch-locally-authored-hit-rank-and-channel-contracts), [runtime](wiki/systems/spell-casting.md#thirteenth-batch-authored-hits-ranks-and-channel-clocks), [census](spell-coverage.md). Local Mainline12.1.0.69933 DB2, read-only world.db and TDB1210 only. No TrinityCore C++/external source, world.db writes, data/.uid commits, UDP5000 or master-destination merge.

## Capabilities / limits

| Path | Implemented | Still missing |
|---|---|---|
|12294 Mortal Strike|Named115804 healing reduction.|Full talent variants.|
|772 Rend|Named388539 native bleed.|Crit vulnerability/talent variants.|
|163201 Execute|316405-gated refund of actual paid raw Rage, including optional spend, after surviving instant260798 hit.|Damage interpolation/admission/talent variants.|
|32379 Shadow Word: Death|Tooltip s2 damage once, native execute threshold/bonus and active364675 extensions; one surviving-target backlash.|Legacy s1/rank variants.|
|257044 Rapid Fire|Native333ms/2000ms/initial tick => seven257045 shots and263585 gains; cancellation.|40-valued periodic mirror/other talents.|
|19574 Bestial Wrath|Actual persistent pet buff and344572 at its own target.|CC removal, charge ranks, periodic12 and native pet inheritance.|
|193530 Aspect of Wild|Initial free193455 at primary.|Additional-target geometry/ongoing cleave.|
|48181 Haunt|Owned mark's death resets caster cooldown.|Optional periodic talent and complete vulnerability/rotation parity.|

Guardian priority was investigated first. Local29264 template establishes1500ms swing time, not its owner-stat/melee damage curve.27829 binds51963, not non-demon inherited AP/SP. No local scaling table/description or archive script body establishes those missing formulas. Native spell/creature rows are retained in source-receipts.json; no guessed numbers or generic melee substitute.

## Proof ledger

83f3f70: initial nine fixtures RED0/9. Eight failures are missing/wrong behavior; Aspect used no explicit enemy and failed admission, not valid behavioral RED. ada5136 corrects native explicit target; its focused RED0/1 observes no Cobra. Implementation verification/census/all40 pending.

Durable evidence: `game-engine/data/diagnostics/spell-dummy13-2026-10-10/` (frozen before source, receipts, RED logs).
