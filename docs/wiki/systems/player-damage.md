# Player Damage

How a player's auto-attacks and spells turn stats into damage, in the order TrinityCore master a352b1fa applies each step. The requirements and tests are in the [player damage spec](../../specs/player-damage.md).

## Auto-attack pipeline

`combat.rs` `auto_attack_system` ticks both hands of every attacker's `AutoAttack` (`timer`, `off_hand_timer`) and calls `try_swing` for the main hand, then the off hand. An `Attacker` with `CharacterStats` is a player; without it (creatures) the swing keeps the creature rules of [[level-scaling]].

1. **Weapon:** `CharacterStats.main_hand` (unarmed 1-2 at 2.0 s) or `off_hand` (none: no off-hand swing). Both come from `EquippedItems` through `item_stats.rs` ([[character-stats]]).
2. **Roll:** `melee::weapon_damage_range` = (min or max + AP / 6 × speed) × (1 + versatility), × 0.5 for the off hand. `melee::roll_weapon_damage` picks a whole number in the truncated bounds (Unit::CalculateDamage `urand`).
3. **Done/taken:** × aura physical damage done × MOD_AUTOATTACK_DAMAGE × target physical damage taken (`AuraStatMods`), rounded.
4. **Armor:** `combat::armor_mitigated` against `target_armor`: a creature's `CreatureArmor` through `UnitScaling::armor_against` (its ExpectedStat armor at its level for the attacker), a player's `CharacterStats.armor`. The constant is `ArmorConstants.at(attacker level against the target)`. Without the `ArmorConstants` resource (unit tests) there is no mitigation.
5. **Outcome:** `melee::swing_chances` with `combat::roll_outcome` applying cached NPC victim NO_PARRY/NO_BLOCK and victim facing/288, then `roll_melee_outcome` (one roll in 0..10000, chances truncated to hundredths), then `outcome_damage`: avoidance 0, crit × 2, glancing −10%/level, block −30% (the blocked amount goes to the combat log).
6. **Level scaling:** `damage_dealt` (a tuned creature attacker's multiplier) and `health_loss` (a tuned creature target's health ratio), unchanged from [[level-scaling]].
7. **Timers:** the swung hand restarts at weapon speed / (1 + haste); the other hand is pushed to at least 0.2 s.

Creatures use the ExpectedStat roll (tuned) or10 every2s instead of the player weapon/AP roll. Their existing hit table shares the victim flag filtering; armor also applies to creature swings.

Avoided swings emit `CombatEventType::Miss | Dodge | Parry` and `CombatLogKind::Miss(..)` with amount 0, add 0 threat, still give swing rage and raise no proc events.

## NPC avoidance flags — 2026-10-18

`zones::CreatureFlagsExtra(u32)` is server-only source template metadata. `EntryProjection::apply_core` copies it while the template is already present, covering ordinary spawn, world summons and same-actor entry projection. Spell summons carry the same real field through private `SummonTemplate`; controlled pets copy it through their existing native spawn-metadata tuple. Neither adds a lookup. `combat::roll_outcome` reads that component and zeroes only parry for0x4 and block for0x10 on creature victims; there is no per-swing WorldData lookup, wire change or extra random draw. Missing metadata is not a query filter: the NPC retains its existing avoidance and remains attackable, with an explicit error queued once per entity. A private diagnostic marker deduplicates even multiple attackers in one deferred batch. This compatibility behavior is explicitly requested for manually constructed NPCs; normal spawns always cache the source flags.

