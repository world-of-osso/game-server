# Class spell scripts

Spell-specific behaviours beyond generic Retail effects live in `crates/server/src/spell_cast/scripts.rs` and `crates/server/src/auras/scripts.rs`. Generic dispatch alone is not gameplay coverage. See [spell casting](../wiki/systems/spell-casting.md).

Generic guardian actors and native damage transfer have a separate [contract](spell-guardians.md) and [all40-spec/source boundary inventory](../spell-guardians-coverage.md). They do not imply closure of unknown summon Dummy roots.

## What it must do

### First-batch defensive and hit bridges

- [x] Shield Block 2565 casts 132404 on the caster, granting the catalog block aura for six seconds.
- [x] Bloodthirst 23881 retains its damage and 80 raw Rage, heals through 117313 for 3% maximum health, and rolls effect 2's modified chance to apply Enrage 184362.
- [x] Rampage 184367 spends its catalog cost, retains the catalog attack triggers, and applies Enrage 184362.
- [x] Holy Shock 20473 casts 25912 at enemies or 25914 at allies (including self), using their catalog damage/heal formulas and retaining its one Holy Power gain.
- [x] Barkskin 22812's periodic Dummy casts Brambles damage 213709 every catalog tick when Brambles 203953 is present, hits enemies only, and stops on aura removal.
### Thirteenth-batch locally authored hit, rank and channel contracts

- [x] Mortal Strike12294 applies named115804 healing reduction; Rend772 applies named388539 bleed with native ticks and lifetime.
- [x] Execute163201 with316405 refunds s2 percent of actual raw Rage paid, including OptionalCost, only if its instant260798 target survives. Damage interpolation remains uncertified.
- [x] Shadow Word: Death32379 resolves its tooltip's s2 damage variant once; s3/s4 define strict execute threshold/bonus, extended by active364675s1/s2. Native backlash remains once; legacy s1 variant stays uncertified.
- [x] Rapid Fire257044's native333ms period/2000ms duration/initial-period attribute cast named257045 shots and263585 Focus gains; cancellation removes later ticks.
- [x] Bestial Wrath19574 casts named344572 from the actual persistent pet at its own target, without substituting a guardian or owner selection. Owner a108 is not applied as a pet buff: pet a79 binding, CC removal, charge grants and pet damage inheritance remain gaps.
- [x] Aspect of the Wild193530 fires a free named193455 at its primary on activation. Additional-target geometry and ongoing extra-target contract remain underived.
- [x] Haunt48181 resets its caster's running cooldown only when its owned mark is removed on target death, not cancellation/expiry.

[Batch13 proof and all40 table](../spell-dummy13-coverage.md). Local guardian records provide wolf1500ms attack time and Gargoyle51963 binding, but no non-demon owner-stat curve or complete guardian melee damage contract. No guessed inheritance/melee added.

### Twelfth-batch actor/resource/snapshot consumers

- [x] Feral Spirit51533 summons228562's native two29264 Spirit Wolves, using only imported guardian_spells bindings. Summon duration/owner cleanup use native lifecycle; wolf melee, movement immunity, buff/Maelstrom and elemental variants remain uncertified.
- [x] The actual49206 Gargoyle's bound51963 always critically strikes with1242147. Successful owner Runic Power costs increase its native211947 damage aura by1% per displayed point (49206s4), using PowerType's display modifier. Refused/triggered casts do not empower it; other owners/actors are excluded. Non-demon stat inheritance and Army-of-the-Dead entry are not certified.
- [x] Dispatch2098 and Eviscerate196819 scale their native damage by the actual combo points paid at successful launch; later pool changes cannot alter hits. Optional6/7-point variants remain uncertified.
- [x] Rake1822's named155722 bleed snapshots the231052-or405834 stealth damage bonus at application; later stealth/rank removal does not reduce its ticks. Reapplication recomputes the snapshot.
- [x] Unstable Affliction30108 grants its s3 owner-specific spell/periodic damage bonus while owned233490 exists, once rather than once per instance. Foreign casters and expired marks do not benefit. Melee/direct damage, multi-instance, dispel and death-refund contracts remain open.
- [x] Shield of the Righteous53600 selects enemies inside its native6yd/120-degree caster-facing cone and applies132403's primary-stat armor modifier once per successful cast, including empty cones. Optional heal triggers and duration extension remain open.
- [x] Butchery212436 with459868 applies its named459870 bleed to each successful enemy damage target, excluding allies/out-of-area units; native tick/expiry remain active. Existing294029 cooldown reduction is retained; secondary soft-cap curve remains open.

