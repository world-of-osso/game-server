# Light of the Titans + native hit masks — 2026-10-22

## Scope

One native flagged producer378405→378412 for Protection66/Garrick209057,
ordinary Word of Glory85673; source-backed native self-DoT enhancement. Root
shared hit-mask correction approved separately by lead, with a required full
workspace gate. No AI policy, raw stats, resource grants, canonical DB edits or
other residual producers. Preserve landed Echo/Prismatic/Ignite and metrics work.

## Sources and exact inputs

Local70448: talent learned41, effect0=40%/child378412, effect1=200%, child10s/2s.
SimC76f8118 `sc_paladin.cpp:1129–1177,1251–1258` supplies producer hook/condition;
its legacy50%/NYI note is not native amount authority. TC a352b1fa has no Titans
script. Generic masks: `Spell.cpp:2343–2400`, Attr3 TreatAsPeriodic/Attr0 IsAbility
for non-weapon spells. Native Word DefenseType0 without IsAbility is a helpful
spell, not a helpful ability.

Code891b9ab1dbde68948b85ae16b8ca3bb7638203d1; protocol
2abc7fa844f6e71d93203ce5421ae853e884854d. T retains frozen world DB
`6b8b5e96b2a1f0caac05df74abce559516cacae9279d85e669d97edfd0dd7f2c`.
Builds use approved helper through `agent-run healer-ai`, normal capacity
admission, one owned job. Tests: `--test -p server --bin game-server --profile
test -- --nocapture --test-threads=1 <named filters>`; check:`--check -p server`.
No T source changes during jobs. Local sim-profile config remains uncommitted.

D = game-engine `data/diagnostics/healer-ai-2026-10-21/light-of-the-titans/`.

## Proof ledger

| Code / scope | Result | Evidence under D |
|---|---|---|
| 6ea5658cc first fixture | Cost-read prerequisite failure, not feature RED: spell_costs required power before generators | `native-cost-prerequisite-failure.log` |
| 234d8bfd4 next fixture | Resource-lifetime prerequisite failure, not feature RED: long injury setup followed earned budget | `native-resource-lifetime-prerequisite-failure.log` |
| 465af2443 actual injury→Judgment earned cost→Word hit | Genuine2PASS4FAIL10.25s: only missing378412; native40/unrelated Flash controls passed; real hostile DoT prerequisites passed | `native-red.log` |
| 733880de7 base producer + Echo controls | 8PASS4FAIL45.67s: Titans still absent because upstream Word helpful mask was0x400 instead of0x4000 | `native-base-green-and-dot-bonus-red.log` |
| b382b9d28 mask fix; test tip2fb442aa2 | 12PASS1FAIL43.13s: native mask controls/Echo/base Titans pass; genuine self-DoT amount110vs329 RED | `native-mask-green-and-dot-bonus-red.log` |
| 58ec9656c enhancement + master merge891b9ab1d | 19PASS0FAIL38.11s; compile12m45s | `native-final-green.log` |
| Same891 code default helper server check | EXIT0/2m08s, no warnings emitted | `final-check.log` |
| Same891 code cargo fmt --all --check | EXIT0 | `format-check.log` |

The19 cases are Titans6, Echo6, Prismatic2, generic rolling2, native mask contract1,
Surge1 and Earthen Rage1. Many admission-only pollers exhausted quiet-window or
capacity limits without launching a compiler; these are not additional test runs
or successful evidence. No unchanged broad suites rerun locally.

## Existing test expectation changes

**None.** Compared with merged origin/master, only new Titans/mask tests and
module declarations are added. Existing Echo/Prismatic tests are unchanged;
shared tick-count extraction preserves them. No old expected proc count, damage,
healing or resource oracle was edited to accommodate the generic mask change.
Therefore there is no changed-existing-oracle list requiring individual TC
citations. New mask controls cite TC `Spell.cpp:2343–2400` and use unmodified
native85673/5857/5394/1464/19434/2061 rows.

## Census and required lead gate

[Exact census handoff](native-hit-proc-mask-census.md):334969 changed possible
spell masks,5670 static proc-aura candidate sets. Exact spell/aura IDs and
before/after metadata are in the normalized CSV/JSON artifacts under D, with
source/native non-Paladin spot-check. Static candidate gains are not observed
procs; script guards/chance/phase/reflect/family/live-target gates still apply.
The lead requires **full workspace suite**, not merely these targeted cases,
before landing. Do not present that broad gate as passed from this ledger.

## Coverage limits

Native basic producer/self enhancement/expiry and negative admission are proven;
actual repeated-Word378412 pool interaction is not separately qualified. Shared
generic/Echo carry controls are retained but do not certify every producer.
No simulator efficacy, complete Protection kit, PvP scaling or pixel acceptance
claim. No source-snapshot/estimator architecture or proc-chance workaround added.
