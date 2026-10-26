# Character Stats

`StatsPlugin` (`crates/server/src/stats.rs`) computes player stats and the approved [NPC follower bridge](../investigations/follower-data.md#passive-derived-stat-refresh--2026-10-15) in the retail 12.x model. Players spawn with default `UnitStats` and the health persisted in redb; followers initialize the same stat fields from their labelled class/spec and retain race0. `update_character_stats` computes both on the first frame and again when their level, spec/profile, equipment, known spells or `StatModifiers` changes. It writes:

- the effective `UnitStats` and `CombatRatings`, and `DerivedStats` (`CharacterStats::derived`: spell power, distinct melee/ranged attack power, the crit/haste/mastery/versatility percentages and leech/avoidance/speed);
- `Health.max` and `Mana.max`, keeping each current value at the same fraction of the new max;
- the server-only `CharacterStats` component, read by the [[spell-casting]] effects and costs.

Ordinary NPCs without `CharacterStats` still have no class-derived spell power, attack power or crit. Approved followers carry `CharacterStats` and the stat fields required by this same production query, while remaining NPCs rather than persisted players. Their absent `SheetOwner` keeps owner-only sheet fields hidden; health/mana replication remains ordinary.

## Native weapon attack power — 2026-10-23

`CharacterStats.weapon_attack_power` retains raw integer main/off/ranged weapon AP from unrounded item-table DPS×6, including effective instance level/quality (TC a352b1fa Player.cpp8180–8191). Sheet melee/RAP fields remain unchanged. Native coefficient selection adds max(main,ranged) weapon AP for main/ranged attacks, or adds offhand AP and halves the full offhand sum, before the appropriate AP percentage (Unit.cpp9973–10000). Damage/heal/absorb coefficients share the ranged/exclusive-offhand/main selection; equipped player and approved follower stats do not replace guardian inheritance. Aura366 derived SP/healing power now reads total main-hand AP; weapon/white-hit inputs retain bare AP to avoid double-counting.

Ignored actual Tiger Palm RED:91 vs TC358–377 from base98+weapon291, native coefficient0.9 and Battle Shout1.05. Generic implementation committed after that RED; four targeted native cases pass13.89s at4b9d2be73 (zero compiler warnings), including offhand/ranged percentages and SP/unequip. Tests are now enabled; parent-owned integration gates remain pending. Autoattack AP/6 versus TC3.5 remains an explicit deferred discrepancy, unchanged. [Proof/source ledger](../../handoffs/ww-weapon-ap-proof-ledger.md).

## Generic mastery base — 2026-10-23

TC a352b1fa `Player::UpdateMastery` (`StatSystem.cpp549–559`) sums aura318 and mastery rating only. The old producer added hardcoded8 to native114585's actual aura3188. An unequipped level90 Arms Player with real passive/StatsPlugin reproduced16 instead of8 at9215bc7fc (genuine native RED10.95s, frozen canonical SHAcd7230ac1d62bf7d7ea8fef5fbb82faa9f36b61ec78108e591f1204c614a8ed6). Removing the constant fixes the generic source rather than adding a per-class correction. Rating DR and specialization coefficients are unchanged. At0576893d7 native zero-rating Warrior/Monk/Mage and actual unlearn/relearn pass1/1 (14.73s); the stats-filter scope passes21/21 (8.35s), including real character selection/level changes and aura8+rating2=10. No compiler warnings; parent integration gate remains separate. Stats-only unit worlds without aura318 now correctly report no implicit base. [Historical Arms decomposition](../investigations/arms-spell-factors.md) separates this from gear, tier and comparator differences.

### Initialization and knowledge gate — 2026-10-24

Removing the unconditional8 exposed a production follower initialization gap: `default_skill_spells(class, 0, level)` rejected skill183's explicit all-races mask `[-1,-1]`. Native `skill_race_class_info` row5 and `SkillLineAbility`25920 supply114585 (aura318,8 points) through that universal skill; ordinary human progression already learns it. `race_mask_allows` now admits the explicit universal mask for race0 while retaining restricted-mask rejection. Stats also gate aura318 plus rating on knowing a mastery spell of the active specialization, matching TC a352b1fa `Player.cpp30545–30550` and `StatSystem.cpp549–568`; spec coefficients never supply a base. Native created-player/level-up/passive-reapply passed, while initialized Garrick failed knowledge114585 at5d1c30c81. After merging1fb0230ad, all19 sa1089 regression cases pass across d96bfa968/faea69f47; source-neutral follower/base+bonus knowledge/lifecycle/race controls also pass. Fixture-only corrections supply real spec knowledge/base passives or the production StatsPlugin; no existing behavioral assertions changed. Two-crate test check passes once atfaea69f47 (2m47s, zero warnings), fmt passes; full integration remains lead-owned. [Per-test proof and production RED](../../handoffs/generic-mastery-initialization-proof.md).

## Named Warrior strength/AP policies — 2026-10-23

[`WARRIOR_ARMORED_TO_THE_TEETH_SIMC_POLICY` and `WARRIOR_BATTLEFIELD_COMMANDER_SIMC_POLICY`](../investigations/arms-spell-factors.md#ap-components) bind actual active384124 ranked226 amounts and424742 effect5 dummy amount for Warrior Players/class followers. Other classes and ordinary NPCs are excluded. The stat producer solves the native strength/armor linear relationship once (including Vanguard), preserving integer native armor and existing factors; Commander enters shared AP percentages, including weapon AP. TC absence, pinned SimC relationships and approval are ledgered with the [contract](../../specs/character-stats.md#named-warrior-stat-policies).

Four actual reference-gear Arms/Protection REDs at e5cc1ef32 (23.48s) establish missing strength/AP; native rank2 amounts10/4 and Commander3 observed, unlearn/relearn controls use the same real actor. Arms remains on one-factor armor2855. At98093ec85 (productiona1af93852) four policy cases pass19.13s: Arms strength2963.6248/armor2855, Protection3048.0583/7382, and Commander3% on both bare/weapon AP with exact removal restoration. Retained native mastery1GREEN8.20s/stats-filter21GREEN9.07s; actual Arms MS1GREEN10.79s. Build has no compiler warnings. Parent integration gate remains separate.

## Follower mastery amount refresh — 2026-10-21

`auras/mastery.rs` originally admitted only `Player`, although approved followers already had `CharacterStats.mastery_points`. This left native76671/76857 aura166 AP bonuses and168534 aura108 damage modifiers at zero on the NPC bridge. The same attribute-driven calculation and amount refresh now admit `Player` or `Follower`; an arbitrary NPC carrying `CharacterStats` remains excluded. Source: TC a352b1fa `SpellInfo.cpp600` / `StatSystem.cpp549`, extended only by the existing [follower class-stat policy](../investigations/follower-data.md).70448 coefficients are unchanged. Native ignored RED reproduced AP620 vs669.6 and Lightning Bolt modifier157.5 vs166.118; enabled mastery/NPC controls pass in the13-case2ba4ec43a GREEN (22.04s). Tests: `followers/passive_stats_tests.rs` (`dps_parity_follower_native_*`).

## Hunter pet health and armor — 2026-10-22

`hunter_pet_stats.rs` derives controlled Hunter pet create health from native ExpectedStat/template metadata, then adds truncated30% owner Stamina×10 HP. Base armor is level×50 plus70% owner armor, truncated. Sources: TC a352b1fa `Pet.cpp847–925` and `StatSystem.cpp1187–1310`; [requirements](../../specs/dps-parity.md#hunter-pet-survivability). Legacy class-level rows exist but contain HP1 placeholders; absent `pet_levelstats` does not justify fabricated primary stats. SimC stable_pet70%Stamina/1.05armor is named divergence `HUNTER_PET_SIMC_SURVIVABILITY_DIFF`, not runtime policy.

FixedUpdate Call Pet can precede Update's equipment/passive calculation. Summon invokes the existing cached `stats::update_character_stats` producer before reading owner stats, matching the Ebon Might snapshot pattern. Fresh currentHP equals the correct maximum without a top-up flag or second formula. Stored wounded currentHP hydrates afterward. Later `Changed<CharacterStats>` refresh applies only the inherited maximum delta, preserving unrelated health bonuses; growth does not heal and shrink clamps current (TC `Unit.cpp10070–10093`). Armor feeds existing physical-damage consumers. Demon/Mage pets and ordinary NPCs retain their existing paths.

Native exact253/level90 Duck174884 reproduced HP1/no armor versus178094HP/5718armor. Separate ordering RED reproduced current56244/max178094; shared-producer fix and six native controls pass, including full fresh summon, real Fortitude growth/removal without healing, reference gear removal/restoration and foreign-controller exclusion. After ps988 integration repairs, six native stat tests, all22 Hunter pet tests and five battleground pet-end tests pass. Lifecycle proofs retain real23HP reopened wounds,17HP logout fence, three revive controls and exact hydrated-health dismissal/recall. The lightweight battleground Traveler fixture supplies the same default UnitStats/CombatRatings/DerivedStats inputs as real character entry; the ordinary cached producer computes CharacterStats. Fresh tame additionally asserts exact native current=max377 and the entire persisted stable. Fixtures load actual ExpectedStats; the legacy378HP seed clamps to native377, so dismissal asserts persistence of actual hydrated currentHP, not the stale pre-hydration seed. Receipts: game-engine `data/diagnostics/dps-parity-fix-2026-10-21/pet-stats-gate-packet.md`. This is not pet melee AP, whole pet aura/stat parity, new mob-hit mitigation proof or Azure survival acceptance.

## Hunter pet melee owner RAP — 2026-10-22

`hunter_pet_stats::owner_bonus_attack_power` is the shared melee/ability source: integer22% of `CharacterStats::total_attack_power(Ranged, StatModifiers)`, including equipped weapon AP through the ordinary producer. `combat.rs` reads that source live at each actual Hunter pet swing, using existing `HunterInheritedStats` to exclude Demon/Mage pets. Inherited AP/3.5×2s is additive before physical modifiers, armor and outcomes (TC a352b1fa `StatSystem.cpp1344/1389/1423`, `Pet.cpp882`). `effects::hunter_pet_attack_power` uses the same source with unchanged native ability coefficients. Missing required owner CharacterStats fails explicitly, rather than silently substituting zero AP. No new cached AP state or intrinsic weapon range is introduced. Existing intrinsic melee base remains unqualified, not a sourced total-DPS claim. SimC76f8118 stable_pet60% ownerAP stays named divergence `HUNTER_PET_SIMC_OWNER_AP_DIFF`.

Native Duck174884/level90 baseline6eeb06d59 dealt four attributed14-damage swings with ownerRAP3034.9246 versus meleeAP2944.9246 and native2000ms base clock. Ignoreded2800ce9 genuine RED kept those14 swings below the independently derived owner-only contribution floor266 after actual armor/modifiers; baseline positive control passed (15.13s combined). The oracle does not invent intrinsic AP/ranges and leaves hit RNG intact, excluding blocked/absorbed/glancing receipts. After generic weapon AP12daba landed, native7a41a8fdf additionally reproduced three positive controls/two genuine REDs34.56s: bare ownerRAP3034.9246 plus native weaponAP934 gives total3968.9246 and required inheritedAP873; current melee583 missed the owner-only floor743, and actual1264355 damage1899 missed floor2484 (native coefficient1.90999997/variance0). Real gear removal/restoration and caster-only70831 melee-AP-versus-RAP controls passed before the shared source fix. Compile-only `critical`/`crit` test-field failure6239 is not behavioral evidence. Shared-source GREEN pending. [Contract](../../specs/dps-parity.md#hunter-pet-melee-owner-rap).

## Sources

| Data | Source | Status |
|---|---|---|
| Class base Str/Agi/Sta/Int per level 1–90 | SimC `engine/dbc/sc_extra_data.inc` `__gt_class_stats_by_level` | No DB2 or game table holds these; SimC measures them in game. See TODO gaps below. |
| Racial modifiers | SimC `__gt_race_stats`, mapped to ChrRaces IDs | Dracthyr, Earthen and Haranir are 0 in SimC and are not in the table. |
| Health per stamina | CASC `gametables/hppersta.txt` (FDID 1391642) | From the local 12.x install. |
| Base mana | CASC `gametables/basemp.txt` (FDID 1391664) | From the local 12.x install. |
| Rating per 1% (crit, haste, versatility) and per mastery point | CASC `gametables/combatratings.txt` (FDID 1391669): columns Crit - Spell, Haste - Spell, Mastery, Versatility - Damage Done | Melee, ranged and spell columns are equal. |
| Spec main stat | `chr_specialization.PrimaryStatPriority`: 0–1 Int, 2–3 Agi, 4–5 Str | |
| Mastery coefficient | Effect 0 `EffectBonusCoefficient` of the spec's `MasterySpellID_0`, for example Holy 183997 = 1.5 | |
| AP conversion | `chr_classes.AttackPowerPerStrength` / `AttackPowerPerAgility` / `RangedAttackPowerPerAgility` | |
| Gear stats, armor, weapon damage | world.db `content_item` + `item_sparse`, DB2 `rand_prop_points`, `item_armor_{quality,shield,total}`, `armor_location`, `item_damage_{one,two}_hand{,_caster}`, `item_squish_era`, `curve_point` (12.1.0.69933); CASC `gametables/{combatratingsmultbyilvl,staminamultbyilvl,itemsocketcostperlevel}.txt` (FDIDs 1391670, 1980632, 1391643) | See [Gear](#gear). |

- SimC checkout: `simulationcraft/simc` commit `303610892fe2cd9c71a75eae89007c8d320788ff` (2026-04-05), client data 12.0.1.66709 (`client_data_version.inc`).
- The three CASC tables are byte-for-byte the values in SimC `sc_scale_data.inc` for build 12.0.1.66709: crit at level 80 is 12.158 and at 90 is 46.
- `octhpperstamina`, `octbasehpbyclass`, `octbasempbyclass` and `chancetospellcrit` are in the listfile but not in the local root.
- Retail has no base health. SimC `dbc_t::health_base` returns 0.
- `scripts/gen_character_stat_tables.py` generates `crates/server/src/character_stats_data.rs`. It reads a SimC checkout and `data/gametables/` (extracted with `casc-local`).

## Formulas

1. **Primary** = class base + racial modifier + gear + `primary_flat`, then × (1 + `primary_pct`/100).
   - Gear Str/Agi/Int count only when they match the spec's main stat. Gear stamina always counts.
2. **Max health** = floor(Sta) × hppersta(level) × (1 + `max_health_pct`/100) + `max_health_flat`, floored.
3. **Mana:** max mana = base mana = basemp(class, level). Intellect does not add mana.
   - Only classes with a non-zero basemp column get a `Mana` component: Paladin, Priest, Shaman, Mage, Warlock, Monk, Druid and Evoker.
4. **Spell power** = Int when the main stat is Intellect, else 0 (TrinityCore Unit::SpellBaseDamageBonusDone). **Healing power** (`CharacterStats.healing_power`, used by heals, periodic heals and absorbs) = Int for every mana class (Unit::SpellBaseHealingBonusDone). Aura 366 OVERRIDE_SPELL_POWER_BY_AP_PCT sets both to AP × pct (Retribution, Protection, Feral/Guardian, Enhancement, Windwalker/Brewmaster passives). The main stat is the active spec's, else `chr_classes.PrimaryStatPriority`. **Melee AP** = (Str × AttackPowerPerStrength + Agi × AttackPowerPerAgility) × composed aura166 multiplier. **Ranged AP** = (level + max(Agi, 0)) × RangedAttackPowerPerAgility × composed aura167 multiplier (TC a352b1fa StatSystem.cpp354–414). Aura366 reads melee AP; native ranged-defense spell AP coefficients and hunter pet inheritance read RAP.
5. **Crit %** = 5 + DR(crit rating / rating-per-1%) + `crit_pct`.
6. **Haste %** = DR(haste rating / rating-per-1%) + `haste_pct`.
7. **Versatility %** = DR(versatility rating / rating-per-1%) + `versatility_pct`.
8. **Mastery %** = (DR(mastery rating / rating-per-point) + aura318 `mastery_points`) × mastery coefficient. Native114585 supplies8 base points; no second hardcoded base.
9. **Armor** = gear armor rating.

DR is curve 21024 from SimC `item_scaling.inc` (12.0.1.66709), applied as in `player_t::apply_combat_rating_dr`. It is the identity up to 30%. Tertiary ratings (gear `ITEM_MOD_CR_SPEED` 61, `CR_LIFESTEAL` 62, `CR_AVOIDANCE` 63) convert with gtCombatRatings Speed/Lifesteal/Avoidance and diminish on curve 21025 (`DIMINISHING_RETURN_TERTIARY_CR_CURVE`, identity up to 10%); no content item carries one today. Rating conversion and DR exist only in `stats.rs`; the server does not use `shared::formulas` rating functions.

Rating units follow SimC `rating_t::init`: the game-table value is the rating needed for 1%, or for 1 mastery point.

## Gear

### Shared profile effects — 2026-10-25

The parity fixture installs declared consumables and expansion effects in `dps_parity/profile_effects.rs`. Numeric Omnium tokens identify native `TraitNodeEntry` rows, not class nodes. The fixture's `ReferenceRace` supplies native base-stat race and racial knowledge without adding `Player` identity to followers. Native aura effect202 admits owner racial effects such as Dwarf59224.

Aura29 misc−2 contributes hybrid Str/Agi/Int through the native main-stat filter, not Stamina/Spirit (SimC `buff.cpp3421–3434`). Aura189 contributes every represented native rating mask to derived stats, not only versatility. Item spell scaling class−7 multiplies its native item budget by the trinket combat-rating multiplier before rounding (SimC `spell_data.cpp424–441`); quality2 Magisters is165 native mastery, not170 tracked-only points. `EquippedItemModifiers.temporary_enchants` carries slot/enchant identities independently of permanent enchants; ordinary Stats and equipment-aura reconciliation consume both. The profile's potion uses native item readiness and removes its one-use fixture entitlement after a successful cast. `consumables.rs` supplies Royal Roast's native coefficient relationship and Recklessness's highest/lowest-secondary selection.

Shared token bindings also cover full Void-Touched augmentation, Thalassian Resistance2 versatility and Light's Potential2 primary stats. `profile_oils.rs` parses temporary oil conditions through the shared SimC expression evaluator and native talent facts; false conditions and `disabled` are not unsupported effects.

Liquid Luster's native driver1295132 scales the per-stack rating; its triggered Lustrous Gleam1295147 has zero base rating and cannot supply that amount alone. `server-combat/src/consumables/liquid_luster.rs` applies the native Gleam once immediately and replaces the driver's periodic trigger with the same payload, retaining native stack limits and a common expiry. The shared literal prepot path casts the real item, ages its aura phase/deadline and actual item cooldown, then lets native periodic scheduling pay elapsed pulses. It does not consume the later combat entitlement. Sources: SimC76f8118 `unique_gear_midnight.cpp288–366`, `consumable.cpp500–565`; [consumable requirements](../../specs/dps-parity.md#equipment).

`spell_cast/scripts/midnight_items.rs` implements pinned SimC76f8118 Heart, Venomfang and Omnium relationships using native values. Native asynchronous stack expiry has an optional stack-loss hook for Venomfang bursts. Perfected Guillotine poison reads the equipped weapon driver's item level, not the trinket's. Proc recipient checks reuse the native nullable-recipient predicate: faction-neutral31146 is attackable and must not be rejected by an `is_hostile` test.

Sources: SimC `unique_gear_midnight.cpp184–220,3750–3803,6063–6202,6279–6383`, `unique_gear.cpp3677–3705`; [profile contract](../../specs/dps-parity.md#shared-profile-gear-effects), [item scripts](../../../crates/server/src/spell_cast/scripts/midnight_items.rs), [profile setup](../../../crates/server-simulation/src/dungeon_simulator/dps_parity/profile_effects.rs). Pet propagation of effect202 is not qualified by the owner-only racial fixture.

### Profile stat-allocation redirects — 2026-10-23

`redirected_base_stats` is server-local per-slot context in `EquippedItemModifiers.base_stat_redirects`, not a replacement equipped item or a wire/persisted modifier. The shared parity input validates the native donor; Stats uses a temporary original-template copy with only donor stat types/allocations, then the original profile's ordinary bonuses/item level. Original socket multipliers, armor, weapon, effects and set membership remain untouched. Explicit0 means no redirect, as in pinned SimC `item.cpp95/1314`; malformed/missing native donors fail before gear insertion. Other unknown options retain their diagnostic gaps.

Actual103 profile→StatsPlugin at7aa72bf47 reproduces five tier allocation errors while all ten ordinary slots match, plus invalid-donor and false-unmodeled-option REDs:3RED11.02s, build0/zero warnings27m56. Implementationf9a01f71e plus master6b merge d45fe30e8 passes3GREEN12.43s, clean25m11build. Two native cases remain opt-in; the env-free diagnostic stays enabled. Lead integration/DPS qualification remain open. [Contract](../../specs/dps-parity.md#equipment), [source/commands/proof](../../handoffs/equipment-stat-redirect-proof-ledger.md).

`item_stats.rs` computes each equipped item's stats; `ItemScalingTables` loads the DB2 tables at startup and the three item-level game tables are generated into `character_stats_data.rs`. Formulas are SimC `sc_item_data.cpp` (`scaled_stat`, `random_suffix_type`, `item_combat_rating_type`, `armor_value`, `weapon_dmg_min/max`) and `sc_const_data.cpp` `dbc_t::weapon_dps`; TrinityCore `Item::GetItemStatValue`, `ItemTemplate::GetArmor`/`GetDamage` read the same tables.

1. **Item level:** ItemSparse `ItemLevel`; items with `ItemSquishEraID` below 2 take the Midnight squish curve 92181 (1→1, 10→10, 342→70, …), rounded (SimC `item_t::download_item_data`). Black Duskwood Staff 21 → 12.
2. **Budget:** RandPropPoints float column for the level: Epic (quality 4-5), Superior (3, 7) or Good (rest); column 0 two-handers/head/chest/legs, 1 shoulders/waist/feet/hands/trinket, 2 neck/finger/cloak/wrist, 3 one-handers/off-hands.
3. **Stat** = floor(StatPercentEditor × budget × 0.0001 − round_half_even(StatPercentageOfSocket × socket cost) + 0.5), with ratings × gtCombatRatingsMultByILvl and stamina × gtStaminaMultByILvl for the slot column (armor, weapon, trinket, jewelry), multiplied before rounding.
   - Modeled `ITEM_MOD`s: 3 Agi, 4 Str, 5 Int, 7 Sta, 71–74 hybrids (added to each listed primary; the spec main-stat filter keeps one), 32 crit, 36 haste, 40 versatility, 49 mastery.
4. **Armor:** shields floor(ItemArmorShield[quality] + 0.5); cloth–plate floor(ItemArmorTotal[material] × ItemArmorQuality[quality] × ArmorLocation[slot][material] + 0.5), robe uses the chest row.
5. **Weapon:** DPS from ItemDamageOneHand/TwoHand (Caster variants with `ITEM_FLAG2_CASTER_WEAPON` 0x200; bows/guns/crossbows two-hand, wands one-hand caster); min = floor(DPS × speed × (1 − DmgVariance/2)), max = floor(DPS × speed × (1 + DmgVariance/2) + 0.5).

The main-hand result is `CharacterStats.main_hand` (unarmed 1–2 at 2.0 s, TrinityCore `BASE_MINDAMAGE`/`BASE_MAXDAMAGE`); an off-hand weapon is `CharacterStats.off_hand`. Auto-attacks swing both ([[player-damage]]). Spell weapon hits (effects 58, 121, 31) roll it and add AP / 6 × speed; 121 uses the normalized speed (dagger 1.7, one-hand 2.4, two-hand 3.3, ranged 2.8).

`EquippedItems.items` holds `(EquipmentSlot, ItemDefinitionSource, item_id)` of physically equipped items. Inventory publishes it on world entry and deltas. `EquippedItemModifiers.items` separately carries slot-bound instance properties; changing only those properties also triggers the production recompute.

### Native equipped-set auras — 2026-10-23

The consolidated rt1086 path reconciles ordinary item/enchant and set-owned auras in the global AuraPlugin FixedUpdate schedule, sharing the aura applier. `item_proc_data::sets` is the sole catalog/predicate source; original Retail `ItemTemplate.item_set` comes from native `ItemSparse.ItemSet`, with native `ItemSetSpell` rows indexed by set/spell. Selection uses actual equipped counts, threshold and current specialization; TC legacy-inactive bit0x01 disables a set. Allocation donors never replace equipped identities. The consolidated source is `AuraSource::ItemSet(set_id)`, not an arbitrary member slot; losing eligibility removes only the owned aura. Earlier603/778 receipts named that internal source `EquipmentSet`; no alternate producer/source path is retained. Active-spec changes also reconcile. No spell-ID cases or imported stat totals.

Strict native RED atb84957a9e: real Arms2067 two-piece equip has0 instead of1 aura1296643; real Protection own2pc likewise0 instead of1 while Arms exclusion passes (two genuine failures29.67s). Both use exact reference Player gear/selected traits and the real application schedule. At778524924 (production8171bf2aa) both native lifecycle cases pass30.66s: equip/unequip,2/4-piece thresholds and spec switches. A control-fixture correction derives membership from native ItemSparse: Prot wears Legs271455 rather than Arms Hands, so the old slot guess accidentally kept two pieces. Production selection was unchanged. Retained Warrior policy4GREEN27.23s; actual Arms MS1GREEN16.91s observes1296643/1296644 owned by EquipmentSet2067 and native2pc10% actually applied. Source product1.818181634 matches oracle1.818182; damage27631, AP4014.553467/caster armor2855/mastery605 unchanged. Both-package `cargo check --tests` passes90.70s, zero warnings. Parent integration gate and live deployment remain separate.

Production entry registers ItemScaling/Inventory/Aura plugins; startup loads the same metadata and login/inventory deltas publish `EquippedItems`/`EquippedItemModifiers` consumed here. This is not a fixture-only activation path. Read-only Sakuin inspection found activePID61507 starting `/var/lib/game-server/bin/game-server`; its installed ELF rodata did not contain current equipment-effects/producer-catalog markers. No exact deployed revision, new set activation or live equip smoke is inferred; no remote mutation/deployment occurred. Evidence: `native-spell-diff-2026-10-23/mastery-b67952ca4/set-activation/sakuin-*.proof.json`.

Consolidated on landed master5d39f490a atfb65c93c6; cf7c76c71 repairs only cross-crate predicate visibility and the relocated combat tests' stale import. Both-package `cargo check --locked -p server -p server-simulation --tests -j2` passes24.49s without warnings. Same actual Arms/Protection lifecycle2 + Warrior policy4 controls pass6/6 in31.34s after11m16s compilation; no doubled set aura, source formulas unchanged. Frozen native world SHA`cd7230ac1d62bf7d7ea8fef5fbb82faa9f36b61ec78108e591f1204c614a8ed6`; binary SHA`62640926474cd68c7c64fd45f9466a20483f2791450780cf6c9c88f9661298d8`. Full commands/code/binary receipts: engine `data/diagnostics/native-spell-diff-2026-10-23/consolidated-fb65c93c6/`. These targeted proofs replace603's duplicate-producer packet; parent integration/acceptance remains separate. No new ilvl/proc facts or DPS-band claim.

603 originally added normal sets only and named skill/subtree gaps. Consolidation preserves rt1086's pre-existing sourced selectors: actual `PlayerProfessions.rank` compared with native RequiredSkill/Rank, and the selected native TraitLoadout subtree compared with ItemSetSpell.TraitSubTreeID. No inferred skill or invented hero choice is supplied. The native conditional-set inventory remains below; it is no longer an unsupported list when those actual selectors are available.

| Set | Name | Required skill/rank |
|---|---|---|
|421|Bloodvine Garb|197/300|
|441|Primal Batskin|165/300|
|442|Blood Tiger Harness|165/300|
|444|The Darksoul|164/300|
|559|Spellstrike Infusion|2539/50|
|565|Khorium Ward|2476/50|
|566|Burning Rage|164/350|
|569|Faith in Felsteel|164/350|
|572|Battlecast Garb|2539/50|
|573|Fel Skin|2531/50|
|574|Strength of the Clefthoof|2531/50|
|575|Felstalker Armor|2531/50|
|576|Fury of the Nether|165/350|

Subtree-qualified row: set1964 `LH Test [do not translate]`, ItemSetSpell row7219/spell1240644/subtree21/threshold1. Its consolidated selector reads the actual selected native subtree; a missing/nonmatching selection does not qualify. Historical603 gap inventory/hash remains `set-activation/selector-gaps.proof.json`; it does not override preserved rt behavior. Real Blood Tiger442/skill165/rank300 control covers below/at/removed profession ranks; no new subtree runtime qualification is claimed here.

Sources: TC `Item.cpp76–230` AddItemsSetItem/RemoveItemsSetItem and `DBCEnums.h1527–1529`; [metadata selector](../../../crates/server/src/item_proc_data.rs), [production aura seam](../../../crates/server/src/equipment_effects.rs), [native lifecycle tests](../../../crates/server-simulation/src/dungeon_simulator/dps_parity/equipped_set_tests.rs), [contract](../../specs/dps-parity.md#equipment), [inventory](inventory.md).

### Instance properties — 2026-10-21

The additive redb `item_modifiers` table stores bonus lists, explicit item-level overrides, permanent enchants, gem item IDs and crafted stat choices by GUID. Existing `item_instances` soulbound bytes and inventory/wire encodings are unchanged. `InventoryRecord.item_modifiers` hydrates only owned GUIDs; the inventory model preserves those properties and projects them alongside equipped catalog identities. Startup creates the new table for existing databases.

The [native producer](../../../scripts/item_instance_data.py) reads the existing read-only 70448 exports into a supplied world database. Canonical publication remains lead-owned. The ordinary `scripts/db2.py import` invokes the same producer inside its existing transaction so native instance tables refresh with the core build. `ItemScalingPlugin` loads the new native bonus/enchantment resources; missing tables fail with the producer instruction.

| Resolution | Current coverage |
|---|---|
| ItemBonus | Types1/2/3/6/25/49/52: level delta, allocation, quality, sockets, crafted-stat replacement, Midnight config and crafting level. Type23 retains ItemEffect IDs; the SQLite resolver joins base/bonus effect IDs to real spell/trigger/cooldown fields. Native on-equip auras execute; scripted proc completeness remains open. |
| Explicit level / crafted choices | Existing RandPropPoints/weapon formula; no repeated legacy squish. Placeholders24/25 are replaced in order, never split into invented allocations. |
| Enchants / gems | Native type5 stats use the existing gtSpellScaling data and native level cap; GemProperties resolves the enchant. Physical-resistance type4/school0 contributes truncated, minimum-one armor (native8163 gives27 armor plus41 Strength/Agility); other resistance schools remain explicit gaps. SourceTC a352b1fa Player.cpp13509–13531. Type3 equip spells use the production aura path. Native8039 parent1236742 has authored BP0 but a nonzero item-scaling coefficient; LINKED_2 forwards the resolved amount to child1241710. |
| Inventory / stats | GUID persistence, owned projection, initial/delta delivery and modifier-only StatsPlugin refresh. Owned GUID hydration/projection passed after their genuine REDs, alongside modifier-only Stats and legacy source controls. |

Proof:18 library regressions pass (including three missing-effect RED→GREEN cases); the actual native chest271468/enchant7987 modifier-only boundary failed687 versus737 before consumer wiring, then passed alongside the unchanged level20 Arcane gear control. Two subsequent native GUID-hydration/projection cases gave genuine RED before implementation, then passed in the24-case library/main integration run. This is not DPS parity or a deployment claim; [the parity contract](../../specs/dps-parity.md) tracks the remaining harness and measurements.

Equipment qualification at447552507: native base268209, enchant8039, bonus13771 and item-level100→200 refresh all pass, including removal of linked children. LINKED/LINKED_2 follow TrinityCore a352b1fa `SpellAuraEffects.cpp5299–5335`: zero preserves child BP0, nonzero overrides it. The older test rejecting nonzero children contradicts that source; its replacement asserts native471492→471491 BP10 and392221→392223 BP8 plus owned removal. This does not qualify scripted procs or300s DPS.

Sources: [instance resolver](../../../crates/server/src/item_instance.rs), [bonus/enchant loaders](../../../crates/server/src/item_instance/load.rs), [GUID store](../../../crates/server/src/item_instance/store.rs), [inventory model](../../../crates/server/src/inventory_model.rs), [stat consumer](../../../crates/server/src/stats.rs). Pinned behavior reference: SimC76f8118 `sc_item_data.cpp` and `item.cpp1740`.

**Checked against Retail tooltips** (Wowhead tooltip API, 2026-09-24):

| Item | Level | Tooltip | Server |
|---|---|---|---|
| Notched Shortsword 727 | 5 | 1–2, 2.60, +1 Agi, +1 Sta | same |
| Black Duskwood Staff 937 | 21 → 12 | 6–9, 3.60 | same |
| Vendetta 776 | shown at 83 | 13–23, 1.80, +13 Agi, +13 crit | same at 83 |
| Forlorn Breastplate of War 35588 | shown at 34 | 26 armor, +8 Str/Int, +12 Sta, +15 crit | same at 34; 15 → 4/6/8, 18 armor unscaled |

## SimC level 81–89 TODO gaps

SimC marks these class blocks TODO for the listed levels:

| Class | TODO levels |
|---|---|
| Warrior | 81–89 |
| Hunter | 81–89 |
| Rogue | 81–89 |
| Priest | 81–89 |
| Death Knight | 82–89 |
| Warlock | 81–89 |
| Demon Hunter | 81–89 |

- SimC still ships numbers for those rows. They follow the main-stat curve of the verified classes (Paladin, Shaman, Mage, Monk, Druid, Evoker), with off-stats at the class ratios in the block comments, for example Warrior "Agi = 0.69 of Primary".
- The server uses the SimC rows as-is and interpolates nothing. The TODO levels are unverified in game.

## Examples

**Level 80 Human Holy Paladin, no gear:**

| Stat | Value | Working |
|---|---|---|
| Str / Agi / Sta / Int | 159 / 56 / 2325 / 159 | Human modifiers are 0 |
| Health | 46 500 | 2325 × 20 |
| Mana | 50 000 | |
| Spell power | 159 | |
| Attack power | 159 | Str |
| Crit | 5% | |
| Haste | 0% | |
| Mastery | 12% | 8 × 1.5 |

**Level 10 Orc Warrior (Initial spec, no mastery), no gear:**

| Stat | Value | Working |
|---|---|---|
| Str / Agi / Sta / Int | 36 / 20 / 556 / 25 | 33/23/555/26 + Orc +3/−3/+1/−1 |
| Health | 1112 | 556 × 2 |
| Mana | none | |
| Attack power | 36 | |
| Crit | 5% | |

**Level 90 Paladin:** 460 crit rating gives 10% more crit, because 46 rating = 1%.

## Fold-in point for auras

`StatModifiers` is a Component with every field additive and a default of none:

- `primary_flat`, `primary_pct`
- `ratings_flat`
- `crit_pct`, `haste_pct`, `mastery_points`, `versatility_pct`
- `max_health_pct`, `max_health_flat`
- `dodge_pct`, `block_pct`

MOD_INCREASE_HEALTH 34 changes current/max health by the same flat delta on aura change, rather than rescaling current health. Aura 49 adds nondiminishing player dodge points. [Defensives audit](../../defensives-audit.md) records catalog amounts, lifecycle tests and remaining gaps.

The aura runtime ([[auras]]) inserts or updates it, and `Changed<StatModifiers>` triggers a recompute. The pure entry point is `compute_character_stats(&StatInputs, Option<&StatModifiers>)`.

## Existing characters

The spawned `Health` comes from redb. The first recompute keeps the persisted ratio: a character saved at 50/100 enters as 23 250/46 500 at level 80, and full stays full. The rescaled value is truncated to whole points (999/1000 → 1108/1110), as TrinityCore's integer health. The periodic save writes the new max back, and the redb format is unchanged.

## Gaps

### Gear

- **Scaled item levels:** Wowhead shows some old items at a level other than ItemSparse `ItemLevel` (Vendetta 20 at 83, Riverpaw Leather Vest 6 at 80, Forlorn Breastplate 35 at 34). The source of that scaling (drop context bonus lists, ContentTuning, `PlayerLevelToItemLevelCurveID`) is not identified; the server uses the static, squished level. Explicit instance bonus lists use the qualified types [above](#instance-properties--2026-10-21); automatic bonus-tree/drop-context selection remains absent.
- **Not fully modeled:** scripted equipment procs, the effects of leech, avoidance and speed (only their sheet percentages), bonus armor, ranged swings. Explicit gem/enchant stats use the qualified instance path [above](#instance-properties--2026-10-21). Auto-attacks swing `main_hand` and `off_hand` ([[player-damage]]).
- **Admin grant:** `grant-equipment` writes equipment without the inventory change queue, so an online player's `EquippedItems` updates at the next inventory change or relog.

### Other
- **Not modeled:**
  - authentic NPC follower gear/stat-budget parity (the approved bridge applies sourced class/spec passive modifiers, not invented NPC budgets);
  - `octclasscombatratingscalar`;
  - multiplicative haste stacking;
  - additional dodge/parry/block aura types beyond handled 49/51.
- **Replication:** `CharacterStats` is server-only. `Health` and `Mana` replicate to every observer; `UnitStats`, `CombatRatings` and `DerivedStats` only to the owning connection (`sheet_visibility.rs`: `SheetOwner(connection)`, set on `PlayerLink` insert, is a replicon visibility filter scoped to those three components). They feed the client's character sheet and its spell tooltips (spell/attack power scaled effect points).
- **Shared formulas:** shared-protocol `formulas/rating_data.rs` interleaves a level column into every sixth entry and ends at level 80, and `rating_to_percent` divides by `raw × 100`, which is 100× too small.
  - The server no longer calls any of them.
  - They are scheduled for deletion in shared-protocol.

## Sources

- [stats.rs](../../../crates/server/src/stats.rs) — computation, components, system
- [stats_tests.rs](../../../crates/server/src/stats_tests.rs) — concrete-number and real enter-world/admin set-level tests
- [item_stats.rs](../../../crates/server/src/item_stats.rs) — item stats, armor, weapon damage, scaling tables
- [item_stats_tests.rs](../../../crates/server/src/item_stats_tests.rs) — real-item tooltip tests
- [character_stats_data.rs](../../../crates/server/src/character_stats_data.rs) — generated tables
- [gen_character_stat_tables.py](../../../scripts/gen_character_stat_tables.py) — generator

## See Also

- [[spell-casting]] — reads spell power, attack power, crit and base mana
- [[class-progression]] — `ActiveSpec`, `Level`, admin set-level
- [[combat]] — auto-attack systems
- [[player-damage]] — how AP, SP, crit, haste, versatility and weapons become damage