Sources: local Mainline12.1.0.69933 DB2/read-only world.db; guardian_spells from TDB1210. `crates/server/src/spell_cast/dummy12_tests.rs`: ten native functions across eight paths; inherited Rapture fixture adds30108 dependency and native10% bonus. Single post-origin/master spell_cast gate325/325 ata995632; fmt/check pass. [Coverage/all40 table](../spell-dummy12-coverage.md), receipts and RED/GREEN ledger: game-engine/data/diagnostics/spell-dummy12-2026-10-10/. No TrinityCore C++ body, invented actor attack, IDs or numeric coefficients.

### First-batch channel bridge

- [x] Penance 47540 starts the catalog friendly47757 or hostile47758 channel, delivering three catalog bolts and stopping future bolts when cancelled.
### Second-batch rotation and resource scripts

- [x] Cobra Shot 193455 applies its authored early-level tooltip damage multiplier and reduces Kill Command 34026's running cooldown/recharge by its effect 2 duration; never starts a cooldown on a ready ability.
- [x] Blackout Kick 100784 with rank two 261917 reduces the running Rising Sun Kick recharge and Fists of Fury cooldown by its effect 2 milliseconds; without rank two neither changes.
- [x] Backstab 53 applies its authored early-level damage multiplier and deals its effect 3 percentage extra damage from behind the target; front hits keep base damage and both retain the combo point gain.
- [x] Pyre 357211 resolves its impact393568 into explosion357212 at the target location, dealing catalog damage to nearby enemies, not allies/out-of-radius enemies.
- [x] Drain Soul 198590 reads its effect 1 bonus and effect 2 health threshold on each periodic damage tick. Death removal awards205292 (one whole Soul Shard); expiry and cancellation do not.
### Second-batch critical, targeting and defensive scripts

- [x] Fire Blast108853 always critically strikes, including a caster with zero critical-strike stats.
- [x] Death Coil47541 damages an enemy via47632 or heals an Undead ally via47633, including Lichborne self-healing. Living allies fail before spending Runic Power. It extends the caster’s existing Dread/Virulent Plagues by the catalog Outbreak effect2 duration without restarting ticks.
- [x] Chain Heal1064 heals its explicit ally then three nearby allies, prioritizing greatest missing health within each catalog jump radius and reducing healing by the effect1 percentage per jump.
- [x] Demon Spikes203720 applies203819 for its catalog twelve-second duration. Armor tracks75% of live Agility via aura268; parry effect0 is excluded unless Deflecting Spikes321028 is present.
### Third-batch locally derived paths

- [x] Prayer of Healing596 heals its primary target plus at most four injured allies inside its catalog40yd radius, excluding enemies and full-health additional allies.
- [x] Garrote703 with196861, applied from regular Stealth1784, applies1330 silence and256148 damage reduction for their catalog five seconds. Without rank or Stealth neither applies; a silence-immune target receives no silence-dependent damage reduction.
- [x] Flash Heal2061 reduces Holy Word: Serenity2050's running recharge by the authored six seconds, sends a charge update and never starts a ready cooldown.
- [x] Scorch2948 guarantees a critical strike strictly below its authored30% health threshold; at the threshold normal critical-strike chance applies.
- [x] Shred5221 with231063 deals the authored25% extra damage against a bleeding target; no bonus without either requirement.
- [x] Thunder Clap6343 applies435203 slow to each enemy its damage hits, not allies or out-of-radius units; slow expires normally.
- [x] Devastate20243 with231834 rolls that rank's modified chance to reset a running Shield Slam23922 cooldown and notifies the owner; absent rank/zero chance preserves the timer.
- [x] Moonfire8921 resolves164812's base direct damage and periodic damage, without applying the child's optional snare to an untalented cast.

