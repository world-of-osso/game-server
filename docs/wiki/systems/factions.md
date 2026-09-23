# Factions and Reactions

Every unit carries a Retail `FactionTemplate` ID in the replicated `UnitFactionTemplate(u32)`. The server derives unit reactions from the DB2 `faction_template` table and uses them for spell target relations ([[spell-casting]]) and auto-attack eligibility ([[combat]]). Code: `crates/server/src/faction.rs`.

## Sources of the template ID

| Unit | Source | Set in |
|---|---|---|
| NPC | AzerothCore `creature_template.faction` (a FactionTemplate ID) | `zones.rs` spawn |
| Player | `ChrRaces.FactionID`, Retail build 12.1.0.69933 (const table `faction_template_for_race`; world.db has no `chr_races` table) | `auth_character.rs` world entry |

- `faction_template` is imported by `scripts/db2.py` (table `FactionTemplate`) at build 12.1.0.69933 and loaded into the `FactionTemplates` resource by `WorldDataPlugin`.
- Example player templates: Human 1, Orc 2, Dwarf 3, Night Elf 4, Undead 5, Tauren 6, Gnome 115, Troll 116, Blood Elf 1610, Draenei 1629, Worgen 2203, Goblin 2204. Unknown race = 0.

### Wrath-era IDs vs Retail

AzerothCore faction values are WotLK FactionTemplate IDs. Measured against Retail `faction_template` (2026-09-23): all 657 distinct `creature_template.faction` values resolve, so 0 of 29,947 templates and 0 of 149,678 spawns are unresolved. Existence is measured; that each Retail row still means the same thing as the Wrath row is not.

## Reaction rules

`FactionTemplates::reaction(a, b)` = how a unit with template `a` regards one with template `b`. These are the template checks of AzerothCore `Unit::GetFactionReactionTo`, in order:

1. A missing or unknown template on either side: **Neutral**.
2. Same template: **Friendly**.
3. `a` hostile to `b`: `b.Faction` in `a.Enemies`, else (not in `a.Friend`) `a.EnemyGroup & b.FactionGroup != 0`: **Hostile**.
4. `a` friendly to `b` or `b` friendly to `a`: same `Faction`; or `Faction` in `Friend` (and not in `Enemies`); or `FriendGroup & other.FactionGroup`; or `FactionGroup & other.FriendGroup`: **Friendly**.
5. `a.Flags & 0x2000` (HATES_ALL_EXCEPT_FRIENDS): **Hostile**.
6. Otherwise **Neutral**.

Group bits: 1 player, 2 Alliance, 4 Horde, 8 monster.

Not modelled: player reputation (AzerothCore returns the player's reputation rank for factions that can have reputation, capped at Neutral when at war), forced reactions, contested guards. So **Unfriendly** is never produced and `Reaction` has only Hostile / Neutral / Friendly.

### Examples (Retail rows)

| Template | Faction | FactionGroup | FriendGroup | EnemyGroup |
|---|---|---|---|---|
| 1 Human player | 1 | 3 | 2 | 12 |
| 2 Orc player | 2 | 5 | 4 | 10 |
| 11 Stormwind City Guard | 72 | 3 | 2 | 12 |
| 25 Kobold Vermin (Northshire) | 25 | 8 | 0 | 0 |
| 31 Rabbit, Deer, Squirrel (critters) | 28 | 0 | 0 | 0 |

- Stormwind guard vs Human player: guard `EnemyGroup 12 & 3 = 0`, `FriendGroup 2 & 3 ≠ 0` → **Friendly** (both directions).
- Stormwind guard vs Orc player: `EnemyGroup 12 & 5 = 4` → **Hostile** (both directions).
- Human player vs Northshire kobold: Human `EnemyGroup 12 & 8 ≠ 0` → **Hostile**. The kobold's own view of the player is Neutral (no enemy mask).
- Human player vs critter: no group overlap, no hostile flag → **Neutral** (both directions).
- Human player vs Orc player: **Hostile**.

## Attack and assist

| Check | Rule | Used by |
|---|---|---|
| `can_attack(a, b)` | neither side Friendly toward the other; creature vs creature needs one side Hostile; player vs player is rejected (no PvP flag or duel model) | spell `Enemy` relation, auto-attack start |
| `can_assist(a, b)` | `reaction(a, b)` is Friendly; a unit always assists itself | spell `Ally` relation |
| `is_hostile(a, b)` | `reaction(a, b)` is Hostile | hostile vs friendly spell range |

- `can_attack` follows AzerothCore `_IsValidAttackTarget`'s faction part, so players can attack neutral critters.
- `can_assist` stands in for `_IsValidAssistTarget`'s reaction check plus its PvC rule, which needs creature `type_flags` (CAN_ASSIST) that are not imported.
- A friendly spell whose explicit target fails `can_assist` self-casts (for example Flash Heal with a hostile kobold targeted heals the caster).
- Auto-attack: `start_attack_system` only activates `AutoAttack` when `can_attack` holds. Targeting a friendly guard therefore selects it without swinging or entering combat.
- Units without `UnitFactionTemplate` use template 0 (Neutral to everyone).

## Sources

- AzerothCore `Unit::GetFactionReactionTo`, `_IsValidAttackTarget`, `_IsValidAssistTarget` (`src/server/game/Entities/Unit/Unit.cpp`), `FactionTemplateEntry` (`DBCStructure.h`), flag values (`DBCEnums.h`).
- wago.tools DB2 `FactionTemplate` and `ChrRaces`, build 12.1.0.69933.

## See Also

- [[spell-casting]] — enemy/ally relations for explicit and area targets
- [[combat]] — auto-attack
- [[world-data]] — world.db tables
