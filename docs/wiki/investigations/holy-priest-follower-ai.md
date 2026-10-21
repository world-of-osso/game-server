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

Mass Dispel's normally-undispellable-Magic potency remains **unqualified**, not
asserted missing solely because72734 is a marker. Ordinary effect38 behavior is
separate. Source-radius clusters and upcoming-timer integration are pending the
shared triage slice; absence is not a fabricated safe footprint or forecast.

## Sources

- [Selected inventory](../../specs/holy-priest-kit.md) — local70448 metadata.
- [Decision contract](../../specs/holy-priest-follower-ai.md) — project policy.
- `crates/server/src/auras/{handlers,procs,scripts}.rs` — generic proc boundary.
- `crates/server/src/spell_cast/scripts/{clearcasting,holy_priest}.rs` — bounded
  class-script precedent and candidate fix.
- `crates/server/src/followers/holy_priest_tests.rs` — actual follower regression.

## See Also

- [[follower-data]] — shared follower spellbook and native fixture provenance.