### Fourth-batch locally derived paths

- [x] Light of Dawn85222 retains its catalog area/radius and heals at most five injured allies; healthy units do not consume the cap.
- [x] Steady Shot56641 generates the Focus authored by77443 on a landed hit.
- [x] Kill Shot53351 rejects targets at or above its20% health threshold before cost/cooldown; strictly below is admitted.
- [x] Mangle33917 with231064 deals20% additional damage against an active bleed, without bonus if either requirement is absent.
- [x] Renewing Mist115151 applies119611's base periodic healing aura to its friendly target, ticks at catalog timing and expires at its catalog duration.
- [x] Vivify116670 with274586 heals allies bearing the caster's Renewing Mist119611 through425804, not other Monks' marks.
- [x] Chain Lightning188443 with343725 generates the passive's two Maelstrom per successful target hit; no passive means no generation.
- [x] Howling Blast49184 applies55095's base periodic disease to each enemy hit; allies/out-of-radius units are excluded.
- [x] Prayer of Healing596 reduces Sanctify34861's running recharge by its authored six seconds and sends an owner update.
- [x] Eruption395160 with395153 extends existing owned Ebon Might395152/395296 by the passive's1000ms once per cast, without restarting ticks or creating buffs.

### Fifth-batch locally derived paths

- [x] Demon's Bite162243 with Insatiable Hunger258876 generates the rank's authored5–10 additional Fury; absent rank keeps base generation.
- [x] Chaos Strike162794 rolls197125's authored chance once per landed root target and refunds through193840, not once per weapon child.
- [x] Disintegrate356995 applies its named370898 slow at channel application; catalog damage ticks and slow expiry remain independent.
- [x] Chaos Bolt116858 always critically strikes and scales its damage with the caster's critical-strike percentage.
- [x] Malefic Rapture324536 erupts owned damaging periodic effects through324540, scaling per effect; foreign auras and periodic heals do not contribute.
- [x] Shuriken Storm197835 generates its authored combo points per enemy hit, not for allies or out-of-radius enemies.
- [x] Death's Caress195292 adds its authored two Bone Shield195181 stacks, capped by the catalog, while retaining damage/Blood Plague.
- [x] Epidemic207317 erupts owned Virulent Plagues through212739 with distinct primary/nearby damage and extends owned Dread/Virulent Plagues by77575's authored1000ms.
- [x] Lava Burst51505's damage child285452 guarantees a critical strike against Flame Shock, scales damage with crit percentage, and retains its existing generic343725-modified eight Maelstrom generation (not a new resource script).

### Sixth-batch locally derived paths

- [x] Sunfire93402 resolves164815's direct/periodic base effects; Improved Sunfire231050 spreads only the periodic effect to nearby enemies within the child radius. Optional aura271/537 effects are not applied blindly.
- [x] Templar's Verdict85256 resolves224266's named damage effect; the child's unidentified Dummy markers remain unsupported.
- [x] Ice Lance30455 resolves its named228598 damage child; Freezing/shatter is not certified.
- [x] Starfire194153 deals full primary damage and its authored70% secondary damage inside the native area.
- [x] Keg Smash121253 deals its authored50% primary-target bonus while retaining native area damage and slow.
- [x] Butchery212436 with294029 reduces Wildfire Bomb259495's existing recharge by its authored3000ms per successful damage-effect hit (including full absorption), up to five hits; no rank or ready recharge means no change.
- [x] Wild Growth48438 applies its base periodic heal to at most five injured allies in the native target-centered area; healthy units do not consume slots.
- [x] Shadow Word: Death32379 applies its authored5% caster-max-health backlash if the target survives the damage, not on a kill.
- [x] Starfall191034 with327541 extends owned Moonfire164812/Sunfire164815 by three seconds, capped at28 seconds remaining, preserving existing tick clocks and foreign marks.

