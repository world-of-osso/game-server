# Retail scenario progression

Scenario definitions and ordered main stages come from local Retail 12.1.0.69933 Scenario/ScenarioStep/CriteriaTree/Criteria records. Local TDB1210 `scenarios` binds map, difficulty and faction. Never infer a scenario from a map name or replace authored stage ordering with boss ordering. Bonus steps (Flags bit1) are not main-stage transitions.

Entry initializes copy-scoped criteria, not lifetime achievement counters. Only active-stage criteria accept gameplay credit. Completing its tree advances to the next ordered main step; the index past the final main step means scenario complete. Map exit clears the tracker; distinct instance copies never share credit. Criteria event matching/counter/tree semantics reuse achievement machinery.

Both client skins expose the same ScenarioObjectiveTracker header, stage block and current criteria. Forever changes art only. Completion replaces objectives with completion state.

## Selected local slice

Gnomeregan: map90, normal difficulty1, Scenario649, main Step1176, root36986, leaf36987/Criteria18529 (KillCreature7800, amount1). Canonical world.db has404 creature spawns, including one Mekgineer Thermaplugg7800. Four bonus steps remain optional, not fabricated sequential stages. This source has one main stage: live evidence can prove entry, credit and advance1→2/completion, not a multi-main-stage playthrough.

Standalone scenario maps currently have no canonical spawns. Populated multi-main-stage scenarios require scripted criteria92 and world-state modifiers absent from existing achievement evaluation; do not bypass those requirements. These remain separate content coverage gaps.

## Data preparation (no canonical writes)

Extract FDIDs1139062 and1132761 with installed casc-local into engine `data/db2/12.1.0.69933`. Then run `python3 scripts/import_scenarios.py --db2-dir <that directory> --tdb <local TDB_full_world_1210.26091_2026_09_09.sql> --db <private copy.db>`. Existing Criteria/CriteriaTree CSVs are required. Importer replaces only scenario/scenario_step/scenario_map and criteria reference tables, records the build and imports no creature/gameobject rows. The CLI refuses canonical `data/world.db`.

Layouts: WoWDBDefs Scenario B7C9ECF5 and ScenarioStep4F644CD3. Runtime/skin contract: local Retail Blizzard_ObjectiveTracker/Blizzard_ScenarioObjectiveTracker.lua (Update, UpdateCriteria, UpdateStageBlock); completion is currentStage>numStages.
