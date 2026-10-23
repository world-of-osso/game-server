# Windwalker / generic weapon AP proof ledger

## Active goal and exclusions

Spec269 native spells and selected talents, literal SimC76f8118 APL, matched BiS300s ST/3T within ±10%, one native Stockade WW receipt. Parent session01a11b35 owns integration gates. First dependency is generic weapon AP, assigned here for all specs. No canonical database writes, Player AssistedCombat edits, other owners' worktrees/targets, tuning, or duplicated native-book helper.

## Source boundary

Pinned public TC a352b1fa tree: `/home/osso/.worktrees/logs/achievements-criteria-2026-10-12/tc-source-a352b1fa/TrinityCore-a352b1fa/` (read-only).

- `src/server/game/Entities/Player/Player.cpp:8180–8191`: equipped weapon AP is `int32(ItemTemplate::GetDPS(itemLevel) * 6.0f)`, per attack slot; use table DPS before swing-range rounding.
- `src/server/game/Entities/Unit/Unit.cpp:9973–10000`: main/ranged use max(main,ranged weapon AP), offhand adds offhand AP then halves the sum; clamp negative total, then apply the respective AP multiplier.
- `Unit.cpp:6869–6891`: damage AP coefficient uses ranged attack type for a non-melee ranged weapon spell, offhand for exclusive offhand attribute, main otherwise. `7388–7410` and `7662–7684` repeat this selection for healing/absorbs.
- Current `effects::effect_power_bonus` uses only bare `CharacterStats.attack_power`/`ranged_attack_power`. Gear computes rounded swing bounds but does not retain native weapon AP.

## Explicit deferred finding: white-hit AP divisor

TC `StatSystem.cpp:455` computes player weapon damage with `GetTotalAttackPowerValue(attType, false) / 3.5f * attackPowerMod`; its creature/guardian paths at1140/1421 also use3.5. SimC76f8118 `engine/action/attack.hpp:13` defines `WEAPON_POWER_COEFFICIENT=6`; `attack.cpp:29` sets its inverse, and `action.cpp:1438` multiplies AP by that inverse and weapon speed. Current server `melee::weapon_damage_range` also uses AP/6. **Parent explicitly deferred this discrepancy; do not change it in the weapon-AP coefficient fix.**

## Proof ledger

| Revision / scope | Command | Result | Validity |
|---|---|---|---|
| `0cef21dfc`, ignored RED only | native `dps_parity_native_tiger_palm_includes_equipped_weapon_ap_before_battle_shout` | Not run: formal capacity HOLD/FIFO | No behavioral RED claim |

RED uses actual native weapon268215, table DPS at its authored level, real Tiger Palm100780 coefficient, native Battle Shout6673 and actual cast/damage log. No synthetic coefficient, weapon range, damage override or database edit. RED committed and pushed before production implementation.

## Historical receipt caveat

Frozen indexv5 reports64,175.72 ST vs SimC206,443.72, and35,050.52 three-target vs376,559.68. The shared269.native.1.json has since been overwritten and currently contains only Tiger Palm/Blackout Kick/Zenith; it cannot substantiate the old decomposition's FoF/RSK cast counts. Preserve new WW receipts under a distinct immutable scope. Historical FoF25→5 cadence policy is already in master; its shared-process flake is not the generic AP cause.