### Seventh-batch locally derived paths

- [x] Colossus Smash167105 applies208086's base caster-owned30% vulnerability to each enemy its damage hits, after that initial damage. Foreign attackers do not benefit; native expiry remains active.
- [x] Survival Instincts61336 applies50322's base50% damage reduction on application and removes the owned child when the parent ends, including cancellation.
- [x] Heart Strike206930 with Heartbreaker221536 grants210738's authored20 raw Runic Power per native chain target; absent talent grants no bonus.
- [x] Power Word: Radiance194509 heals its primary and at most four injured allies inside the catalog radius, prioritizing allies without this priest's Atonement. Foreign marks do not demote allies.
- [x] Raging Blow85288 with383854 rolls its authored chance once at the root hit, applies280270 and restores one used charge, sending an owner update. No rank/failed roll preserves recharge.
- [x] Whirlwind190411 grants its authored base3 Rage;12950 adds1 per enemy up to five, applying85739 at its modified maximum stacks when enemies are present. Selection occurs before its damage children.
- [x] Verdant Embrace360995 with Panacea387761 instantly heals the caster through387763, not the selected ally; absent talent grants no self heal.
- [x] Rake1822 with231052 or405834 and active stealth gains its authored60% initial-damage bonus and163505 stun. Missing either condition preserves ordinary initial damage/DoT/combo point.

These are bounded paths, not whole abilities. Colossus optional343/In for the Kill, Survival child Dummy2, Heart Strike rank316575/front-arc targeting, Radiance raid-pet priority (Atonement application/duration follows in batch9), Raging Blow other reset talents, Whirlwind copied cleave/consumption and soft cap, Verdant flight/primary heal, and Rake periodic snapshot remain uncertified. Sources: read-only TDB1210.26091 archive registrations/proc/link rows, TC a352b1fa, and local Mainline12.1.0.69933 descriptions/effects. Archive names route investigation; they are not standalone behavior proof. No spell_bonus_data table exists in this archive; DB2 SP/AP/scaling fields supply its numerical equivalents.

### Eighth-batch locally derived paths

- [x] Reap1226019 launches1225827 along its87/134 caster-to-destination line using the authored radius/width. Multiple hostile units ahead can be hit; allies, units behind, beyond radius and in another map are excluded.
- [x] Provoke115546 rejects ordinary allies and foreign Black Ox statues61146 before cooldown/cost.116189 actually equalizes highest threat, changes the creature victim and holds it for the catalog6000ms, with800% increased threat from that Monk. An owned statue uses118635's catalog area, excluding allies/far enemies; removal/expiry ends the hold and threat bonus.
- [x] Demonbolt264178 grants280127's20 raw Soul Shards once AfterCast while retaining native damage.
- [x] Crash Lightning187874 uses its authored120-degree destination-directed cone/radius8 and grants187878's12-second enhancement. Current Mainline's unconditional tooltip takes precedence over the older TDB two-target gate.
- [x] Breath of Fire115181 uses its90-degree destination-directed cone/radius12. Targets carrying this Monk's Keg Smash121253 gain123725 periodic damage and deal its authored5% less damage to that Monk until expiry; foreign marks and enemies behind are excluded. This consumes a spell-specific named field, not a generic redefinition of aura269.
- [x] Fan of Knives51723 awards its authored one combo point once per cast with a successful damage hit, not per enemy; an empty area awards none.
- [x] Holy Word: Sanctify34861 heals at most five injured allies in its requested ground area; healthy units do not consume slots. Existing Prayer recharge reduction remains.
- [x] Incinerate29722 generates244670's two base Soul Shard Fragments once for root damage, not per splash target.
- [x] Unstable Affliction30108 applies233490's named base periodic damage to the enemy, ticks and expires normally.
- [x] Arcane Barrage44425 consumes all Arcane Charges AfterCast.

