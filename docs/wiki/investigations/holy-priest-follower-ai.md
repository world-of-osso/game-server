# Holy Priest follower AI

Henry Garrick215011 uses class5/spec257's selected70448 starter build, not a
historical healer rotation. This investigation tracks native consequences and
project triage decisions separately. Work remains in progress.

## Source boundary

The [complete selected inventory](../../specs/holy-priest-kit.md) contains88 rows
and every selected passive description, with level intervals. The
[decision contract](../../specs/holy-priest-follower-ai.md) owns the scheduling
requirements and source corrections; do not duplicate the inventory here.
Hashed source rows and90 native starter snapshots are in engine diagnostics
`holy-priest-2026-10-21/kit-source.json` (local70448).

## First native reproduction

At9f32cff77/protocol03cebdb, `holy_priest_native` produced2FAIL in14.66s:

- Actual Henry14 knows109186. After166 ordinary Flash Heal casts, no114255
  appeared. This finite bound has a1e-6 miss budget at the native8% chance; it
  does not prove a Retail proc frequency.
- Actual Henry35 spent Serenity, cast real Apotheosis, then could not cast
  Serenity again: the Holy Word charge remained depleted.

Receipts: engine diagnostics `holy-native-red.log` (stdout) and
`holy-native-red-runtime.jsonl` (saved original stderr/exit). Initial optimized
build31m48s; no repeat build was run to recover omitted stderr.

## Native core qualification at66f0b1d6b

Exact module runs: six pure policy cases, four Henry native cases, four native
Word cases and two opt-in observer cases GREEN. This is **not** a completed
native decision policy: the production specialization seam still defers while
its level90 group integration tests are being qualified.

The native two-use Surge case first reproduced free Flash from doubled stacking
at a79dd0e43, then passed after bc21c17ef: two uses were earned by28 ordinary
Smite casts; one Flash retained the50% cost and left one use. Native reset,
200% amplification, learned4sec PoM reduction, unlearned control and Ultimate
PoH redirection also passed. Counter tests distinguish ready Words' zero useful
reduction from actually removed seconds, and count real proc gains/consumption.
Receipts: `holy-{policy,native,words,metrics}-v2.log` in the same diagnostics root.

New group/threat fixtures failed before their behavior boundary: the small
`load_native_creature_world` helper intentionally supplies template/type metadata,
not `creature_classlevelstats`. Real summon construction therefore rejected the
missing native class/level rows. Correcting the fixture from the authoritative
server tables is required; these setup failures are not claimed as AI/threat REDs.

## Surge root cause

109186 has only a DUMMY4 effect, amount8 and TriggerSpell0. Its AuraOptions have
ProcChance100 and mask81920. Generic `handlers::can_proc` **includes DUMMY4**;
matching and chance can therefore succeed. But the generic dispatcher has no
PROC_TRIGGER_SPELL effect to cast and no registered Surge `on_proc` hook, so it
executes no buff-producing consequence. The tooltip supplies114255's identity.
This is not evidence that the generic proc event executor is blocked.

Existing Omen113043 demonstrates the same bounded pattern: native chance from a
DUMMY field plus a class-specific consequence on the ordinary proc path. Commit
00848e6cd adds Surge's matching/chance hook and114255 consequence, plus Apotheosis
charge restoration. The subsequent native qualifications above prove the bounded
consequence, not every selected talent or a Retail proc-frequency distribution.

## Iteration and comparison

Both owned checkouts use an uncommitted `[profile.sim.package.server]
opt-level=1` override authorized by the lead; dependency optimization remains3.
Before/after seed comparisons must use the same override. The detached simulator
checkout's target symlinks only to this agent's own Holy target; no target copy or
cross-agent target sharing. Capacity admission still gates every build/run.

## NPC temporary-threat reproduction

At e95989467, both authentic ordinary casts applied their real103 aura but failed
to change the native current victim: Henry586 and Meredy55342 kept the1000-threat
NPC reference instead of the500-threat Player reference. These are genuine REDs,
not the earlier construction failures (`holy-threat-fixture-v3.log`,2FAIL6.32s).
The shared `temporary_threat` reader returned zero for every non-Player. Removing
that restriction applies the existing native flat modifier to every listed unit;
raw threat and the existing expiry/reselection path remain untouched. Atf53b75c0b,
both native cases GREEN5.39sec (`holy-threat-v4.log`), including expiry and raw
threat preservation. Mirror Image's unimplemented guardian consequence is outside
this control.

