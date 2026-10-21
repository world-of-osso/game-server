# Follower utility

## Contract

All eleven `follower_profiles` use only their level-available selected70448/server spellbooks. Damage/heal rotations remain separate. Every timing/recipient choice below is **FOLLOWER_UTILITY_POLICY**, not a claim about Retail follower AI. [Existing follower contract](follower-dungeons.md).

- [ ] Interrupt every engaged party caster, sorted by danger then remaining cast time. Danger order: loss-of-control, healing/resurrection, damage, other. Use native effect/aura metadata, including authored trigger children. Rotate the first opportunity across living followers after each accepted kick. Reserve the exact admitted cast until it ends/changes; never double-spend on a cast awaiting impact. Ordinary range/LOS/resource/cooldown/immunity checks own acceptance.
- [ ] Stun/incapacitate a casting enemy only after the party's available ordinary kicks cannot cover it. Never chase for control. Native control immunity/creature-type rules remain authoritative. Root-only utility is inventoried, not misrepresented as stopping a ranged cast.
- [ ] Dispel only actual eligible native types/harmfulness. Rank removable loss-of-control before periodic harm before other removable debuffs; health then entity identity break ties. Friendly purges/hostile cleanses are forbidden by ordinary effect eligibility.
- [ ] Maintain learned raid buffs only outside combat. Native spell targeting distributes group buffs; missing party recipients are considered, not only the caster. Do not spam group buffs while an application is pending.
- [ ] Use personal/external defensives when observed net health decline projects death inside3seconds. No invented spell coefficients or native mitigation overrides. Unchanged-health/no-history is not an invented incoming damage rate. Avoid a second defensive while an effective personal defensive is present.
- [ ] Lust once on an engaged native dungeon/world boss pull, coordinating party casts and respecting native exhaustion/sated effects/cooldowns. Do not grant Bloodlust/Heroism when absent from the selected spellbook. Ordinary casts own their aura effects.
- [ ] Combat resurrection prioritizes a dead healer, then dead tank, in the same party/map. Respect native casting time/cost/cooldown and reserve the corpse during an admitted resurrection. Follower corpses auto-accept; player recipients retain real offers. DPS corpses are not battle-res targets under this policy.
- [ ] Use learned pursuit speed/gap closers only while an engaged enemy lies beyond normal attack range, on the existing safe/admitted pursuit. Never replace route admission or charge into a protected target. Native motion owns displacement; the scheduler never teleports actors.
- [ ] Never break someone else's breakable CC: preserve existing hostile cast/swing cancellation and area-footprint rejection. Do not cancel missiles already in flight. DPS/healer taunts remain disabled, including Feral/Restoration Growl and Windwalker Provoke.

## Data and exclusions

Census: `game-engine/data/diagnostics/follower-utility-2026-10-21/census.md`, reproducible with `scripts/follower_utility_census.py`; all level intervals and direct effect facts retained in `census.json`. Pet interrupt uses the actual summoned Felguard's learned Axe Toss, never a player spell grant. Hunter pet actions require a real pet; no guessed pet/family/stable selection. Travel, forms, portals, ritual/item creation and other manually chosen abilities are inventoried but not made autonomous merely because they are non-rotation spells.

## Player-race follower model geometry

`FOLLOWER_PLAYER_RACE_MODEL_REACH_TC_PLAYER_DEFAULTS` is explicit project policy. A unit must be an actual follower and its70448 `CreatureDisplayInfo.ExtendedDisplayInfoID` must be nonzero. Only then use TC a352b1fa `src/server/game/Entities/Object/ObjectDefines.h:39-40`: `DEFAULT_PLAYER_BOUNDING_RADIUS = 0.388999998569489f`, `DEFAULT_PLAYER_COMBAT_REACH = 1.5f`, multiplied by the unit's scale. TC's player scale path applies these constants; applying them to these follower character models is the named project decision, not an invented TDB row. This rule takes precedence over creature-model sizing for qualified followers. Ordinary NPCs and unqualified/unknown follower displays still require exact authored geometry; no guessed1.5 exception.

Bo's real first-fight pursuit/attack regression must pass in the ordinary2D gameplay app. The policy belongs to the production model lookup and is identical in baseline/candidate. All five missing source displays116316,121275,121277,121289,122612 have their nonzero ExtendedDisplayInfoID independently matched to the local70448 CSV; receipt `geometry-policy-source.json` records hashes and exact rows. `scripts/follower_model_geometry.py` separately emits native missing-used-display SQL and unresolved-source receipts, without modifying canonical world.db or publishing policy constants as source rows.

## Named native-mechanic gaps

- `SHUJA_LUST_NOT_LEARNED_70448`: neither Bloodlust nor Heroism is in the selected Elemental loadout. Time Warp/Fury remain available to the learned Mage/Evoker.
- `SOULSTONE_CORPSE_CHILD_UNLINKED_70448`:20707 triggers dummy6203, not resurrect95750. The utility layer does not invent that missing script link or claim Soulstone battle-res support.
- `DUMMY_PURSUIT_MOTION_UNIMPLEMENTED`: Divine Steed190784, Charge100 and Shield Charge385952 have no executed native motion in the current ordinary backend. Steed's description references221883; Charge100 can still grant its native Rage. Do not count accepted casts as movement or invent wrapper-child links.
- Fear5782 is also dummy-only in70448; direct metadata cannot classify it as a stopping aura. Native118,853 and learned stopping controls provide the actual control/danger fixtures. Inventory does not fill missing spell mechanics by name.

## Proof

- [ ] Native behavior regressions: `follower_utility_policy` filter, RED before implementation, GREEN after.
- [ ] Existing kick/control/dispel/CC focused regressions.
- [ ] Detached `origin/dungeon-simulator` seeded comparison: exact code revisions, seeds, landed interrupts, dangerous cast completions, party deaths; absent mechanics/encounters stay explicit rather than inferred from Stockade.
