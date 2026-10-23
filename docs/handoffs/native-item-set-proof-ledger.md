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
- Native set lifecycle implemented after genuine RED: original Retail ItemSparse.ItemSet counts, native thresholds/legacy flag/skill/spec/subtree gates, separate ItemSet aura ownership and ordinary aura application/removal. Equipment, ActiveSpec, TraitLoadout and skill changes trigger reconciliation. Targeted GREEN pending; Equipment facts remain unimplemented.
- Private `item-sets.delta.sql` SHA3032b1eb6d352e6a32d042e072575292772f7fa809b4570e7d130922b347b4ed adds only the two tables and build receipts to unchanged trait-enabled private source. Canonical publication remains parent-owned.
- Source census: active native sets have no nonzero shapeshift mask/exclude fields (one zero-mask StanceBarOrder-only row27733). Heirloom level/bonus-curve retirement and non-Retail set namespaces are not qualified by this slice.

## Tier alias policy

Parent-approved `SIMC_TIER_ALIAS_ITEMSET_BINDING`: pinned SimC76f8118 `engine/dbc/generated/item_set_bonus.inc` supplies tier aliases such as mid2→native ItemSet2062(Ret),2065(Elemental). DB2 owns membership and spells; SimC alias is not a native DB2 tier field. Generic set facts must reject unknown aliases, not default to false. Binding implementation pending.

## Remaining

Native runtime RED→GREEN; lifecycle isolation; private table delta; parent integration; generic APL set facts; tier spell-effect/proc qualification; ST/3T DPS and dungeon observations. None is complete merely because tables import.