The group setup then reached a second distinct boundary: the fixed-stat player
harness omitted UnitStats/CombatRatings/DerivedStats, so StatsPlugin's query never
matched the owner even with Level90. The fixture now uses the ordinary stat-entry shape and derived native health,
not manufactured health or SP. Native90 stat/actual124257 group-damage control
GREEN6.65sec atf53b75c0b (`holy-pressure-control-v4.log`). Five subsequent policy
cases genuinely RED: Hymn, Apotheosis, PoM, Guardian Spirit and Desperate Prayer.
Fortitude coverage also failed, but required the generic target118 mechanic and
a correctly formed companion group rather than an individual-cast workaround. The Purify case instead failed on17146's native95mana cost;
its corrected no-mana dummy uses authentic23268 (same72/3sec Magic DoT, no power
row). This setup failure is not a Purify policy RED.

The production Holy adapter now reads real patients, source-radius clusters,
native recovery/cost admission and owned41635 wards. Its cooldown/maintenance
ownership prevents generic rows undoing a refused choice. At4b0d483c2, six native decisions GREEN14.72sec. At694de6f59, expanded
qualification yields11GREEN, one Fortitude ignore and one genuine RED: seven
native10% hits at1sec spacing predict death within3sec while patients remain
above25%HP, but the old20%-only Guardian rule never casts it. Guardian now also
uses the shared projected-death deadline; both forecast and HP cases GREEN at
775bf36f2. Native70% Apotheosis Word discount also GREEN (not free). The separate
NPC rescue fixture initially failed because full party AI killed its damage
source before the first Bite. Advancing to the next ordinary dummy, without
restoring pools, reaches the real boundary: NPC Guardian rescue, aura consumption
and later death to the same native123483 hits GREENd71af72ec.
Earned-Surge priority, effective native32 Sanctify, spread-cluster refusal and
full-health cooldown conservation are native GREEN controls. Fortitude coverage
was explicitly ignored while native target118 raid expansion was missing;
common master655258480 now supplies that fix in both comparison trees and the
regression is re-enabled. At1fb04d63b the complete bounded run yielded36GREEN
and one Fortitude failure: the fixture had native actors sharing an owner but
never created a `GroupRegistry` party. Ownership is intentionally not membership.
The fixture now uses real `create_companion_party`, actual follower online records
and the owner's test ClientId; its membership and coverage proof remains pending.
No production policy change or individual-buff workaround.

Qualification now also covers real Holy Fire/Smite/Nova and native offensive
safety inputs. A new PoH opportunity case spends Apotheosis and Words through
ordinary casts/expiry before real moderate group damage. The existing Purify case
now checks its first decision on a non-emergency Magic patient above95%HP; the
earlier eventual-removal GREEN did not establish priority over an unnecessary
paid heal. At f6c6fffd4, the six Holy namespaces run43 cases:39GREEN/4FAIL109.81sec, build3m46sec/no warnings. Formal-party Fortitude and all five safety/consequence controls except the two positive weave cases pass. Purify genuinely chooses2050 before527 on actual non-emergency Magic above95%HP. PoH never casts596 on the moderately injured native cluster after real Word depletion; per-patient shared ranking omits its aggregate benefit. These are Holy policy REDs. Holy Fire/Smite launches585 but fails the assumed new-target damage assertion; Nova never launches132157. Their setup/admission boundaries remain under diagnosis, not established production bugs. Four failures are explicitly ignored pending qualification; original output is `holy-successor-f6-targeted.log` in the diagnostics root.

Purify priority root cause: generic kit utility rows run only after shared triage
has tried its ordinary healing candidates. Even negligible real23268 injury
therefore admits2050 before the available527. Holy now owns527 and selects the
ranked actually dispellable patient after emergencies/MassDispel, before routine
heals. Native kit eligibility retains Magic/Disease types and ordinary spell
admission. Ignored first-action regression is pushed before this change; GREEN
execution at fcdcb79cd: first actual527 and aura removal GREEN. All43 including
ignored give40PASS/3FAIL94.45s; remaining PoH and two weave boundaries below.

Smite diagnostic proves real585 dealt2203 damage to the original engaged target364,
not the fixture's freshly spawned374. Retaining the live original target is
correct; the fixture must not demand an unsolicited target swap. Corrected test
waits for Holy Fire then Smite damage on the same original enemy.

PoH ranking root cause: shared candidates estimate one recipient, making native596
look smaller and more expensive than2061 despite its real multi-patient footprint.
Holy now owns596, sums the existing effective estimates for injured members of
each native primary-centered cluster, and compares healing per native mana with
available ordinary direct heals. Three injured recipients are required; best
cluster ties use entity identity. Surge/Words and throughput cooldowns precede
this choice, PoM upkeep follows it. No shared-ranking change, synthetic healing
or additional spell metadata. Original opportunity regression predates implementation.