Pinned TC a352b1fa Unit.cpp2378–2488/2749–2861 and CreatureData.h346–348 supply the two masks. Runtime IsTotem, NO_CRIT/NO_CRUSHING and other flags remain named gaps; facing is covered below. No NO_DODGE or SUNWELL flag is invented. Genuinecfb1638ac RED uses actual native Ice Tomb36980/level80 and256 ordinary wire-observed swings, seeing15Parry/7Block; unflagged native Grandmaster Dummy31144 control passes. Probability of missing both outcomes at6%+3% is0.91^256=3.27037e-11; corrected masks forbid them deterministically. Initialc7d19 fixture panic omitted actual class-level authority and is NOT feature RED. Full read-only native WorldData fixes that setup, without changing rows, levels, HP/stat values, clocks or RNG. Atb3b6e49a5 the 256-swing flagged/unflagged pair passes, while actual Spirit Wolf and ordinary Hunter Call Pet expose separate genuine missing-cache REDs (None versus nativeSome0/Some2). Their two producer paths now insert the real source field without new lookups. The first ordinary-spawn cache test selected an absent Kobold6 placement and is NOT feature RED; it now uses an actual native Stormwind Infantry49869 placement and source ground data. Nonattackable battle-pet companions remain outside scope. At357c32137, all57 scoped native flags/cache/producer, player-defense, ordinary pet, Dummy12, AP and existing hit-table controls pass79.65s,0ignored/0warnings. Ordinary Stormwind Infantry, spell-summoned Spirit Wolves and actual called Hunter pet all carry their real template flags. The approved common fixture repair attaches actual native Kobold6 flags without replacing caller WorldData; no other manual fixtures required repair. At1be6c12b7 server-bin check37.50s/fmt2.38s pass without warnings; changed-Rust manual readability and diff-whitespace checks pass (metrics binary unavailable). Master41856b72b and own clean-FF protocol029a927 were integrated; lead subsequently confirms nf570 EXIT/check/fmt0 and landingcb55b98c6. Missing-cache once/entity logging and same-actor entry cache refresh have code/wiring proof, not separate integration REDs; source/receipts live in engine `data/diagnostics/class-coverage-b-2026-10-17/melee-npc-flags-*` and `npc-auto-flags-*`.

## Melee front arc and direction override — 2026-10-18

TC a352b1fa Unit.cpp2404–2414 evaluates victim facing on every melee roll. `combat::roll_outcome` now clears parry/block behind any victim, and dodge only behind a player. Actual aura288 treats every direction as front, but does not override source NPC NO_PARRY/NO_BLOCK or the existing casting/CC exclusion. No extra RNG draw, clock/animation change or new state/wire field is introduced.

The shared pure `targets::in_front_arc` retains the prior incoming-spell dot-product boundary (`>=0`) and its WoW forward vector. `targets::orientation_from_rotation` retains Rotation.y minus pi/2, with the existing missing-rotation0 value. Both `hit.rs` and combat use these helpers; spell-defense semantics are unchanged.

Ordered ignored REDdeb2168f0 was pushed before product edits. Explicit ignored execution gives genuine2FAIL/4PASS31.66s: behind unflagged native31144/equal80 emits9Parry/11Block across256 ordinary attempts, while actual Rogue Evasion5277 (200dodge,10000ms) logs5behindDodge events. Corresponding front and real97574(self/permanent/single288) NPC/player controls pass before the consumer. NPC dodge remains an explicit behind control; its256-attempt no-Dodge probability0.97^256≈4.11e-4 stays below1/1000. Five Evasion attempts fit its unchanged native lifetime, with all-Miss probability0.05^5=3.125e-7. No HP/stat/row/flags/timer/RNG rewrites. At64483bb3a explicit ignored execution passes all6 native cases37.81s,0warnings; only after that proof the six ignores are removed. Merged17f061dc5/master41e386c4d ordinary discovery/affected controls produce109PASS/2FAIL/3ignored78.29s, including all6 new cases. Both failures are old front-assumption fixture setups: NPC-flags native targets atx3/orientation0 faced away from caster0, and the player-defense helper atx1 had the same default facing. Only those two helpers gain explicit front orientation; mask/CC assertions, stats, flags, RNG and sample counts are unchanged. At5b214cc4f corrected front-control modules pass7/7 in36.87s,0warnings. Retain104 disjoint passes from17f061dc5:111 scoped executable cases GREEN as a composite, not a replayed111-case batch; three legacy LOS tests remain ignored. Later5322f4ab4/master748a5ad78 adds PVP-deserter port handling only, disjoint from the tested combat/hit/target/fixture paths. Final checks and independent lead gate remain pending. Receipts: engine `data/diagnostics/class-coverage-b-2026-10-17/melee-facing-*`.

