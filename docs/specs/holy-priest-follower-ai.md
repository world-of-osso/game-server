# Holy Priest follower decisions

## Scope and authority

Henry Garrick (215011), class5, spec257, uses the native selected starter
spellbook and talents at his actual level. Spell/effect/cost/radius/proc values
come only from the local 12.1.5.70448 catalog. Decisions are named project policy
`HOLY_PRIEST_TRIAGE_V1`, not SimulationCraft or a Retail healer rotation.
Generic patient eligibility, damage history and triage remain shared healer policy.
No unlearned spells, synthetic procs, fabricated auras or alternate catalogs.

Source receipt: game-engine `data/diagnostics/holy-priest-2026-10-21/kit-source.json`
contains the88 published rows, the90 native starter-loadout snapshots, the Priest
talent tree and per-spell SHA256 hashes of the referenced local catalog rows.
Publication source is server master c6be257c5; protocol03cebdb.

## Source corrections to historical expectations

- Surge of Light109186 has native8% chance. Its114255 buff grants at most2 uses,
  instant Flash Heal and **50% less mana for Holy**, not a free heal. Stack count
  represents uses, not doubled discount. Successful launch spends one use;
  cancelled/failed casts do not. No fabricated100% proc qualification.
- Heal2060 and Circle of Healing204883 exist as spell records but are absent from
  Henry's selected70448 spellbook. Do not grant/schedule them or assume Heal
  reduces Serenity: current63733 explicitly names Flash Heal only.
- Sanctify34861 is selected at levels15–68. Ultimate Serenity1246517 replaces it
  at69; its native tooltip redirects Sanctify talents to Serenity. Old Sanctify
  scheduling must not reintroduce the removed spell.
- Dispel Magic528 is not selected. Purify527 is learned10; Mass Dispel32375 is
  learned50. Do not invent the missing hostile single-target dispel.
- Apotheosis200183 is learned35. Native values are200% increased Holy Word
  reduction and70% less Holy Word mana, not historical free Holy Words.

## Decision contract

1. Preserve native admission and triage exclusions: living same-copy party,
   actual known spell, range/LOS, mana, cooldown/charges, crowd control and
   ordinary aura eligibility. Emergency healing takes precedence over upkeep,
   throughput cooldowns and damage.
2. Guardian Spirit protects an imminent-death patient; Serenity supplies urgent
   direct healing. Desperate Prayer protects an injured Henry. Fade is a personal
   threat/defensive action, not an enemy damage filler. Never treat an aura's
   successful admission as proof its missing scripted consequence exists.
3. Spend Surge on an injured reachable patient before an ordinary paid Flash
   Heal when no higher emergency applies. Maintain the actual owned PoM ward
   **41635**, including its bounced holder, rather than testing root33076.
   Do not overwrite a live owned bounce solely because its holder is healthy.
4. Use Sanctify only on a real injured cluster inside its native destination
   radius. Prayer of Healing uses its native primary-centered footprint. Divine
   Hymn requires sustained group injury and a safe channel opportunity; Apotheosis
   requires group pressure plus depleted Holy Words. Avoid spending these merely
   because combat exists or before immediate triage needs are met.
5. Fortitude casts once when any party recipient lacks it, relying on native
   target118 raid/party expansion; never replace that mechanic with per-member
   casts. Party coverage must be demonstrated through that native group expansion. Purify prioritizes actual
   eligible harmful Magic/Disease auras. Mass Dispel requires an applicable native
   dispel footprint; Leap of Faith requires a demonstrated hazardous patient and
   a safe Henry endpoint. No random displacement, fake hazard or empty dispel.
   Holy Fire, Smite and Holy Nova weave only when shared triage reports healing
   safe; native engaged-enemy AoE/CC guards remain in force.

Thresholds, scoring and pressure windows are project policy and must be documented
with the implementing tests. No claim that a threshold is sourced from DB2.

### V1 project thresholds and ownership