Native e9697c1db trace confirms a separate shared lifetime blocker:
`FOLLOWER_GROUP_HEAL_TOPPED_ANCHOR_CANCELS`. Actual596 CastStart progresses2.25sec;
at2.30sec another party heal tops its anchor while three other patients remain
injured. Shared `cancel_topped_cast` cancels before native2.306sec completion.
Retry on another anchor likewise cancels at2sec with two other patients injured.
This is not a596 native refusal or a missing Holy decision. Original full-party
regression stays ignored RED; it is not replaced or claimed GREEN. An additional
Henry-only controller case isolates native group selection/consequence without
other followers healing the anchor. Shared gameplay remains frozen for matched
cohorts; common fix and full-party GREEN belong to the lead's integration decision.
Evidence: `holy-poh-{refusal,lifetime}-targeted.log`. Temporary trace instrumentation
is removed before candidate freeze.

Nova root cause: selected132157 has native RangeIndex1/self, but the generic
kit's enemy target requests pursuit; `spell_reach` rejects its zero native range
before a cast can launch. Holy now owns132157 and self-targets only after shared
healing safety and actual native-radius engaged-enemy count pass. Same-copy alive
fresh enemies and breakable CC veto the footprint, matching the existing generic
AoE safety rule. This stays in Holy policy to keep frozen comparison runtime
difference specialization-only; no generic movement/range changes. Native positive
and safety controls pass at0de4fe1af: all five weave cases GREEN, including
actual Holy Fire→Smite damage, Nova on three engaged enemies, and incoming-damage,
fresh-pack and real Polymorph vetoes. The full-party PoH case alone remains RED.

## Frozen native and cohort qualification at d765e1901

Six bounded Holy namespaces give **43PASS/0FAIL/1namedignored in41.70sec**.
The44th case is the original full-party596 lifetime RED above, not waived or
replaced. Production differs from baseline027798898 only in `holy_policy.rs`;
shared triage, observers, simulator31f288baf, common655258480 and protocol03cebdb
are identical. Same uncommitted sim serveropt1/deps3 override; frozen SQLiteSHA256
874178ece3c6d41e738d915329ffdf84d50a206450e2529daeb4eadd6b4a8b5d plus read-only
DB2/gametables/rootJSON manifest. Roster209057,215011,209053,209059,209065;
seeds1–20/jobs2 each side, Stockade34/1/level32 and Azure2515/205/level70.
All80 reports/context validations complete; provisional drifted baselines excluded.

| Metric | Stockade before→after | Azure before→after |
|---|---|---|
| Clears / party deaths / Henry deaths | 0→0 /25→25 /0→0 | 0→0 /25→34 /0→0 |
| Henry-source overheal | 68.338806%→68.405532% | 39.008917%→43.658042% |
| Whole-party-source overheal | 18.649303%→18.221189% | 30.521694%→27.986640% |
| Serenity applied/requested seconds | 5072.55/5490→5098.05/5522 | 13332/13332→14638/14642 |
| Surge uses/effective gains | 109/112→87/95 | 220/223→250/253 |
| route/pull/death/sim blockers | 0/1/16/3→0/1/15/4 | 0/0/20/0→0/0/20/0 |

**No overall-quality improvement or route-completion/live proof.** Azure deaths
and Henry overheal worsen; party-source overheal improves. Native596 launches0→11,
PoM0→149, Fade0→128 and Guardian0→3 in Azure, demonstrating real decisions, not
full-run success. Simulated exposure differs: Stockade50074.75→52788.3sec,
Azure8608.2→11946.8sec. Differently truncated totals are not fixed-duration
throughput. Native Death-event order places every additional Azure death after
Garrick: before0pre/5post, after0pre/14post, with20tank deaths both sides.
Native0.5sec samples bracket median tank-death upper bounds88→89simsec; median
paired-seed delta+0.5sec. Median terminal delay after tank death266.65→548.55sec.
Longer post-tank followup permits more secondary deaths; this neither erases the
34versus25 total nor establishes direct healer-quality causality. Exact brackets
and native event order: `azure-death-exposure-analysis.json` in the same diagnostics.
Guardian48153 heals are self-sourced by the protected recipient;
party totals include them without silently attributing ally rescues to Henry.

Native0.5sec mana curves remain separate CSVs; median run-minimum fractions
Stockade0.741566→0.706916 and Azure0.777966→0.624797. After Stockade non-death
seeds7/11/16/17/18 show Henry no cast/full4430mana,99–118yd from tank. Observations
only, not an established route root cause; generic simulator fixes remain13179-owned.

