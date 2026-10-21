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

## Proof

- [ ] Native behavior regressions: `follower_utility_policy` filter, RED before implementation, GREEN after.
- [ ] Existing kick/control/dispel/CC focused regressions.
- [ ] Detached `origin/dungeon-simulator` seeded comparison: exact code revisions, seeds, landed interrupts, dangerous cast completions, party deaths; absent mechanics/encounters stay explicit rather than inferred from Stockade.
