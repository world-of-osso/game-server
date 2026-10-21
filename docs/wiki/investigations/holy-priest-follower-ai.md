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
charge restoration. Targeted qualification remains pending; no GREEN claim.

## Iteration and comparison

Both owned checkouts use an uncommitted `[profile.sim.package.server]
opt-level=1` override authorized by the lead; dependency optimization remains3.
Before/after seed comparisons must use the same override. The detached simulator
checkout's target symlinks only to this agent's own Holy target; no target copy or
cross-agent target sharing. Capacity admission still gates every build/run.

## Sources

- [Selected inventory](../../specs/holy-priest-kit.md) — local70448 metadata.
- [Decision contract](../../specs/holy-priest-follower-ai.md) — project policy.
- `crates/server/src/auras/{handlers,procs,scripts}.rs` — generic proc boundary.
- `crates/server/src/spell_cast/scripts/{clearcasting,holy_priest}.rs` — bounded
  class-script precedent and candidate fix.
- `crates/server/src/followers/holy_priest_tests.rs` — actual follower regression.

## See Also

- [[follower-data]] — shared follower spellbook and native fixture provenance.