Engine diagnostics `holy-priest-2026-10-21/frozen-comparison-report.md` holds exact
raw numerators, mana/exposure summaries and coverage matrix;
`comparison-{stockade,azure}/comparison.json` holds summarize.py outputs with
blocker evidence/lineage, each beside `before-mana.csv` and `after-mana.csv`.
Raw20-seed directories, scope/validation receipts, archived executables and SHA
receipts remain unchanged. `holy-freeze-{build,targeted}.log` and
`freeze-native-scope.json` identify exact native filters/revision. Lead final
check/readability/integration gate pending; bounded tip is not full goal completion.

## Named selected-kit gaps

Verified source inspection at66f0b1d6b: these numeric spell IDs have no class-specific
consumer in server Rust. DUMMY fields with no trigger cannot supply the listed
consequence through the generic proc dispatcher. This does **not** mark unrelated
ordinary modifiers on the same talent unsupported. These gaps are not silently
filled by AI policy, nor certified by generic aura admission.

Gap names below have prefix `HOLY_`:

| Gap | Native parent / consequence |
|---|---|
| ECHO_OF_LIGHT_PRODUCER | 77485: direct heal →77489 HoT |
| SPIRIT_REDEMPTION_LIFECYCLE | 20711: lethal hit →27827 state; generic316 only accounts absorption |
| RESTITUTION_REVIVAL | 391124: healing-dependent return after Spirit expiry |
| FOCUSED_WILL_MELEE_PROC | 45243: actual melee →45242 protection |
| TWINS_INFUSION_SHARE | 373466: additional native10060 recipient |
| COSMIC_RIPPLE_RECHARGE | 238136: Word recovery →243241 group heal |
| LIGHTS_RESURGENCE_RENEW | 193157: PoM heal →139 chance |
| BINDING_HEALS_SELF_TRANSFER | 368275: healing another patient also heals Henry |
| EVERLASTING_LIGHT_MISSING_MANA_CHANCE | 1249233: native50% missing-mana scaling; base1091868% producer is separate |
| DIVINE_SERVICE_BOUNCE_SCALING | 391233:33110 amount depends on remaining41635 uses |
| PROTECTIVE_LIGHT_SELF_FLASH_PROC | 193063: self Flash →193065 protection |
| DIVINE_IMAGE_SUMMON_EMPOWER_COPY | 392988: Naaru lifecycle and mirrored casts |
| PRISMATIC_ECHOES_MASTERY_BONUS | 390967: mastery-specific script scalar; ordinary Renew modifier remains data-backed |
| LASTING_WORDS_RENEW_APPLICATION | 471504: timed139 from Serenity/Sanctify-or-Ultimate recipients |
| TRANSLUCENT_IMAGE_FADE_EXTRA_REDUCTION | 373446: extra373447 consequence; base586 reduction remains native |
| ANGELIC_BULWARK_THRESHOLD_SHIELD | 108945: threshold114214 shield and native recovery window |
| BENEDICTION_HYMN_RIPPLE_AND_UPGRADE | 1262760: Hymn pulses and next-Flash upgrade |
| LEAP_OF_FAITH_EFFECT77 | 73325: native77 has no effect handler; do not cast an imaginary rescue |

## Native utility decisions

Atd71af72ec, actual three23268 Magic applications all exist but Henry never casts
32375; actual native threat selects Henry but AI never casts586. Both are genuine
policy REDs (`holy-{mass-dispel,fade-policy}-v8.log`), not auras fabricated to
force eligibility. Candidate policy uses shared observed victims for Fade and
real dispellable-Magic/native-radius footprints for Mass Dispel; positive and
separated/other-victim/empty controls GREENeaa4ea879: two native dispel cases,
one empty-cooldown case and four native threat cases. This proves ordinary Magic
removal and actual target selection, not special normally-undispellable potency.

Mass Dispel's normally-undispellable-Magic potency remains **unqualified**, not
asserted missing solely because72734 is a marker. Ordinary effect38 behavior is
separate. Source-radius clusters and authored upcoming timers are now supplied
by shared triage; only those observed inputs may justify geometry/preparation.

## Sources

- [Selected inventory](../../specs/holy-priest-kit.md) — local70448 metadata.
- [Decision contract](../../specs/holy-priest-follower-ai.md) — project policy.
- `crates/server/src/auras/{handlers,procs,scripts}.rs` — generic proc boundary.
- `crates/server/src/spell_cast/scripts/{clearcasting,holy_priest}.rs` — bounded
  class-script precedent and candidate fix.
- `crates/server/src/followers/holy_priest_tests.rs` — actual follower regression.

## See Also

- [[follower-data]] — shared follower spellbook and native fixture provenance.
