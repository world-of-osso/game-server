# Pet battle engine

Standalone `server::pet_battle` library. No Bevy plugin, networking, UI, encounter spawning, journal code or startup changes. Current capability is bounded; see [full property coverage](../../pet-battle-effect-coverage.md), not a blanket Retail parity claim.

## Content

`Catalog::load_world_db` strictly loads ten battle SQLite tables. Missing schemas, invalid types, conflicting known state values and dangling turn/ability/aura references return contextual errors. Identical species4597 speed-state rows are deduplicated; undefined species-state rows (35 in this export) remain in `unresolved_states`. No runtime CSV fallback. Tests copy the ten battle tables from shipped `data/world.db` into isolated in-memory SQLite; no server-local CSV export dependency or mutation of shipped data. No data is committed.

Runtime producer: `scripts/import-pet-battle-tables.py <local-CASC-CSV-directory> <world.db>` atomically replaces the ten tables below, preserving unrelated tables. Original DB2 column names and numeric SQLite affinity are retained: descriptions/LuaName/ParamLabel fields are TEXT, StateMultiplier REAL, other fields INTEGER. The account pet catalog startup loads and retains the turn catalog from the same world.db. The importer requires all ten exports before beginning the database transaction; malformed rows fail explicitly. `scripts/tests/test_pet_battle_tables.py` proves multiline text, signed values, numeric types, idempotence, preservation and invalid/missing export rollback.

Verified 2026-10-10: canonical resolved world.db backed up beside it as `world.db.pre-petbattle-20261010`, then imported. Species3001, toy1166, content_item58185 and spell_effect629424 rows preserved. Current corrected CASC exports contain8086 species-state rows and14975 species-ability rows; the older inventory counts in the historical handoff predate corrected exports.

| DB2 | SQLite |
|---|---|
| BattlePetAbility | battle_pet_ability |
| BattlePetAbilityTurn | battle_pet_ability_turn |
| BattlePetAbilityEffect | battle_pet_ability_effect |
| BattlePetEffectProperties | battle_pet_effect_properties |
| BattlePetState | battle_pet_state |
| BattlePetSpeciesState | battle_pet_species_state |
| BattlePetBreedState | battle_pet_breed_state |
| BattlePetBreedQuality | battle_pet_breed_quality |
| BattlePetSpeciesXAbility | battle_pet_species_x_ability |
| BattlePetAbilityState | battle_pet_ability_state |

### Numeric provenance

Species + breed raw stamina/power/speed states multiplied by DB2 quality multiplier and level. Health `round(raw*quality*level/20)+100`; power/speed `round(raw*quality*level/100)`. Constants20/100 and health offset100 are runtime rules, **not derivable from DB2 alone**. Pinned TrinityCore `a352b1fa` CalculateStats supplies them; its rounding comment remains an upstream uncertainty. Preserve float32 arithmetic: Rare's CSV value.649999976 evaluated in f64 incorrectly makes species39 health1562 instead of float32 result1563. Species39/breed3/Rare/25 fixture proves1563/260/276; identical species4597 speed50 rows yield284, not a summed293.

Retail tooltip `Blizzard_FrameXML/SharedPetBattleTemplates.lua` defines power scaling `points*(1+.05*power)`, variance spread `variance/200`, healing dealt/taken multipliers and floor-formatted output. Blizzard5.3 accuracy article supplies baseline5% critical,1.5x critical damage and+/-10% variance. Current explicit DB2 variance overrides baseline spread. Old Legion emulator's2x critical is not followed.

Local family abilities236..245 describe passives and displayed strong+50%/weak-33%. Current weak factor.67 follows that displayed percentage; exact game-table factor and runtime rounding still need independent Retail proof. Beast is+25% below **half** health, not below25% HP. Aquatic's current description includes outgoing DoT+25%, as well as incoming DoT reduction50%.

### Round pipeline

`Battle::step` consumes immutable state + two decisions + injected `BattleRng`; returns next state and ordered semantic events. Trait draws uniformly0..999999; caller owns seed/replay. No global time or random state. Forfeit ends immediately. Faint replacement requires an explicit swap, does not tick/advance rounds, and requires the living opponent to pass.

Regular decisions are preflighted through every turn and referenced aura graph before RNG. Voluntary swaps precede attacks. Speed determines attack order; ties draw50/50 (legacy runtime reference, not captured Retail evidence). Pass continues committed multi-turn abilities. Faster lethal hits cancel the defeated pet's queued attack. Cooldowns tick for all three pets. Ending occurs when a team has no living pets; both dead teams draw.

Aura states are added/removed without altering their source metadata. Single-turn DoTs tick N times, including the application round. Multi-stage Curse of Doom218 retains its final turn5 despite duration4: four waiting rounds, then impact. Positive periodic63 Wish273 follows the current active team pet when it heals; other admitted auras bind to a pet. Metabolic Boost194 refresh does not duplicate speed state. Control immunity uses Critter's broader root protection separately from Resilient's stun/sleep/polymorph protection. Time Stop600 removes aura927 and invokes event5 to grant Resilient924.

Nine families have CPU behavioral coverage: Beast, Flying, Critter, Magic, Mechanical, Undead, Humanoid, Aquatic and Dragonkin. Humanoid heal percentage comes from Recovery726/property53, not a new constant. Elemental state is retained but negative-weather immunity cannot execute until weather support exists.

### Limits

12 property handlers, parameter/graph admission gates,23 focused tests. Coverage count is **not** all-row behavioral validation. Unsupported92 IDs, object/team damage auras (Cyclone190), weather, conditional opcodes, target overrides, max-health/special states and proc events1..4/9/12 fail explicitly. Control-duration ordering, exact damage rounding/weak game-table factor and several proc timings remain uncaptured. No wire/UI integration or full Retail parity claim.

## Sources

- [Contract](../../specs/pet-battle-engine.md), [104-property coverage and reproduction](../../pet-battle-effect-coverage.md).
- Local-CASC DB2 build12.1.0.69933 in game-engine/data/db2; no Blizzard CDN/table downloads.
- Local Retail `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/Blizzard_FrameXML/SharedPetBattleTemplates.lua` AttackBonus/StandardDamage/StandardHealing; `Blizzard_PetBattleUI/Shared/Blizzard_PetBattleUI.lua` speed hints/ability cooldown presentation.
- TrinityCore a352b1fa `src/server/game/BattlePets/BattlePetMgr.cpp`, CalculateStats.
- Blizzard news10055295, **Pet Battle Accuracy Changes in Patch5.3**, official article/search-indexed archived text.
- LegionCore snapshot9222216e0dda0708ae53e6e6f6b07db7d65ba46f `PetBattleAbilityEffect.cpp` opcode/implicit-target names; `PetBattle.cpp` ties; legacy reference only, not copied implementation or Retail proof. Saved in game-engine/data/diagnostics/petbattle-engine-20261010.

## See Also

- [[world-data]] — static SQLite boundary.
- [[toys]] — account collections remain outside this turn library.
