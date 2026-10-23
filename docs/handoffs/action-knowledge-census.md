# Native follower action-knowledge census

Verified2026-10-23. Generic controller knowledge gating is not enabled until selected visible/parent actions remain admissible. Raw `KnownSpells::contains` alone is incorrect.

## Native evidence

`followers::kit_source_tests::follower_kit_export_native_source` ran against the private Feral snapshot with exact-master9d13ac19c binary. PASS24.02s;990validated native starter books (11profiles×90levels). The startup-loader inode test independently proved both baked startup loaders resolve to the private database in the bubblewrap source view. No source/canonical DB writes.

Artifacts: engine `data/diagnostics/feral-parity-2026-10-23/all-follower-native-starter-books{.json,.proof.json}`, `all-kit-action-knowledge-census.json`. All1090published kit rows were compared across every level where each row is available.

| Follower / spec | Published rows | Raw unknown rows | After sourced bridge |
|---|---:|---:|---:|
| Shuja Grimaxe262 |93|1|0|
| Captain Garrick66 |104|0|0|
| Meredy Huntswell63 |90|0|0|
| Austin Huxworth253 |107|0|0|
| Crenna Earth-Daughter105 |107|0|0|
| Henry Garrick257 |88|0|0|
| Kybel1467 |87|0|0|
| Warlord Breka Grimaxe73 |98|0|0|
| Bo269 |109|0|0|
| Lantha Shadeborne266 |99|2|0|
| Ralia Dreamchaser103 |108|1|0|

## Every raw rejection

| Action / levels | Actual known source at every rejected level | Classification |
|---|---|---|
| Berserk106951,29–58 |343223; selected TraitDefinition108167.VisibleSpellID106951 | Selected visible action, not an unselected talent. Incarnation102543 overrides106951 through TraitDefinition108183 when selected. |
| Ascendance1219480,51–90 |114050; native114050 description and existing producer `SIMC_TALENT_CAST_ACTIONS` | Named sourced parent→actual-action bridge. No queried TraitDefinition.VisibleSpellID binds this pair. |
| Hand of Gul'dan105174,11–90 |1250273; selected TraitDefinition130665.VisibleSpellID105174 | Selected visible action. The same definition overrides172 with passive1250273; an actor requesting the baseline needs the actual cast action, not the passive aura. |
| Demonbolt264178,15–90 |Demoniac426115; SimC76f8118 `sc_warlock_init.cpp249–250` conditional lookup | Named sourced parent→actual-action bridge, already used by `follower_spec_kits.py`. Initial raw classification as unselected was wrong;264178's separate TraitDefinition120942 is not the only admission source. |

There are zero missing-from-book rejections after these four sourced bridges in the990starter books. This source/metadata census does **not** prove a new runtime helper or every custom reference loadout; native behavioral RED/GREEN remains required. Never admit arbitrary same-name spell IDs or all class talents.

## Shared helper contract (draft)

`crate::spell_cast::knowledge::resolve_actor_action(&World, Entity, requested: u32) -> Result<u32, CastFailure>` returns the actual cast ID. It must preserve raw knowledge, selected TraitDefinition visible/override actions and native aura overrides. Existing source-qualified parent bridges are ledgered only where native metadata lacks an action link. Unknown/unselected actions fail; no new spell is inserted into KnownSpells. Both ordinary follower kit selection and literal fixture execution consume the same result. Generic NPC request admission is unchanged.

Seven native cases are committed in `followers/action_knowledge_tests.rs`: selected Feral106951 and Demo105174 on both controller paths, unselected Incarnation on both paths, and selected Demoniac→Demonbolt literal admission. They are pending execution, not claimed RED/GREEN. The explicit Feral reference-loadout control separately exercises30s native kit and checks granted Shred remains usable.

## Sources

- [Existing producer bridges](../../scripts/follower_spec_kits.py), `SIMC_TALENT_CAST_ACTIONS` and its pinned source citations.
- [Native starter exporter](../../crates/server/src/followers/kit_source_tests.rs), [native progression](../../crates/server/src/class_progression.rs).
- [Feral source census](feral-source-census.md), [shared literal machinery](../wiki/systems/simc-apl.md).
