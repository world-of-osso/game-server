# Native item set lifecycle

## Goal and source

Apply native ItemSetSpell spells from actual equipped ItemSparse.ItemSet counts; remove below threshold and replace on spec/subtree change. TC a352b1fa `Entities/Item/Item.cpp:76–242` owns count, legacy-inactive, skill, specialization and selected-subtree eligibility. Preserve individual ItemEffect/bonus/enchant spells. Private DB only; parent owns publication and integration gate. No invented effects or DPS tuning.

No native set-count path was present at60828114d. `equipment_effects.rs` applied individual equipped/bonus/enchant spells only. Its MID2 offhand test is ItemBonus13771→ItemEffect234561→1297382, not tier threshold application.

## Source fixtures

Native70448 ItemSet2062 Radiance of the Consecrated Flame has items271468/271466/271465/271464/271463, flags0/skill0. ItemSetSpell7715–7720 choose Ret70 roots1296660(two)/1296661(four), Prot66 roots1296658/59 and Holy65 roots1296656/57. Native2065 chooses Elemental262 roots1296625/26. Metadata presence is not spell-effect or proc qualification.

## Proof ledger

- c893c7499 introduced two ignored native Ret tests: counts1→2→5→3→1→4→0 and same actor70→66→70. Initial compile failed on fixture-only `Shoulders` spelling; corrected to existing `Shoulder` atc19c14e80. No behavioral RED yet.
- 2d411a47a normal importer RED: `python3 -m unittest scripts/tests/test_db2_item_sets.py -v`, assertion failed because ItemSet/ItemSetSpell output catalogs were absent;4.924s. `item-set-import-red-v2.log` under engine diagnostics `ret-parity-2026-10-23/`. Prior v1 SQL missing-table error is setup evidence, not behavioral RED.
- At80c23d207, normal importer1GREEN0.050s:1008 ItemSet/2971 ItemSetSpell; exact Ret/Elemental membership+threshold+spec+build receipts retained, unrelated table preserved.
- At80c23d207 Rust scope/c19c14e80 fixtures, native set2RED and generic Equipment3RED39.20s; Ret2062 count2 observed(false,false), expected(true,false); no tier aura in spec fixture. `equipment-set-red-v2-full.log` retains complete proof, private source SHA cb201a5e860f31fd6a1d31d3a0ef1b925e560130fdadd5fcd65ecbd443a39f15.
- Native set lifecycle implemented after genuine RED: original Retail ItemSparse.ItemSet counts, native thresholds/legacy flag/skill/spec/subtree gates, separate ItemSet aura ownership and ordinary aura application/removal. Equipment, ActiveSpec, TraitLoadout and skill changes trigger reconciliation. At a8bd77d3b, unchanged native thresholds/spec cases2GREEN, existing equipment4GREEN and owned commerce1GREEN53.88s; native weapon aura instance preserved across spec switches. Generic Equipment5expectedRED were batched into the same process (overall exit101), not misreported as part of the native GREEN. `item-set-green-equipment-red-v2-full.log`/`.proof.json` preserve exact command, code revision and private DB hash. No Rust warning. Equipment facts remain unimplemented; parent gate pending.
- Private `item-sets.delta.sql` SHA3032b1eb6d352e6a32d042e072575292772f7fa809b4570e7d130922b347b4ed adds only the two tables and build receipts to unchanged trait-enabled private source. Canonical publication remains parent-owned.
- Source census: active native sets have no nonzero shapeshift mask/exclude fields (one zero-mask StanceBarOrder-only row27733). Heirloom level/bonus-curve retirement and non-Retail set namespaces are not qualified by this slice.

## Current-master reconciliation (parent approved2026-10-23)

Masterdd63aec17 includes native ItemProcInputs and item_instance_data.PRODUCER_TABLES. The latter is now the single normal importer owner: duplicate-registration source test genuinely RED atbd8ec4a79 →GREEN0.635s atfc60b4a26, real producer importer exercised (not mocked), native1008/2971 and item_instance_source receipts retained. Native DDL remains for source-projected standalone fixtures.

Parent requires ItemProcInputs as the sole set catalog/predicate, with one count helper reading each original ItemTemplate's native ItemSparse.ItemSet. Native source shows241 active members absent from ItemSet.ItemID display arrays. New ignored RED fixtures use real Dark Arctic38591/38592 (set754/2pc51273), inactive Arcanist201 (3pc41657), and Blood Tiger442 (skill165/rank300,2pc41670). Atde7b6e406 all three failed genuinely: Dark Arctic51273 returnedfalse, inactive41657 returnedunsupported-predicate error, and skill41670 returnederrors instead of`false,false,true,false`. Combined native batch9RED/runtime56.50s; `reconcile-primary-judgment-red-v3-full.log` preserves exact assertions.

