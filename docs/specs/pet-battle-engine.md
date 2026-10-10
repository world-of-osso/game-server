# Pet battle turn engine

Pure deterministic server library; no transport, encounter spawning, UI or journal mutations. Three pets per team, one active pet; player decisions are ability, swap, pass and forfeit. Randomness is injected. Unsupported DB2 properties and event combinations must fail explicitly rather than imitate direct damage.

## Data boundary

`Catalog::load_world_db` reads these SQLite tables with original DB2 column names and integer numeric affinity (StateMultiplier REAL; descriptions, LuaName, ParamLabel_* TEXT):

| Export | SQLite table |
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

Runtime has no CSV fallback. Tests import authenticated local-CASC build12.1.0.69933 CSVs into in-memory SQLite; `PET_BATTLE_DB2` specifies export directory. No data committed. pets-server-fix currently provides species metadata only; battle-table producer integration is a later dependency.

## Stats contract

Species + breed raw stamina/power/speed states multiplied by quality and level. Max health `round(stamina*quality*level/20)+100`; power/speed `round(raw*quality*level/100)`. Quality multipliers come from DB2. Species39/breed3/Rare/25 yields1563/260/276. Divisors20/100 and health offset100 are runtime constants, **not derivable from DB2 alone**: TrinityCore pinned a352b1fa `src/server/game/BattlePets/BattlePetMgr.cpp`, CalculateStats. Its rounding comment retains an upstream uncertainty; no claim of captured Retail numeric proof.

## Acceptance

Behavior tests must use real ability IDs and hand-derived numbers for damage strong/weak/neutral, aura expiry, control turn loss, speed, passives, multi-turn and cooldown decisions. Coverage reports distinguish handler availability from every-row validation. Full104-property parity is not implied by a handler for a frequently used opcode.