## Autoattack attacker251 — 2026-10-17

TC a352b1fa Unit.cpp2389 uses `GetUnitDodgeChance` for autoswings;2750–2782 supplies the chance calculation, with exact2775 adding signed attacker `SPELL_AURA_MOD_ENEMY_DODGE` before the final zero clamp. This is not a victim modifier. Existing weapon-spell support remains unchanged.

The existing attacker `ActiveAuras` query supplies one transient signed251 scalar to `SwingSnapshot`/`Attacker`, releasing the attacker borrow before victim auras are mutated. `combat::roll_outcome` adds it only on the existing hit-table/allowed-avoidance path, then clamps zero; the existing behind-player gate still clears dodge afterward. Casting/CC, missing-player-stats behavior, parry/block, flags, native RNG order and timers remain unchanged. No wire or persistent component is added. Player expertise/248/runtime IsTotem/other flags remain separate gaps.

Ordered ignored REDd5587d263 was pushed first. Actual native execution gives2FAIL/4PASS87.19s, without fixture/compiler errors: native Sunwell45769 effect1=-20 still allows20Dodge in512 ordinary equal80 dummy swings; native NPC Risky Feint48250 effect2=+20 with Dust Cloud26072 effect0 giving90miss yields236Miss/20Damage/0Dodge in256 swings instead of only Miss/Dodge. Signed +20/-25 (Word of Sha110294 effect0), recipient-only Sunwell, behind-player and native853 stunned-player controls pass before the consumer. Samples were fixed before execution:512 for3% native dummy controls (.97^512≈1.69e-7 each),256 for90miss controls (.9^256≈1.93e-12). Effects are reapplied after actual swings without changing authored amounts, durations or swing timers. Player health is produced by native level80 stats, not overwritten. Sunwell is an explicit coefficient fixture, not legal loadout certification. At1dbe19063 explicit ignored execution passes all6 native cases110.87s (275.07s including compile),0warnings; only then all six ignores are removed. Merged e4d9c1efd/master7a0f00fc0 ordinary-discovery affected cohort passes122/122,3legacyLOSignored154.21s,0warnings, including all six newly enabled cases and original weapon-spell251 controls. No samples or authored effects changed after RED. Independent lead acceptance remains pending. Receipts: engine `data/diagnostics/class-coverage-b-2026-10-17/melee-aura251-*`; [contract](../../specs/player-damage.md).

Post-history-rewrite integration maps old035ae5cfb to de08aab30 with an identical tip tree; old receipts remain pre-rewrite evidence, not relabelled. Merge with masterac36572bb retains the Guardian Spirit per-attacker same-tick `try_swing` health commit and one-shot receipt;251 snapshot wiring moves into its `auto_attack_unit` construction while the same roll consumer remains. Original nine filter groups plus `guardian_spirit_native` and `ardent_defender` integration proof is pending.

## NPC autoattack expertise — 2026-10-17

TC a352b1fa Unit.cpp2389/2391 calls the dodge/parry chance functions;2777–2782/2817–2822 subtract signed non-player `MOD_EXPERTISE`240 total/4 before final zero clamps. `SwingSnapshot` reads one additional NPC-only scalar from existing attacker ActiveAuras; `auto_attack_unit` passes it to the same `roll_outcome`. Dodge combines251 before subtracting240; parry subtracts240, block is untouched. Existing masks, behind-player/CC/casting/missing-stats exclusions, one native roll and Guardian same-tick one-shot health commit remain unchanged. Player expertise/rating,248,IsTotem and other flags are not implemented by this slice.

