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

## Turn contract

`Battle::step` returns new state plus ordered events. RNG draws uniformly0..999999; no global seed/time. Speed decides order; exact ties roll50/50. Voluntary swaps precede attacks. Pass continues a committed multi-turn move. Faint replacement is an explicit caller decision, not an automatic strongest-pet selection. Forfeit ends immediately; both forfeits draw.

Supported dispatch:22(zero-point dummy),23(heal),24(damage),26(positive aura),31(set caster state),49(remove named caster aura),50(negative aura),52(control),53(percent heal),54(periodic aura),63(positive periodic aura),222(variance damage). Parameter combinations and referenced aura graphs are admitted before execution; presence in this list does not mean every effect row is admitted. Proc events0/5/6/7/8/10/11 handled; events1..4/9/12 and weather excluded pending causal evidence. Rejected abilities return explicit errors.

Family passives use state names from BattlePetState and current family descriptions236..245: Beast25% belowhalf; Flying50% speed abovehalf; Critter control immunity; Magic35% hit cap; Mechanical20% once; Undead one immortal following round with25% damage penalty; Humanoid Recovery726 percentage; Dragon next-round50% on crossing enemyhalf; Aquatic incoming periodichalf/outgoing+25%. Elemental weather immunity cannot be exercised until weather handlers exist.

Power coefficient.05 and variance range `variance/200` are Retail `Blizzard_FrameXML/SharedPetBattleTemplates.lua` lines490..502. Base5% critical, critical1.5x, and base+/-10% variance are Blizzard's **Pet Battle Accuracy Changes in Patch5.3** (news10055295); the older Legion emulator instead uses2x crit and is not followed. Current DB2 variance parameters override the base spread. Full Retail runtime observation remains a separate proof level.
