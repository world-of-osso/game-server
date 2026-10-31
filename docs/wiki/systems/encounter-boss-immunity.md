# Encounter boss immunity

A creature whose `(map, entry)` is in `MapStore::raid_creature_encounters` is a DungeonEncounter boss. That is the same binding that drives encounter state and credit ([maps/raid.rs](../../../crates/server-combat/src/maps/raid.rs)). Such a creature is immune to these mechanics:

| Mechanic | ID |
|---|---|
| disoriented | 2 |
| fear | 5 |
| root | 7 |
| sleep | 10 |
| stun | 12 |
| freeze | 13 |
| knockout | 14 |
| polymorph | 17 |
| sapped | 30 |

The rule is in `auras/mechanic_immunity.rs` (`ENCOUNTER_BOSS_MECHANICS`). It applies at all three immunity gates:
- the whole-spell category mechanic, in the hit roll
- harmful effect hits
- aura effect retention

A spell with `SPELL_ATTR0_NO_IMMUNITIES` still bypasses the rule.

## Source (2026-10-31)

Our data has no immunity source for these bosses:
- TDB1210 `CreatureImmunitiesId`, `flags_extra` and `StaticFlags` are 0 for all 109 follower-dungeon bosses.
- No 70448 CreatureDifficulty DB2 row exists for them.
- TrinityCore 15e0ed76 grants mechanic immunity only from CreatureImmunities rows and auras. `IsDungeonBoss()`/`isWorldBoss()` gate only respawn, charm AI, threat and a player-caster knockback skip (`SpellEffects.cpp:4077`).

The rule therefore comes from retail combat logs. Counts are `applydebuff` events on enemies, from public Warcraft Logs Mythic+ runs:

| Run | Report, fight |
|---|---|
| Windrunner Spire (Midnight S1) | N8TLdfacxCbH1yr9 1 |
| Operation: Floodgate (TWW S3) | CPzKQd3xLrWJgRNw 1 |
| Ara-Kara (TWW S1) | VX4wfWcRajdbkGKN 2 |
| Nexus-Point Xenas (Midnight S1) | frjtBDLW2AdnHQk4 9 |
| Priory (TWW S3) | k3p64B1jA8Fv9rGX 26 |
| Eco-Dome Al'dani (TWW S3) | d4GDr83TPxYCVcX6 1 |
| The Dawnbreaker (TWW S3) | x3HzDRn6yVjqCbt8 7 |
| Ara-Kara (TWW S3) | XCmNaPcWdFA4kJ8D 9 |
| Magisters' Terrace (Midnight S1) | QTKp4CaWZydRjrwA 3 |
| Maisara Caverns (Midnight S1) | 3nTxa7fYVZ2yFDgJ 20 |

"Boss" here means an actor whose game ID is in `raid_encounter_creature`.

| Mechanic | Spells sampled | Trash | Adds in boss pulls | Bosses |
|---|---|---|---|---|
| stun 12 | Leg Sweep, Shockwave, Capacitor Totem, Chaos Nova, Terror of the Skies, Storm Bolt, Binding Shot, Shield Charge 385954, … | 1058 | 256 | 0 |
| incapacitate 14/13/17/10/30 | Incapacitating Roar, Paralysis, Freezing Trap, Hex, Hibernate, Sleep Walk, Imprison, … | 506 | 44 | 0 |
| root 7 | Earthgrab, Entangling Roots | 37 | 45 | 0 |
| disorient 2 | Blinding Sleet | 77 | 3 | 0 |
| fear 5 | Psychic Scream, Sigil of Misery | 6 | 10 | 0 |

The same bosses did receive these mechanics:
- snare 11: Chains of Ice 444826 226, Enfeeble 139, Remorseless Winter 100, Moonfire 58, Frostbolt 40, Consecration 37, Typhoon 7, Thunderstorm 7, Ursol's Vortex 6, Dazed 3, Earthbind 1.
- bleed 15
- taunt 36

One snare did not land: Thunder Clap 435203 (aura 33, mechanic 11) had 2435 applications on trash and adds and none on a boss. Other snares do land on bosses, so this exclusion is specific to that spell. It is not modeled.

No silence aura landed on any unit in the sample, so silence stays outside the rule.

Knockback and grip leave no aura, so the logs cannot measure them. They stay outside the rule.

Scripts and raw output: `/home/osso/.worktrees/logs/wcl-boss-cc/` (`analyze.py`, `boss_debuffs.py`, `results.json`, `boss-debuffs.json`).

Live trigger: in the class-party Halls of Infusion clear on 2026-10-31, the Prot Warrior's Shield Charge stunned Khajin 189727.