Ignored2abb070d2 was pushed before any consumer. Native28305 effect1/SpellEffect17754=14 (zero scaling/coefficient, DurationIndex0) was qualified FIRST: one application remains14 after1536s without refresh,1PASS32.88s. The cold command completed and wrote native logs; a subsequent Pyrun terminal-method `.run()` wrapper error prevented an exit-code receipt. The test result is preserved, not relabelled exit0 or replayed just to recover logs. Native62627 effect2/56718 supplies10000 expertise permanently, with its actual100hit/other effects unchanged; real Rogue5277 supplies200dodge for10000ms. Actual62627-vs-Evasion five ordinary2s swings yield genuine5Dodge RED; cancellation/foreign controls pass. Two NPC-to-NPC cases initially observed no owner-only CombatLogEvent: neither has a player owner. These are fixture failures, NOT feature RED. Their fixture now subscribes the actual all-client CombatEvent broadcast without inventing NPC ownership or altering gameplay input.

Corrected ignoredb1c2d11f2 execution runs only those two cases, preserving disjoint qualification/Rogue/control evidence: native14 yields16Dodge/37Parry/13Block in512; native10000 yields14Dodge/26Parry/17Block in512, genuine2RED66.39s. Samples and native clocks/RNG remain unchanged. Native14/4=3.5 should remove3%Dodge while retaining2.5%Parry and3%Block; no-Parry chance .975^512≈2.346e-6 and no-Block .97^512≈1.687e-7, well below1/1000. Five Evasion attempts fit its unchanged lifetime; no-Dodge positive-control chance .05^5=3.125e-7. This is not legal loadout or full hit-table certification. Consumer925224b2a committed after genuine RED; explicit ignored native behavior5/5GREEN70.24s (265.77s including compile),0warnings. Retain disjoint pre-consumer runtime-lifetime1PASS32.88s: six native cases GREEN as composite proof, not a replayed6-case batch. All six ignores are removed only after that proof; lead integration gate pending. Receipts: engine `data/diagnostics/class-coverage-b-2026-10-17/melee-npc-expertise-*`; [contract](../../specs/player-damage.md).

## NPC victim dodge aura49 — 2026-10-17

TC a352b1fa Unit.cpp2389/2761–2762 adds signed victim `MOD_DODGE_PERCENT`49 to ordinary NPC dodge; level bonus and attacker251/240 precede the final clamp. `combat::roll_outcome` reads the already-present target_auras only for `Victim::Creature`, inside the existing can_avoid/missing-stats guard. Player49 stays in CharacterStats, not double-applied. No query, snapshot field, state or wire addition; parry/block, masks, NPC behind dodge, native RNG/clocks and Guardian same-tick health commit remain unchanged. Runtime IsTotem, NPCparry47/block51,248/player expertise remain separate gaps.

IgnoredREDdf529041d was pushed FIRST. Actual native5277 effect0/1892=+200/10s and62356 effect1/56357=-200/20s were qualified before the signed behavior case: both remain live with summed49=0 across8s, block51=0 (effect0 excluded),1PASS33.62s/exit0,0warnings. Native unarmed Default speed2s is unchanged; five attempts fit native Evasion10s. Real behavior execution gives2genuineFAIL/4PASS35.27s: front and behind native31144/equal80 each log5Damage/0Dodge while actual victim200 is live. Cancellation/natural expiry, foreignNPC, qualified signed-zero and already-landed NPC62627 expertise ordering controls pass. No fixture/compiler failures count as RED; no rows/HP/stats/RNG/duration/timer edits.

