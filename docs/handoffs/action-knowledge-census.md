# Native follower action-knowledge census

Verified2026-10-23. Shared controller knowledge gating preserves selected visible/parent actions. Raw `KnownSpells::contains` alone is incorrect. Current own-build15case GREEN is recorded below; independent integration gate remains open.

## Native evidence

`followers::kit_source_tests::follower_kit_export_native_source` ran against the private Feral snapshot using the supplied binary later identified as **UNKNOWN_PRE_9D13_MISLABELED**, not master9d13ac19c. PASS24.02s;990native starter books (11profiles×90levels). Its startup-loader inode test reportedPASS for both private-database paths. Raw outputs are preserved; corrected proof sidecars withdraw the revision claim. No source/canonical DB writes. Own a95c0df7a runtime census independently traverses the same990books and reproduces all four raw unknown-action families in the genuine knowledge RED.

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

Own c3934c31d native runtime census confirms zero lost kit actions in all990starter books after these four sourced bridges. It proves admission, not every action's mechanics or every custom reference loadout. Never admit arbitrary same-name spell IDs or all class talents.

## Shared helper contract (15case GREEN, integration pending)

`crate::spell_cast::knowledge::resolve_actor_action(&World, Entity, requested: u32) -> Result<u32, CastFailure>` returns the actual cast ID. It must preserve raw knowledge, selected TraitDefinition visible/override actions and native aura overrides. Existing source-qualified parent bridges are ledgered only where native metadata lacks an action link. Unknown/unselected actions fail; no new spell is inserted into KnownSpells. Both ordinary follower kit selection and literal fixture execution consume the same result. Generic NPC request admission is unchanged.

Fifteen native cases in `followers/action_knowledge_tests.rs` cover both controller paths, actual cast IDs from passive/override parents, Cat Swipe's existing48629 form source, Elemental/Demonology sourced parent actions, unselected Incarnation and the complete990starter-book admission census. Genuine RED at a95c0df7a:10FAIL/5PASS,51.69s test,48m39s cold compile,exit101,zero compiler warnings. Actual kit positive controls pass; actual unselected Incarnation casts and missing literal IDs fail. Exact command/environment live in engine diagnostics `knowledge-red.proof.json`. The explicit Feral reference-loadout control separately exercises30s native kit and checks granted Shred remains usable.

The implementation imports native TraitDefinition.VisibleSpellID and caches only the selected definition links in TraitLoadout. `spell_cast::knowledge::resolve_actor_action` is shared by both consumers; the former kit-only native form resolver is moved beneath that module. `SIMC_KNOWN_PARENT_CAST_ACTION` covers only the existing114050→1219480 and426115→264178 projections where DB2 has no operative action link. Selected visible/override metadata is resolved first. No same-name inference or spellbook insertion occurs.

Own c3934c31da0f5f05fc58e9a11dfda0f7e9da655c GREEN:15/15PASS,38.29stest,22m30compile,1391.13swall,exit0,zero compiler warnings. Native four-stage empower resolver control PASS13.06s. Both pre-existing Ret final/base verdict controls fail before the verdict at `native_action_override_tests.rs86`: Judgment `NoCharges` is not retried by their three-generator fixture. The resolver is not on that direct generator path; Ret owner14913 owns the fixture correction, independent integration remains open. No charge semantics changed. Tests remain explicitly native-DB-qualified via `--include-ignored`.

Proofs: `knowledge-green`, `empower-resolved-control`, `ret-final-verdict-control`, `ret-base-verdict-control` `.proof.json`/`.output.log` under the engine evidence directory. Own binary SHA256 `8dffa7758d3f9cf7a3d075384ff05e15d34ed8294a09e0f3f26286eef279e2e7`; shared-protocol2abc7fa844f6e71d93203ce5421ae853e884854d. Exact argv/environment/revision are in proof sidecars. This tip contains **no** TDB proc provenance importer or generic default-proc refusal guard; those await the effective audited source manifest.

## Passive endpoint correction

Native selected Ret TraitDefinition141572 replaces35395 with404542, but404542 is PASSIVE (Attributes0=64). Controller/APL knowledge is not permission to manually cast a passive. Own c51184367 genuine native RED returnsSome404542 instead of unavailable:1FAIL/18.25s,18m55compile,1155.11swall,zero compiler warnings; `knowledge-passive-red.{proof.json,output.log}`. After complete normalization, the shared resolver now rejects a passive final endpoint with existing `CastFailure::NotKnown`. Literal selection treats that as unavailable and falls through, not a fatal unsupported gap. The known book and low-level passive casting remain unchanged; passive→active Berserk/HoG bridges remain permitted. At9a889ad6b, the new endpoint control and14existing controller/bridge controls PASS; the old all-row census FAILS because it demanded selection of passive metadata rows. All38223rejected level-rows/684unique IDs are originally PASSIVE; zero originally active IDs were rejected. The corrected census still visits every row/profile/level: returned endpoints must be active, unavailable original passive metadata is expected, and any lost originally active row fails. Its targeted qualification remains pending. Production resolver is unchanged by that test correction;14781 receives the same contract through the lead.

## Crate split B ownership

The controller adapter `server::spell_cast::knowledge::resolve_action(&World, Entity, u32) -> Result<Option<u32>, String>` lives in the server library beside the shared resolver. Unknown/NotKnown endpoints returnNone; unsupported source errors remain explicit. Server follower tests call this library policy directly; `server-simulation` Prepared execution imports it. No server→simulation dependency or test-name change is introduced. The underlying actor-action resolver, selected metadata, actual cast IDs and passive endpoint semantics are unchanged. The existing resource probe/cost contracts `read_native_cast_at_resource_max`, `applicable_costs` and `CostPool` are public server SPI for the same dependency direction; no new resource semantics are added. First two-crate compile exposed their old crate-only visibility before any tests ran; that is compile evidence, not a behavioral failure. Post-split targeted checks must cover both crates before the replacement gate.

## Sources

- [Existing producer bridges](../../scripts/follower_spec_kits.py), `SIMC_TALENT_CAST_ACTIONS` and its pinned source citations.
- [Native starter exporter](../../crates/server/src/followers/kit_source_tests.rs), [native progression](../../crates/server/src/class_progression.rs).
- [Feral source census](feral-source-census.md), [shared literal machinery](../wiki/systems/simc-apl.md).