Guardian Spirit protects patients below20% health or within the shared3sec
projected-death window; Desperate Prayer uses Henry's
below50% health plus observed incoming damage. Shared urgent direct-heal candidates
precede routine throughput/upkeep. Fade requires an actual engaged enemy currently
attacking Henry, and does not interrupt an existing friendly cast. Mass Dispel
requires three actually dispellable harmful Magic patients inside one native
friendly dispel radius; global aura count or an empty footprint is insufficient.
Purify follows those emergency/group-dispel decisions and precedes routine healing,
using its selected native harmful Magic/Disease eligibility; no empty dispel.
Surge selects below90%, PoM below95%, and group
pressure requires three actual patients:65% for depleted-Word Apotheosis,55% for
Hymn. Apotheosis precedes the channel when both qualify. A channel is refused for
shared emergency patients or an authored near-term hit on Henry. Sanctify needs
three injured members inside the shared native-radius cluster. Fortitude waits
for calm shared triage and casts once around Henry when any party buff is missing.
Prayer of Healing requires at least three actually injured recipients inside its
native primary-centered cluster. Sum their existing effective-healing estimates
and compare aggregate healing per native mana against available ordinary direct
heals; choose the best useful cluster before PoM upkeep, after Surge/Words and
throughput cooldown decisions. No invented coefficient or map-wide injury count.
Holy Nova uses its native self target (RangeIndex1), never an enemy pursuit for
this zero-range spell. Require shared healing-safe inputs, the selected kit's
engaged-enemy count in its native radius, and no living fresh enemy or protected
breakable CC in that footprint. Holy owns the decision; no generic movement change.
These are project decisions, not DB2 magnitudes. Owned cooldown/upkeep rows stay
owned even when no native admission is available; ordinary direct healing remains
available through the shared generic candidate ranking.

## Consequence coverage required

| Feature | Native consequence / proof obligation | Initial state |
|---|---|---|
| Flash Heal → Serenity | native2050 effect1 reduction6sec | Existing native tests |
| PoH → Sanctify | native34861 effect2 reduction6sec | Existing native tests |
| Surge | natural8% acquisition, instant/half-cost, one use, cap2 | Native Henry4GREEN66f0; missing-mana talent remains a named gap |
| Prayer of Mending | real41635 damage heal33110 and delayed155793 bounce | Existing native tests; upkeep decision missing |
| Voice of Harmony | learned390994 PoM → Serenity reduction4sec | Native cast qualification GREEN66f0, including unlearned control |
| Apotheosis | restore native Holy Words;200% enhanced reduction;70% mana discount | Reset/amplification GREEN66f0; group timing GREEN4b0; native nonfree70% Word discount GREEN775 |
| Ultimate Serenity | native replacement and real nearby healing1250581 | PoH→Serenity CDR GREEN66f0; nearby healing remains unqualified |
| Guardian Spirit | actual death prevention, one-shot rescue and expiry | HP/forecast policy GREEN775; nativeNPC one-shot rescue and later death GREENd71; earlier player expiry tests retained |
| Divine Hymn | actual channel/ticks/recipient selection and safe timing | Timing unqualified |
| Fortitude/Purify | party upkeep / native effect38 eligibility | Purify realMagic GREEN4b0; Fortitude regression enabled after common target118 fix, integration proof pending |
| Mass Dispel/Leap/Fade | actual applicability/safety/consequence | Generic Fade/Meredy103 suppression and expiry GREENf53; FadeAI/groupMagic REDd71→GREENeaa with negative footprint/victim controls. Leap77 blocked; special72734 potency unqualified |
| Damage weaving | no unsafe offense while injury demands healing | Shared framework dependency |
| Remaining selected passives | exact native modifiers or explicit named script gaps | See [named gaps](../wiki/investigations/holy-priest-follower-ai.md#named-selected-kit-gaps); not runtime-complete |

## Temporary threat

Native103 temporary threat modifiers apply to every unit already represented in
a creature's threat list, not only Player-tagged units. Fade on Henry and Mirror
Image55342's native103 on Meredy must reselect the victim while retaining raw
threat, then restore that reference when the real aura expires. Mirror Image's
separate guardian/DUMMY consequence is not certified by this control.

## Verification

Targeted tests use real catalog records, ordinary cast completion and naturally
acquired procs. Record exact RED and GREEN revisions/commands. Existing GREENs are
retained evidence, never relabelled as newly reproduced REDs.

Freeze a common baseline containing the current Holy mechanics, generic healer
framework and latest merged `origin/dungeon-simulator`, with Holy `decide` returning
`Defer`. The candidate differs only by Holy policy. Select Henry for both sides,
with identical protocol/catalog/route/level/seeds1–20 and optimized incremental
`sim` profile. Both initialize the same opt-in Holy observer and publish its
per-actor counters at `performance.holy_metrics` in final and progress reports.
This comparison measures policy, not the earlier Surge/Holy Word mechanic fixes. Report deaths, overheal percentage, sampled mana curve, actual Holy
Word cooldown reduction versus available reducible time, and actual proc grants
versus consumed uses. Separate unavailable mechanics from zero observed events;
no fabricated efficiency denominator. Keep before/after simulator revisions and
settings with the receipts. Stockade stays at its native default32, not a level90
party overleveling low-level enemies. Qualify the full90 kit separately on a
native90 party receiving actual70448 max-health damage (124257) and native harmful
Magic (23268, native72/3sec with no power cost;17146 requires95mana
and is invalid for a no-mana dummy); do not manufacture injury, mana or buffs. Repeat the20-seed cohort
on scaling follower difficulty205 when that simulator is available. Plane geometry
is not live navigation certification.