The existing NPC broadcast test helper gains a count argument for these5-swing windows; all existing240 callers retain explicit512 and their unchanged60-frame-per-swing delivery budget. Native NPC-to-NPC receipts remain actual all-client CombatEvent messages, never fabricated player ownership. Five-swing all-Miss probability .05^5=3.125e-7; baseline all-avoid .14^5≈5.378e-5, combined family bound below1/1000. Consumer is committed only after genuine RED; GREEN and independent lead gate pending. Nativejoint CC+49 remains source/wiring proof, not a separate new nativeRED. Receipts: engine `data/diagnostics/class-coverage-b-2026-10-17/melee-npc-victim-dodge-*`; [contract](../../specs/player-damage.md).

## Hit table values

| | Chance | Source |
|---|---|---|
| Miss | 5%, +19% with an off-hand weapon | Unit::GetUnitMissChance, MeleeSpellMissChance |
| Dodge (creature) | 3% + victim49 + 1.5%/level above, before attacker251/240 | GetUnitDodgeChance |
| Parry (creature) | 6% + 1.5%/level above | GetUnitParryChance |
| Glancing (creature 4+ levels above) | 10% + 10%/level | RollMeleeOutcomeAgainst |
| Block (creature) | 3% + 1.5%/level above, stops 30% | GetUnitBlockChance, GetBlockPercent |
| Crit | `CharacterStats.crit_pct` | GetUnitCriticalChanceDone |

TrinityCore's creature values differ from SimC's Retail target (3% miss, 3% dodge, 3% parry at equal level). The server follows TrinityCore; the difference is unverified against Retail.

## Armor

