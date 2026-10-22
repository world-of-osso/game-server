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
- Source-role integration `dbb2e3d37` removes unconditional attack-name lists; source damage/heal defaults consume native/DB2/TC/named-child facts while condition-bearing selection rules remain explicit. Burning Rush initially regressed to enemy attack; actual native self-damage selector1 produced a genuine new RED, then the generic self-only damage rule restored utility. Original six producer REDs are GREEN.
- Stable native interval identity: generic family projection initially merged native Blessing1022 key28 into new alias key23. Genuine RED `9d6ac68e8` → `02cefcb9f` preserves actual native interval starts;22 source/rotation controls pass, with four disjoint intrinsic controls retained. No interval-removal guard weakened.
- Provisional full COPY packet at `963c40f8f`: `E/source-kit-role-diff-v1.{json,md}`, `source-role-classified.{world.db,receipt.json}`, `source-role-stable-key-delta.{sql,receipt.json}`. Matched1090 old rows equal the sealed baseline exactly;1094 prepared rows,0 removed keys,43 stable-key updates including four additions.24 intrinsic-classification role changes (20 utility→attack,4→heal),0 policy role changes; all11 per-spec histograms and explicit policy-retained counts are in the JSON. Stable-key replay passed.
- Copy-producer integration7 passed; one concurrent canonical input-SHA drift refusal was rerun only as the failed idempotence case against immutable input and passed (37.942s). Production hash guard remains unchanged. Proofs: `E/source-role-{copy-producer-controls,copy-idempotence-frozen}.*`.
- **Unpublished/provisional**: lead requires native-book initialization and selected-variant resolver to land first. Additional non-APL CC/off-form/filler exclusions are a pending lead behavior decision; do not silently treat every sourced damage effect as intended APL use. No runtime effectiveness/DPS claim.

## Producer input

`follower_spec_kits.py` requires `--tc-child-edges <artifact>`; `kit_rows` receives that path explicitly. No slot-local diagnostic alias or alternate search path. Receipts record the artifact and classifier/policy SHA-256s. The initial integration probe failed on an absent slot-local artifact path before assertions; it is a setup failure, not behavioral RED.

Copy-producer tests require `FOLLOWER_KIT_TEST_WORLD_DB` and a source JSON whose `world_db` names that same immutable snapshot. The first8-test integration probe had7 passes and one explicit input-SHA drift refusal while canonical publication ran concurrently (`E/source-role-copy-producer-controls.*`); not an output/idempotence failure. The production source hash guard is unchanged. Only the failed case needs a bounded rerun against the frozen input; preserve the other exact command receipts and their input qualifications.

## Approved policy boundary

`role_slot` is also an executable action policy. Lead approved retaining small cited APL exclusions separately from generic source classification. Arcane Explosion stays intrinsically damaging but utility1000/AI-disabled under its Fire APL contract. Read-only audit found32 existing utility roots with sourced damage/heal reachability, including self-damaging Burning Rush, pet-only healing and conditional crowd-control damage. Intrinsic reachability alone must not invent enemy targeting or unconditional APL use; distinguish classification-driven changes from policy-retained rows in the full delta.
