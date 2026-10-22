# Source-driven follower kit classifier

## Active goal

Lead-approved: replace the role NAME whitelist with classification from native effects, DB2 trigger/child chains and pinned TC-scripted child bindings. Treat damage, healing and utility consistently. Preserve established intentional action policy; only source-ambiguous overrides may remain small and cited. Run RED-first fixtures for Shield Charge/Feral Frenzy and existing role controls; diff the full kit DB and report per-spec role changes. Publish only a prepared-copy delta packet for the lead; never mutate canonical world.db.

## Reproduced input boundary

- Current source796e0e7c9, native snapshot464fbb46:73DPS54569.60/103DPS49373.95, both18/20adds, no Shield Charge/Frenzy casts.
-385952 exists61–90 but `utility`/priority1000 is skipped by AI. Parent Dummy3/trigger0; native description references damage385954. TC a352b1fa `spell_warrior.cpp:1773–1789` casts385953; native385953 effect96 triggers damage385954. The full chain is385952→TC385953→DB2385954, not a direct TC damage-child cast.
-274837 exists only55–56 as utility1000. Native starter export learns1243807 from57 through90 instead. TraitDefinition138989 explicitly sets1243807 OverridesSpellID274837; default replacement is real, not arbitrary clipping. Preserve canonical family availability with selected-variant resolution, not an unearned base talent grant. Native Frenzy description names274838 damage; pinned SimC `sc_druid.cpp:4114–4140` supplies its scripted child behavior. TC has no indexed named Frenzy script.
- Existing producer `combat_policy` is a name-based role/priority policy with final utility1000; `kit_rows` intersects only the selected starter books and compresses their actual observed levels. Both layers explain the failure.

## Proof ledger

- RED fixture file: `scripts/test_follower_source_roles.py`. Actual native all-level source exports and read-only native DB feed `kit_rows`; concrete output rows assert Shield Charge damage, Frenzy canonical availability through90 and existing Fireball/Regrowth/Blink roles.
- Producer RED `dd4ab3d26`: six subtest failures; existing Fireball/Regrowth/Blink control passed (`E/source-kit-classifier-red.*`). No production role-classifier integration yet.
- Native-family projection `399469c58` reads `TraitDefinition` and `SpecializationSpells.OverridesSpellID` recursively for non-passive controller actions; does not change the native source book. Explicit Berserk/Incarnation retirement remains separate. Partial proof `E/source-family-alias-partial.*`: Frenzy canonical action now exists at55/56/57/90; six role assertions remain RED. Existing Berserk projection control passed.
- Source-only intrinsic graph now has six explicit description-confirmed SimC child bindings in `scripts/follower_spell_role_policy.py`, separate from APL selection: Feral/Frantic Frenzy, Blessed Hammer, Kill Command, Wild Thrash and Emerald Blossom. Genuine Frenzy2-failure RED→GREEN and command/area4-failure RED→GREEN; current `bcc604b49` graph4/4 tests pass (`E/source-{frenzy-child,command-area}-role-*`). Child reachability does not certify execution or owner damage attribution.
- Full DB before/after role census, producer integration and delta replay remain pending. Do not publish the controller aliases before authoritative native-book initialization and selected-variant resolution.

## Producer input

`follower_spec_kits.py` requires `--tc-child-edges <artifact>`; `kit_rows` receives that path explicitly. No slot-local diagnostic alias or alternate search path. Receipts record the artifact and classifier/policy SHA-256s. The initial integration probe failed on an absent slot-local artifact path before assertions; it is a setup failure, not behavioral RED.

## Approved policy boundary

`role_slot` is also an executable action policy. Lead approved retaining small cited APL exclusions separately from generic source classification. Arcane Explosion stays intrinsically damaging but utility1000/AI-disabled under its Fire APL contract. Read-only audit found32 existing utility roots with sourced damage/heal reachability, including self-damaging Burning Rush, pet-only healing and conditional crowd-control damage. Intrinsic reachability alone must not invent enemy targeting or unconditional APL use; distinguish classification-driven changes from policy-retained rows in the full delta.