- `world_data.rs` loads ExpectedStat `CreatureArmor` and `ArmorConstant` with their ExpectedStatMod columns, and `ArmorModifier` per template.
- `LevelStats::armor` holds a tuned template's armor per level; an untuned template's `CreatureTemplate.armor` is ExpectedStat armor at `min_level` (ContentTuning 0, expansion 0 falling back to -2).
- `zones.rs` inserts `CreatureArmor` (armor at the spawn's own level) on every spawn.
- `ArmorConstants` (a resource, index = level) is the ExpansionID -2 ArmorConstant row with no class mod, what TrinityCore passes for a player attacker. Every ExpectedStatMod class row has ArmorConstantMod 1, so creature attackers get the same value.

Level 10: CreatureArmor 90, ArmorConstant 210 (30%). Level 30: 155 and 361.

## Spells

`spell_cast/effects.rs`:
- `scaled_amount`: BasePoints + BonusCoefficient × SP + BonusCoefficientFromAP × AP (unchanged).
- `with_variance` on SCHOOL_DAMAGE and HEAL: × (1 ± Variance / 2). Most class spells have Variance 0.05. Test helper: `test_harness::varied`.
- `with_crit`: + 100% (× 2), with CritDamageAndHealing mods (unchanged).
- `deal_damage`: × damage done × versatility × damage taken, then armor if `reduced_by_armor` (physical school, no bleed mechanic 15), then absorbs and level scaling. `apply_heal` takes versatility too.
- TRIGGER_MISSILE (32) shares TRIGGER_SPELL's handler: Frostbolt 116 → 228597 (2.448 × SP).

Spell power is Intellect only for an Intellect main stat (`stats.rs`); warriors, rogues and other Strength or Agility specs have 0.

## Maximum-health percent effect165

TC a352b1fa SpellEffects5165–5173 adds `CountPctFromMaxHealth(effectValue)` directly to spell damage; Unit.h809 returns an integer. Native effect165 now selects real hit units and truncates authored percent of their maximum health before common damage resolution. It does not use current health or add SP/AP, direct damage-done or SpellDamageBonusTaken modifiers. Final crit, armor, resilience, absorbs and existing death/hit boundaries remain shared. `damage_resilience_multiplier` extracts the existing final versatility factor without changing ordinary damage-taken semantics.

Real BB gun67531 carries0.01percent and only effect165. Native RED9e862790 accepts its cast on authored16802 but leaves50000/100000 health unchanged instead of49990. Native GREEN4091b81b passes2/2(18.30s), including actual damage10, native Enrage26662/Barkskin22812 bonus exclusion and5000max sub-point truncation; zero damage still produces an actual hit and TargetCreatureId4 credit. Both-faction leader grants and durable re-entry are proved separately in [achievements](dungeon-achievements.md#pre-effect-spell-hit-target-creature-entries). Required workspace achievement/criteria/statistic/instance filter178/0/1ignored plus site-export1/1 and server/engine checks/three-repo fmt pass at source4091b81b; later docs-only commits preserve proof. Acquisition/equipping, holiday availability and world-leader content are not certified.

## Numbers

| Case | Working | Result |
|---|---|---|
| L10 Human Warrior, no spec, Worn Shortsword (0-1, 2.60) vs L10 Hogger | trunc(0-1 + 33 / 6 × 2.6) = 14-15; × 0.7 = 9-10; × 13,137 / 1,133 = 104-115 of the pool | ~133 swings (unit test: 110-160) |
| L10 Arms Warrior, same sword, vs L10 Mangy Wolf (live) | Str 33 × 1.05 (Plate Specialization) = AP 34.65 → 15-16; × 5.76 (Arms Warrior +476% auto-attack) = 86-92; × 0.7 = 60-64 | 7 hits + 3 dodges, dead in 24 s |
| L10 Human Mage Frostbolt | 2.448 × Int 38 = 93 ± 2.5% | 91-95, 181-191 on a crit |

Live proof: `game-engine/data/diagnostics/playerdamage-20260926/` (server and client combat logs, kill screenshot).

## Sources

- [melee.rs](../../../crates/server/src/melee.rs) — weapon damage, hit table, outcome, armor formula
- [combat.rs](../../../crates/server/src/combat.rs) — swing pipeline, timers, events
- [level_scaling.rs](../../../crates/server/src/level_scaling.rs) — `CreatureArmor`, `ArmorConstants`, `armor_against`
- [effects.rs](../../../crates/server/src/spell_cast/effects.rs) — spell amounts, variance, versatility, armor, TRIGGER_MISSILE
- TrinityCore master a352b1fa: `Entities/Unit/Unit.cpp` (CalculateMeleeDamage, RollMeleeOutcomeAgainst, CalcArmorReducedDamage, SpellBaseDamageBonusDone, SpellCriticalDamageBonus), `Entities/Unit/StatSystem.cpp` (Player::CalculateMinMaxDamage, UpdateAttackPowerAndDamage, GetPrimaryStat), `Spells/SpellEffects.cpp`, `Spells/SpellInfo.cpp`
- SimC `WEAPON_POWER_COEFFICIENT` (6) for AP per weapon DPS

## See Also

- [[combat]] — auto-attack systems and death transitions
- [[character-stats]] — AP, SP, crit, haste, versatility, weapons
- [[level-scaling]] — damage and health multipliers after armor
- [[spell-casting]] — effect handlers
- [[auras]] — `AuraStatMods` (MOD_AUTOATTACK_DAMAGE, damage done/taken)


### Shared weapon-spell defense roll

Weapon-spell miss, ranged deflection and block consume one native integer roll and cumulative truncated hundredths. Magic behavior remains separate. Existing live player DodgePercentage now feeds melee spell outcomes before block; source frontal/control/attribute guards and lifecycle have six native Evasion controls. Non-player caster aura240 now subtracts its native active total/4, clamped at zero. Source62627/38598 is authenticated; receiver-corrected ae251a10 records actual Strike `Miss(Dodge)` despite caster expertise10000. Earlier unrelated-connection results are invalid feature REDs. Corrected application/removal/foreign ownership controls and merged31 block/dodge/table/cw150 controls pass; current workspace/all-target check and fmt pass. Independent acceptance remains pending. Physical aura117 must add the victim's nonnegative maximum selected-mechanic total after miss and before NoActiveDefense. Effect mechanic overrides category; signed applied amounts/stacks sum only within a selected mechanic. Initial cd651cb7 full-table RED is invalid: neutral NPC6 omitted guarded baseline5, yielding85+10 rather than100. Native hostile Kobold40 replaces it; corrected2c4a7ecb records actual stun atdraw10 with verified hostility/miss90/live11710. Implementation follows this genuineRED; c31715af native3/3 GREEN proves256 miss/resist outcomes without stun, removal, foreign recipient and mismatched silence selector. Source-identical27 prior block/dodge/table controls retained; independent acceptance pending. Recipient/removal/silence-selector controls bound this proof, not full117, magic or ability certification. [Source and receipts](../../class-completeness6.md#physical-mechanic-resistance--native-slice-2026-10-14). Caster251 now adds its signed active total before expertise and final clamp at the existing player melee-dodge boundary. Native48250e2+20/110294e0−25 and actual hostile40/38598 give genuine damage RED at24fb3cb5; merged18274ca5 eleven native controls/check/fmt pass. Signed sums, clamp, removal, recipient ownership and retained Evasion/expertise boundaries are exercised; recipient effects do not contribute. [Exact scope and proof](../../class-completeness6.md#signed-attacker-enemy-dodge-modifier--native-slice-2026-10-14). Ordinary NPC melee-spell dodge now uses native3% plus victim49 and positive effective-level bonus, with rear PvE attacks permitted; control/casting/ranged/attribute guards remain. Caster251/nonplayer240 ordering is shared with players. Native70188 effect0/49=100 establishes genuine Strike damage RED→front/rear dodge GREEN; removal/stun/attributes/ranged controls use source template tuning and a separate nonlethal health buffer. Existing block-only controls isolate dodge with native permanent45769 effect1/251−20 and suitable player level, not RNG suppression. Follow-up chance-boundary assertions use actual native40/626 tuning and player levels10/31/1/2:3/3/9/7.5 after49 removal, plus native100 points while active. Seven scoped controls pass. This certifies exact arithmetic/tuning integration, not a sampled3% attack distribution or whole NPC loadouts. [NPC source and receipts](../../class-completeness6.md#npc-physical-spell-dodge--native-slice-in-progress-2026-10-15). NPC parry, player expertise/248 modifiers and player hit baseline remain unimplemented boundaries. Two actual native cast invariants and quantization controls pass alongside retained block controls; 45 adjacent controls, workspace/all-target check and fmt also pass; independent acceptance remains pending. [Exact source and proof](../../class-completeness6.md#shared-weapon-spell-roll-boundary--batch59-in-progress).

### Native player critical/block hit state

Autoswings consume integer aura253 chance on the defender and double the already-truncated blocked amount. Player weapon-spell hit plans retain sparse per-unit block results through flight; the existing innermost matching `ResolvingHits` frame supplies that result before absorption. Critical spell blocks double the integer shield percentage before truncation/full-block cap, not the amount after truncation. Chance is read at impact; facing/block hit result belongs to launch. Source: TC a352b1fa Unit.cpp2564–2570,1469–1473,2610–2741,1243–1257.

This player slice does not certify NPC block flags/baseline/totems or full hit-table ordering. Ordinary NPC weapon-spell blocks consume source3% plus aura51 and positive effective-level bonus; authenticated NO_BLOCK0x10 excludes blocking, unlike NO_XP0x40. Native blocked damage is30%, doubled to60% for critical blocks. Four NPC controls plus eleven player controls and45 adjacent controls pass; workspace/all-target check and fmt pass. Independent acceptance remains pending. Source Totem/Lightwell runtime is still unsupported, not inferred from template type. Ordinary NPC implementation and its bounded proof are tracked separately in [the NPC boundary ledger](../../class-completeness6.md#ordinary-npc-spell-block-boundary--batch58-in-progress), including source-defined Totem/Lightwell identity and the unsupported summon-runtime boundary. Aura638 ownership conflicts between pinnedCPP and MartialExpert description; generic253/186 certificates remain absent. Exact source and proof ledger: [class completeness](../../class-completeness6.md).
