# SmartAI census — verified 2026-10-11

55,241 TDB rows. Admissions are supported event/action/target kinds, not full quest or per-row execution coverage. Source: read-only `data/world.db`, pinned TC a352b1fa; runtime `smart_ai.rs`.

| Dimension | Admitted kinds | Admitted rows | Unsupported rows |
|---|---:|---:|---:|
| Action | 24 | 41,696 | 13,545 |
| Event | 19 | 41,014 | 14,227 |
| Target | 8 | 50,628 | 4,613 |

## This continuation

- Actions66 SET_ORIENTATION and17 SET_EMOTE_STATE: 1,998 rows /1,116 quest-linked rows from the original handoff definition (740+376). Authentic-row RED/GREEN2/2. Target-specific eligibility still matters.
- Events17 SUMMONED_UNIT,35 SUMMON_DESPAWNED and54 JUST_SUMMONED: real child initialization/parent notification and delayed expiry with entry filters; cooldown RED/GREEN. Event7 EVADE now runs its already-present interpreter call.
- Escort point/wander, point/chase and post-combat return each have a fixed-schedule authentic Wizzle regression. Follow position/arrival/cancel and eight temporary lifetime modes remain in targeted behavioral coverage.
- Timed-list chance and20..40ms bounds are observed flag actions; already GREEN when first added, not invented RED fixes.

## Highest remaining action gaps

| Rank among unsupported | Type | Rows |
|---:|---:|---:|
| 1 | 45 | 2,056 |
| 2 | 5 | 1,321 |
| 3 | 25 | 958 |
| 4 | 85 | 732 |
| 5 | 72 | 659 |

## Live proof boundary

Modern and Forever completed legacy quest863 through normal GUI prerequisite858 reward, acceptance, all three paths, authored group0 summons, scripted completion, reward choice and Sputtervalve turn-in. Twelve inspected1280px PNGs: `/syncthing/AgentShared/2026-10-11/smartai-escort/`.

**Controlled authentic legacy fixture, not mixed-Retail certification:** private UDP5780 database copied exact AzerothCore combat rows for five enemy templates3282..3286. The unchanged mixed-Retail copy scales enemies to30/4,379HP while the cart remains18/417HP; instrumented deaths are genuine combat, not necessarily escort leash. No canonical database mutation, removed mobs, immunity, fake progress or admin quest completion. Source/before/after: engine diagnostics `smartai-coverage-2026-10-11/live/legacy-escort-fixture-provenance.json`.

Personal/spell-property summons, summon-death82/on-despawn86 hooks, unsupported targets and remaining timed/events/actions stay explicitly unimplemented. Counted admissions do not certify these branches.

## Proof

- Diagnostics: canonical engine `data/diagnostics/smartai-coverage-2026-10-11/`; `red/green-wizzle-*`, `red/green-summon-hook-*`, `red/green-ranked-orientation-emote-*`, `green-timed-chance-max-delay-compiled.log`.
- [Contract](specs/smartai-coverage.md), [escort fixture/setup](specs/smartai-escort.md), [runtime explanation](wiki/systems/boss-scripts.md#smartai-escort-coverage-2026-10-11).
