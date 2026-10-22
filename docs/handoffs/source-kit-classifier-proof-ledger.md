# Source-driven follower kit classifier

## Active goal

Lead-approved: replace the role NAME whitelist with classification from native effects, DB2 trigger/child chains and pinned TC-scripted child bindings. Treat damage, healing and utility consistently. Preserve established intentional action policy; only source-ambiguous overrides may remain small and cited. Run RED-first fixtures for Shield Charge/Feral Frenzy and existing role controls; diff the full kit DB and report per-spec role changes. Publish only a prepared-copy delta packet for the lead; never mutate canonical world.db.

## Reproduced input boundary

- Current source796e0e7c9, native snapshot464fbb46:73DPS54569.60/103DPS49373.95, both18/20adds, no Shield Charge/Frenzy casts.
-385952 exists61–90 but `utility`/priority1000 is skipped by AI. Parent Dummy3/trigger0; native description references damage385954. TC a352b1fa `spell_warrior.cpp:1773–1789` casts that child.
-274837 exists only55–56 as utility1000. Native starter export learns1243807 from57 through90 instead. TraitDefinition138989 explicitly sets1243807 OverridesSpellID274837; default replacement is real, not arbitrary clipping. Preserve canonical family availability with selected-variant resolution, not an unearned base talent grant. Native Frenzy description names274838 damage; pinned SimC `sc_druid.cpp:4114–4140` supplies its scripted child behavior. TC has no indexed named Frenzy script.
- Existing producer `combat_policy` is a name-based role/priority policy with final utility1000; `kit_rows` intersects only the selected starter books and compresses their actual observed levels. Both layers explain the failure.

## Proof ledger

- RED fixture file: `scripts/test_follower_source_roles.py`. Actual native all-level source exports and read-only native DB feed `kit_rows`; concrete output rows assert Shield Charge damage, Frenzy canonical availability through90 and existing Fireball/Regrowth/Blink roles.
- Runtime RED pending; no production classifier implementation yet.
- Full DB before/after role census and delta replay pending.

## Policy boundary awaiting lead

`role_slot` is also an executable action policy. Arcane Explosion is intentionally disabled as utility1000 by an existing cited Fire APL contract despite native damage effects. A purely intrinsic-damage→combat-slot rewrite would activate such exclusions. Lead asked to confirm retaining small cited APL exclusions separately from generic source classification; no silent reinterpretation.