Reconciliation follows those REDs: ItemProcInputs now owns the only native set catalog and eligibility predicate; a spell→set index references those same catalog rows. One count helper reads original `ItemTemplate.item_set` projected from ItemSparse.ItemSet; never ItemSet display arrays or redirected stat identities. Lifecycle and APL tier facts consume this resource; duplicate ItemSets catalog/count/eligibility code is deleted. Real fixture loaders install the same native proc catalog. Atdd0ed8fe5 shared predicate3PASS, lifecycle/equipment6PASS, commerce1PASS, Equipment9PASS and retained Protection7PASS; warning-free native compile6m35s/runtime156.72s. Combined command also included separate launch REDs, failing Ret funding and the pre-existing explicitly ignored Venomfang producer RED, so overall exit101 is not called GREEN. Exact `reconciled-native-launch-red-v2.*` records the subset. Existing WW native consumer subsequently1PASS47.55s at02090739e; `ww-consumer-green-v1.*`. Parent's requested single replacement packet will merge master8e7c7a685 before integration.

## Input compatibility inventory (read-only audit2026-10-23)

Production loader deliberately requires both native tables; no empty-table fallback. `item-sets-canonical-readonly.proof.json` compares all canonical table values with native70448 CSVs:1008/2971 exact. Canonical `data/world.db` resolves engine `data/server-world/world.db`; its `item_instance_source` records native CSV hashes, while `db2_build` has no set receipts. Generic NUMERIC schema matches the item-instance import helper; the historical invocation/actor is not established. Normal importer now owns durable table registration.

| Test/input | Where it reads item templates | Set tables / action |
|---|---|---|
| Default native suite, startup, `world_db().powers_db`, bank/quest/loot/stats/class tests | Checkout `data/world.db` → current canonical source (or gate's explicit bind) | Current canonical has exact native tables; no data write needed. Any private bound replacement must include them. |
| `DPS_PARITY_WORLD_DB`, equipment/native parity/persistence-instance tests | Explicit private path; `dps_parity_native.py` also binds it over startup data | Own `world.ret-sets.db` already includes exact tables. Old trait/Ret baseline copies lack them; derive a new proof copy, do not rewrite frozen inputs. |
| WW ignored proof | Engine `data/diagnostics/ww-parity-2026-10-23/native-before-ww-policy/world.ww-frozen.db` | Both absent; new derived private copy plus delta required before using new loader. |
| `MYTHIC_PLUS_WORLD_COPY` | `mythic_plus.rs` completion fixture explicitly loads templates from this path | The physical gate path must contain both tables. Parent's gate assigns this to the historical skyborn world; derive `server/world.itemsets.db` with the SHA-checked delta and use that copy. |
| `TRADING_POST_WORLD_COPY`, `ADVENTURE_GUIDE_WORLD_COPY`, `SITE_DATA_WORLD_COPY` | Direct consumers load their own catalogs; their bank harness loads templates from base `test_data::world_db_path`, not these env paths | Tables required in base/bound world. Extra env DBs need them only if also used as that base. Parent's gate assigns TP to canonical, and Guide/Site to historical skyborn world; parent will switch Guide/Mythic/Site to the derived `server/world.itemsets.db` together. |
| Skyborn handoff source | Engine `data/skyborn-handoff/2026-10-07-c1cdf6e/server/world.db` | Both absent; any loader proof against this historical source needs a new derived copy plus delta. |
| Class-summon/goggle generated temporary worlds | `prepare_class_summon_db.py` / `prepare_achievement_goggle_db.py` backup the base world before scoped modifications | Inherit both tables from prepared base; no independent schema patch needed. |
| Generated owned-kit/commerce runtime SQL | `skyborne_kit_fixture.rs` generated by `scripts/skyborne_kit_fixture.py` | Producer previously discarded set schemas. Actual captured kit has zero ItemSet memberships. Projection genuineRED→GREEN1/1 3.067s at47d8b1ab6; only native table schemas added, existing captured rows unchanged. This is explicit fixture projection, not a runtime fallback. |
| Optional ignored auction/crafting process proofs | `AH_FIXTURE_WORLD`; `CRAFTING_PRIVATE_WORLD` when used by a full app | Require prepared native tables if these proofs are selected; paths are caller-owned, no default inferred. |

Python normal-import test constructs its own temporary native schema; Forever projection proof needs `FOREVER_DB2_SOURCE` and emits existing source-decoder warnings for encrypted zero-filled sections without suppressing them.

## Tier alias policy

Parent-approved `SIMC_TIER_ALIAS_ITEMSET_BINDING`: pinned SimC76f8118 `engine/dbc/generated/item_set_bonus.inc` supplies tier aliases such as mid2→native ItemSet2062(Ret),2065(Elemental). DB2 owns membership and spells; SimC alias is not a native DB2 tier field. Generic set facts must reject unknown aliases, not default to false. Generic alias binding is covered by the nine focused Equipment controls; unknown aliases remain errors.

## Remaining

Parent integration; generic APL set facts; native skill/subtree eligibility proof; tier spell-effect/proc qualification; ST/3T DPS and dungeon observations. Targeted native lifecycle GREEN is not proof of tier child mechanics or DPS parity.