These ten paths are bounded, not whole-ability parity. Reap soul collection/optional Fury; Provoke optional speed/overlapping or reapplied taunts; Demonbolt zero/460553 markers; Crash cleave consumption/procs/split damage; Breath secondary falloff and combat-reach geometry; Fan poisons/soft cap; Sanctify Dummy3; Incinerate critical/talent additions; Affliction five-instance accumulation/vulnerability/dispel/death-refund; Barrage charge-scaled damage/secondary hits/mana refund remain uncertified or unimplemented. Sources: local Mainline12.1.0.69933 CSV/imported rows and TDB1210 archive routing; no invented IDs or world.db writes. [Census and proof](../wiki/systems/spell-casting.md#eighth-batch-projectile-taunt-and-low-coverage-paths).

### Ninth-batch locally derived paths

- [x] Healing Rain73920 pulses73921 at its fixed requested ground center every authored2000ms, for the catalog18000ms, healing at most six injured allies inside the authored11yd radius. Recasting replaces only that caster's previous field.
- [x] Earthquake61882 pulses77478's damage at the requested ground center every1000ms for six seconds in the authored8yd radius; separate casts overlap. Knockdown is not substituted with damage or an invented stun.
- [x] Emerald Blossom355913 anchors at its selected ally's original location, then after1500ms heals at most three injured allies within its authored10yd radius through355916.
- [x] Remorseless Winter196770 damage child196771 applies211793's base20% slow only to enemies actually hit; the slow expires natively. Missing local child radius remains an uncertified range boundary.
- [x] Scourge Strike55090 applies191587's base periodic effect if this caster has no existing Virulent Plague on its enemy; existing marks are not refreshed as a substitute for spreading.
- [x] Metamorphosis187827's additional15 Fury modifier for Fracture263642 requires321068; absent Metamorphosis or the rank preserves base25 Fury. Soul Fragment creation is separate.
- [x] Void Step1234796 teleports to the validated requested same-map destination; missing/out-of-range destinations fail before cooldown or cost.
- [x] Radiance194509 applies194384 Atonement to its selected allies for60% of normal duration (9000ms), keeping foreign marks independent.
- [x] Tranquility740's self shield absorbs up to its named60% of caster maximum health, ending with the channel. Shield depletion does not stop the channel's healing.

Rain, Earthquake and Void Step require a finite requested destination within the native spell range; missing or invalid destinations fail before cost/cooldown rather than silently centering on the caster.

These nine paths do not certify whole abilities. Rain tooltip total-healing expression references six two-second intervals while current native duration is18sec; native duration/period drive this bounded path. Rain's14/magic-reduction markers and1298679 child; Quake chance/knockdown; Blossom child markers/talent variations; Winter absent child radius and root25 marker; Scourge plague eruption/spread and3 marker; Fracture souls/root marker and Meta periodic marker; Atonement transfer and other Dummy fields; Tranquility soft-cap/HoT extension/ally marker remain unresolved. Sources are only local Mainline DB2/imported read-only rows and inherited TDB1210 routing, not unavailable TrinityCore C++ bodies. [Scope and proof](../wiki/systems/spell-casting.md#ninth-batch-local-mainline-ground-fields-and-owned-marks).

## How it works

- [Spell scripts and coverage boundaries](../wiki/systems/spell-casting.md#class-dummy-scripts)

## Implementation inventory

- `spell_cast/scripts.rs` — spell-ID hit, damage, crit, target-plan and cooldown hooks.
- `spell_cast/mod.rs`, `spell_cast/effects.rs` — validation, channel construction and normal effect execution.
- `auras/scripts.rs`, `auras/mod.rs` — periodic/removal callbacks and owned-duration extension.
- `stats.rs`, `auras/{handlers,stat_mods}.rs` — live stat-derived armor.
- `spell_cast/scripts/{batch3,batch4,batch5,batch6,batch7,batch8,batch9}.rs` — locally sourced bounded paths.
- `spell_cast/{dummy_tests,dummy2_tests,dummy3_tests,dummy4_tests,dummy5_tests,dummy6_tests,dummy7_tests,dummy8_tests,dummy8_defense_tests,dummy9_tests}.rs` — real-ID casts and observable outcomes. Paths relative to `crates/server/src/`.

## Tests asserting this spec

- `crates/server/src/spell_cast/dummy_tests.rs` — `dummy_warrior_*`, `dummy_paladin_*`, `dummy_druid_*`, `dummy_priest_*`; Bloodthirst chance endpoints use explicitly labelled catalog clones, while the ordinary cast uses unchanged data.

- `crates/server/src/spell_cast/dummy2_tests.rs` — 24 native cast tests across nine class scripts, including tooltip level curves, conditional ranks, timer changes, cancellation/death and live armor changes.

- `crates/server/src/spell_cast/dummy3_tests.rs` — 16 per-spell cast tests, chance endpoints, radius/threshold/condition boundaries and Garrote silence immunity. [Census and surviving proof](../wiki/systems/spell-casting.md#third-batch-local-retail-rows).

- `crates/server/src/spell_cast/dummy4_tests.rs` —17 cast tests across ten paths, including passive/bleed/ownership gates, strict threshold, area cap/radius, ticking/expiry and existing-timer-only updates. [Census and proof](../wiki/systems/spell-casting.md#fourth-batch-local-retail-rows).

- `crates/server/src/spell_cast/dummy5_tests.rs` —14 per-spell authentic catalog cast functions with chance/ownership/rank/cap gates. Full spell_cast207/207 at`f6b842a`; [census and proof](../wiki/systems/spell-casting.md#fifth-batch-local-retail-rows).

- `crates/server/src/spell_cast/dummy6_tests.rs` — 17 real catalog cast functions for the nine bounded paths, full spell_cast224/224 at`7cff842`; [scope and current proof](../wiki/systems/spell-casting.md#sixth-batch-local-retail-rows).

- `crates/server/src/spell_cast/dummy7_tests.rs` —21 authentic-data cast functions, including100 Survival variance samples and one-charge/oldest-recharge preservation. Full filter ran once244/245; Bear's existing3% Fire mitigation corrected in the sole failing fixture, then batch21/21 at`81c2bf6`. No245/245 broad-pass claim; [census and proof](../wiki/systems/spell-casting.md#seventh-batch-tdb-routed-retail-derivations).

- `crates/server/src/spell_cast/dummy8_tests.rs`, `dummy8_defense_tests.rs` —13 cast functions plus one real incoming-damage/expiry function across ten paths. Targeted13/13 at`70c4f20`, defense1/1 at`2d883e8`; single post-master broad spell_cast run261/261 at`2fb37d8`. Format/check pass; [census and proof](../wiki/systems/spell-casting.md#eighth-batch-projectile-taunt-and-low-coverage-paths).

- `crates/server/src/spell_cast/dummy9_tests.rs` —14 native functions across nine bounded paths. Requested post-master `spell_cast` filter ran once at`314a8f1`:274/275 (all14 new functions passed). The sole obsolete Blossom no-action fixture was corrected at`521455e` and passes its focused rerun. Separate shield-depletion RED at`43701c9` led to`e78fad0`; absorb regressions5/5 and both Tranquility functions2/2 pass, with final format/check. No fresh275/275 broad-pass claim; [census and proof](../wiki/systems/spell-casting.md#ninth-batch-local-mainline-ground-fields-and-owned-marks).

### Tenth batch: low-count spec paths

- [x] Blade Flurry13877 launches the named331850 initial strike in its native8yd area: five targets, eight with active272026. Active381878 grants its s2 combo points per landed initial hit. Subsequent28% damage transfer and unknown markers remain unsupported.
- [x] Consecration26573 creates one owned fixed-position81297 damage field; native8yd area, t1=1000ms, native duration, tooltip1.05 coefficient; recast replaces only that owner's field. Optional slow/Radiant variants and summoned actor parity remain open.
- [x] Feral Lunge196884 launches215802 at the selected enemy and preserves existing generic254 endpoint movement; traversal/visual parity is not certified.
- [x] Voidform228260 applies the named194249 damage effects for their native duration after cast; no guessed Volley/replacement charge behavior.
- [x] Arcane Surge365350 applies named365362 damage/regeneration effects after cast. Mana-spent damage, soft-cap and Clearcasting remain open.
- [x] Protection Judgment275779 with active315867 launches220637 Holy Power once on its damage hit; no rank means no resource award.
- [x] Hand of Gul'dan105174 launches86040 at the target's native8yd area, multiplying damage by root s1=3 with the unchanged30-raw shard cost. Imps, Doom and variable-shard casting remain unsupported.
- [x] Lightning Bolt188196 with active343725 awards its s1=6 Maelstrom once on a landed bolt. Maelstrom Weapon consumption/variants remain open.

Sources: local Mainline12.1.0.69933 DB2 only; no TrinityCore implementation bodies or invented child IDs. Divine Storm's reduced-damage formula and Glacial Advance's advancing geometry were not derivable from these sources and remain deferred.

- `crates/server/src/spell_cast/dummy10_tests.rs` —11 native functions across eight paths at`83512eb`: actual damage/area/resources/buff expiry, absent passives, owner-only replacement and cancelled casts. Single post-master `cargo test -p server --bin game-server spell_cast` at`4b281cf`: **286/286**. [Census and full40-spec table](../wiki/systems/spell-casting.md#tenth-batch-low-count-spec-paths).

### Eleventh batch: bounded low-count paths

- [x] Sinister Strike193315 uses description-variable338's exact level curve, including the level10/20 clamps.
- [x] Void Volley1242173 fires ten native1242189 bolts at the primary and one at each nearby enemy in its authored radius; native1000 raw Insanity is retained. Voidform228260 launches its initial volley after completion. Inter-bolt sequencing and replacement charges are not certified.
- [x] Rain of Fire5740 launches42223 damage in a fixed8yd destination field at native1000ms intervals for8000ms; overlaps retain independent clocks, with no ally damage or pulses after expiry.
- [x] Blade of Justice184575's native404358 splash requires active403826, without duplicating the root's Holy Power.
- [x] Hammer of the Righteous53595 launches only88263's named damage at other nearby enemies while the caster is inside their own active Consecration26573. The unavailable203785 variant remains unsupported.
- [x] Shred5221 with343232 or405834 plus active stealth receives s3=60% extra damage and double modified crit chance, with the tooltip's343232s1 extra combo point. Existing bleeding-rank bonus is retained. Thrash-on-Shred405300 remains separate.
- [x] Thrash106830 applies the named405233 periodic bleed only to hit enemies; native ticks and expiry apply. Arcane variant429523 remains unsupported.
- [x] Lava Lash60103 with334046 applies334168; caster-owned aura271's named334168 consumer respects its Flame Shock family mask, so own Flame Shock gains100% damage but unrelated Lightning Bolt and foreign attackers do not. Other271 mask contracts are not generalized; Colossus Smash's existing all-owner-damage contract is preserved.

Sources: only local Mainline12.1.0.69933 descriptions/effects/variables and read-only imported rows. No invented IDs or source-C++ claim. `crates/server/src/spell_cast/dummy11_tests.rs` exercises these paths: final13/13 plus inherited Colossus1/1, format/check exit0 at`bce19e3`. Requested post-master filter ran once298/299 before rejecting the universal271 rule; no renewed299/299 broad claim; [scope/proof](../wiki/systems/spell-casting.md#eleventh-batch-local-mainline-low-count-paths).

### Shared resource mechanics

- [x] Flat `SpellPower.OptionalCost` spends additional available raw power, up to its authored bound, without making that optional amount an admission requirement. Missing required power refuses before any pool is changed.
- [x] `SpellDuration.DurationPerResource` uses the successful cast's combo-point expenditure, respects native `MaxDuration`, and survives later pool changes. Rupture1943 and Rip1079 retain8/16/24-second native durations for1/3/5 points; periodic-Dummy and other unidentified fields remain unsupported.
- [x] Envenom32645 and Black Powder319175 multiply their named damage effect by combo points actually spent. Envenom's native buff duration uses the same snapshot. Native Black Powder area/ally/far selection is retained; its soft-cap and Find Weakness branches are not certified.
- [x] Arcane Barrage44425 snapshots36032s2 charge damage before its existing AfterCast consumes charges. Active321526 refunds its authored hundredths-percent of maximum mana per charge. Missing rank gives no refund; missile impact cannot reread the cleared charges. Secondary231564 hits remain unsupported.
- [x] Arcane Surge365350 spends its native flat optional mana at successful execution; cancellation spends nothing. Its exact mana-to-damage curve is undetermined: the tooltip's maximum does not establish interpolation. No guessed linear formula or457521 modifier is retained.

Scope/ranking: [all40-spec table](../spell-mechanics-coverage.md). Flat optional costs affect six ranked roots/seven memberships; native per-resource duration affects three/four (overlapping roots, not nine unique closures). Actor family ties at six roots but has no established native spell rotation/budget mapping. Damage-copy fields and actor scripts remain unresolved. No world.db writes or new importer is needed for these resource paths; `OptionalCostPct`, resource-rank cap extensions and whole-ability parity are not certified.

## Known gaps (current cycle)

- [ ] Unknown Dummy fields remain unsupported; no generic no-op handlers. Source diagnostics list remaining marker and talent contracts.

## Out of scope

- Full talent/hero variants: Fresh Meat's guarantee, Enrage talent gating and Rampage's optional AoE branch are not certified by these base-script tests.
- Penance Atonement, Power of the Dark Side and Dark Reprimand variants are not certified by the base-channel tests.
- Second-batch optional/uncertified paths: Teachings/normalization, Find Weakness, Volatility, Unholy Vigor, Chain Heal additional-jump LOS/talent range, Calcified Spikes/Retaliation, Fire Blast parallel casting/Hot Streak.
- Third-batch unimplemented variants: Improved Garrote windows/other concealment spells; Prayer selection injury priority/LOS beyond ordinary cast validation; Prayer-to-Serenity optional1246517; Shred405300 Thrash-on-Shred variant (stealth completed in batch11); Thunder Clap reduced damage beyond five targets/root variant; Moonfire second target, damage reduction and Astral Power variants. Unknown root/descendant markers are not certified by base-path proof.
- Fourth-batch limits: unknown Dummy metadata remains unresolved. Steady Shot Marksmanship cooldown/level curve, Kill Shot enabling talents, Mangle marker, Renewing Mist overheal jumps/rank231606 procs, Vivify tooltip multiplier/talent variants, Chain Lightning Maelstrom Weapon/target-count marker, Howling Blast secondary damage/CAP/Frost Fever resource procs, Sanctify extra marker (target cap completed in batch8), Ebon Might stat distribution and extension caps are not certified.
- Fifth-batch limits: unidentified Demon's Bite/Disintegrate/Chaos Bolt markers; Chaos Strike's30 mirror is not separately consumed; Shuriken soft-cap/stealth/talent fields; Bone Shield consumption/haste/armor parity and Blood Plague leech; latent plague talent effects. Crash Lightning's native104 cone and initial enhancement are completed in batch8; cleave/proc damage remains uncertified. These base-path tests do not certify full class rotations.
- Sixth-batch limits: unknown root/child markers; Starfire/Keg/Butchery reduced-damage curves beyond their soft caps; Starfire429523 variant; Keg Shuffle/Brew reduction; Wild Growth declining tick curve/Tree of Life targets; Shadow Word: Death execute/damage-effect variants; Ice Lance Freezing/shatter; Starfall overlap/base tick child contracts. Selector104's authored destination-directed cone is exercised by batch8 Crash Lightning/Breath of Fire; unit-center geometry does not certify combat-reach edge expansion or scope-wide rotations.
- Realm UDP 5000, deployment and merging.
