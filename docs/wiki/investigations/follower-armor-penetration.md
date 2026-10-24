# Follower armor penetration

Verified2026-10-23. The first real WW269 Tiger Palm factor receipt exposed an actor-policy difference, not a generic armor/K discrepancy. The lead approved `FOLLOWER_CLASS_ACTOR_PLAYER_EQUIVALENT_ARMOR_PENETRATION`: player-affected class followers retain native aura280 armor penetration; ordinary NPCs do not. This is a project bridge policy, not a claim that TC treats NPC followers as players.

## Source and inputs

| Input | Native evidence |
|---|---|
| Dummy31146 | Actual fixture native/effective level93, armor1470; authored template class1, ArmorModifier1. |
| ExpectedStat | Installed70448 CSV and frozen world rows475/478, levels90/93, ExpansionID−2: CreatureArmor1470, ArmorConstant3430. Warrior ExpectedStatMod4 armor/K multipliers are1. |
| Caster | Actual level90 class follower, not a synthetic Player. |
| Martial Precision450990 | SpellEffect1151272, zero-based effect0, aura280, amount12. Actual pre-cast aura receipt contains that effect. |

TC a352b1fa `Unit::CalcArmorReducedDamage` applies rating/aura280 penetration only to `TYPEID_PLAYER`. It caps the penetrable armor by victim level, then reduces damage by `min(armor/(armor+K),0.85)` with integer truncation. Native `auras_changed` previously zeroed280 for every non-Player, including class followers. SimC's player reads450990 as12% armor penetration: its effective armor is1293.6 while K remains3430.

For this high-level victim the penetration cap includes all1470 armor. The approved follower policy therefore expects1293.6/(1293.6+3430)=0.2738589212 mitigation. The old actual native hit observed0.3000110388; the extra0.000011 is damage truncation in the measured stage ratio. Do not change base armor, K or spell coefficients to close this difference.

## Proof ledger

Artifacts: `game-engine/data/diagnostics/native-spell-diff-2026-10-23/armor-source-qualification/`.

- `expected-stat.proof.json`: frozen database/CSV/source hashes, exact ExpectedStat/SpellEffect rows and actual target/aura snapshots. Fresh TC Unit.cpp matches the earlier pinned artifact byte-for-byte, SHA256 `5aee33e5fad0132bb1905ea6d1c330806374457b15f918d89e0ce5d60353f981`.
- `follower-armor-policy.receipt-red.log`: real scope84d60394e Tiger Palm capture fails the independent policy expectation, exit1; no injected aura/resource values.
- At a9a17aa4c the real policy cast passed the mitigation expectation: observed rounded-stage fraction0.2738695145 and damage39471. The test then correctly failed because the observer still printed pre280 armor1470; native policy damage was not the failure.
- Three280 controls passed on that pinned binary: ordinary player-controlled NPC remains unaffected, Player Martial Precision removal restores damage, and100% penetration retains the TC cap. Runtime13.55s; `native-armor-policy-controls.proof.json` records the binary hash.
- Observer correction returns actual post-penetration armor/K from `combat::armor_mitigated` and the actual mitigation scalar from `melee::armor_reduced_damage`. No formula is copied or recalculated. The old rounded stage ratio remains separately named `native_observed_armor_fraction`; `armor_fraction` is the actual formula variable. At998083262 observer/native cases3GREEN35.07s,280 controls3GREEN8.19s, armor math1GREEN and forecast8GREEN23.45s. Actual armor1293.599976/K3430/fraction0.273858905 matches SimC; damage stays39471 exactly across observation changes. Current artifacts/diffs live in `green-998083262/`; the earlier zero-match `combat_forecast::` probe is not accepted evidence.

The earlier per-hit capture schema itself passed1/1 and preserved damage38050 exactly from its RED baseline. That is not armor-policy GREEN or DPS parity. Arms MS, Assassination MH, Fury MH and BM melee oracle receipts all show1470/K3430/0.30; the WW0.274 observation must not be generalized to every physical spec.

## Separate unqualified TC branch

TC also adjusts K for max-level players/player-owned units using equipped-base item-level delta and `GlobalCurve::ArmorItemLevelDiminishing`18. Installed70448 GlobalCurve19 maps to curve27400, whose points rise above1 after delta17. Native `ArmorConstants` currently has no item-level branch. This is not the observed12% follower difference. The relevant player/owner item-level input and a quantitative native counterexample are not qualified here; no curve or K tuning is included in this policy change.

## Sources

- [TC pinned Unit.cpp](https://github.com/TrinityCore/TrinityCore/blob/a352b1fa9017138cc2f1dcfc0a51afaa17391ecf/src/server/game/Entities/Unit/Unit.cpp#L1689-L1778) — source boundary and armor calculation.
- [Native aura recomputation](../../../crates/server/src/auras/mod.rs), [armor pipeline](../../../crates/server/src/combat.rs), [ExpectedStat producer](../../../crates/server/src/level_scaling.rs).
- [Per-cast regression](../../../crates/server-simulation/src/dungeon_simulator/dps_parity/single_spell.rs), [ordinary-NPC control](../../../crates/server/src/spell_cast/aura_stat_spell_tests.rs).
- [Follower contract](../../specs/follower-dungeons.md).

## See Also

- [[simc-spell-oracle]] — compare actual formula stages and actor semantics before changing mechanics.
- [[follower-data]] — class-stat bridge source limits.
- [[combat]] — generic physical damage path.
