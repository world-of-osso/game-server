# Native item-instance DPS parity

Compare native 70448 server combat with shipped MID2 SimulationCraft 76f8118 profiles; Evoker uses its explicitly approved prior-tier MID1 reference. Sources: `scripts/dps_parity.py`, `scripts/item_instance_data.py`, `crates/server/src/item_instance.rs`; [character stats](../wiki/systems/character-stats.md) describes the existing production stat path.

## What it must do

- Tank DPS comparisons with `DPS_PARITY_TANK_TRACE_DIR` replay the matching `<spec>.<targets>.<seed>.simc.log` through the existing tank replay. The recorded dummy level, armor coefficient, incoming attacks, pauses and external heals apply to the selected 1T or 3T fixture. Count damage to all fixture targets and preserve native incoming-hit resource and mitigation behavior.
- Blood Boiling Point's delayed Blood Boil echo is mirrored by each active Dancing Rune Weapon, as is the foreground Blood Boil. An echo consumes an intervening Boiling Point proc and schedules another echo. A new empowered cast forces any pending echo while the same damage modifier applies to both casts and their weapon copies (SimC `sc_death_knight.cpp:9051–9079`). Blood Vampiric Strike earns Heartbreaker's runic power for each enemy hit, like Heart Strike (`sc_death_knight.cpp:11282–11289,11322–11325`).
- Blood Beast inherits its owner's level for native combat and summon stats (SimC `pet.cpp:69`) and starts Corrupted Blood on cooldown (`sc_death_knight.cpp:4754–4758`). Rune Weapon Blood Plague continues through its remaining duration after the weapon is dismissed; a dismissed weapon cannot mirror further attacks. Blood's disease ticks can apply the selected Brittle debuff (`sc_death_knight.cpp:7809–7821`). Rune Weapon Death Strike copies execute before the owner heals and consumes Hemostasis (`sc_death_knight.cpp:10116–10130`).
- Guardian Twin Moonfire copies foreground and Galactic Guardian Moonfire to one other enemy. Prefer an undotted enemy; foreground casts prefer the shortest remaining Moonfire (SimC `sc_druid.cpp:7897–7944,14025–14072`).

- Foreground literal actions within SimC's default 250 ms cooldown tolerance may be selected and queued. Execute the selected action only after its native cooldown or charge is ready; do not cast a lower-priority action in the meantime. Recheck native admission at execution. Native off-GCD and item-use actions do not use this queue.
- [ ] SimC also queues some `use_while_casting` off-GCD actions; native does not. In the Fire seed-70448 1T trace, SimC queues Combustion twice and cancels both when Fireball finishes, so neither executes. The DPS effect of other cases is not established.

- Literal rows inherit `use_off_gcd` set by pinned SimC class action constructors even when the row omits the option. Native admission still enforces the spell's own GCD, costs and cooldowns; SimC only schedules off-GCD actions when their trigger GCD is zero. The constructor inventory covers Warrior stances, Ignore Pain, Berserker Rage, Spell Reflection and Taunt; Evoker Quell; Demon Hunter Demon Spikes and Fiery Brand; Death Knight Dark Command; Druid interrupts, Cancel Form, Growl, Ironfur, Barkskin, Prowl, Survival Instincts and Auto Attack; Shadow Void Torrent; Monk Black Ox Brew and Purifying Brew; Paladin Divine Protection, Ardent Defender, Guardian of Ancient Kings, Lay on Hands, Hand of Reckoning and Shield of the Righteous. Enemy auto-attacks and Druid's background Heart of the Wild heal are not literal player actions. Shield Block and Celestial Brew do not set intrinsic `use_off_gcd` in this revision.
- Monk `CAST_DURING` actions are admitted during the named Spinning Crane Kick channel; Zenith Stomp is also admitted during Celestial Conduit. The original channel remains active after an admitted instant action, and ordinary GCD, resource and cooldown checks still apply. Brewmaster's recast of Spinning Crane Kick itself remains unimplemented: SimC models it as an unchanneled extending ground event, whereas native 322729 currently owns a channel. Devourer fragment pickup retains its separately qualified cast-during policy.

- Parity measurements fail if the selected actor cannot run the spec's registered literal rotation; an assisted player or unrelated follower kit is not a valid substitute. In the ordinary Patchwerk add-wave fixture, `raid_event.adds.has_boss` is false: SimC reserves boss metadata for pull events (raid_event.cpp3193–3200). Dungeon pull metadata is not inferred from this fixture.
- Player weapon auto-attack damage applies the attack-power multiplier to the weapon roll as well as the attack-power contribution (SimC76f8118 `action.cpp:1429–1439`).
- Equipped Call the Swarm1310082 delivers Acidbrood1310088 through native missile1310083, and equipped Void Eruption1310208 delivers1310209. Payload damage uses the originating item's native driver effect1 budget and native role multiplier, not the zero-valued damage-leaf points. Preserve native proc eligibility/RPPM/ICD, travel, crit and full unsplit AoE.
- The pinned `bugs=true` Void Eruption behavior does not apply its inactive Venomcursed15% bonus. Casting-item context must survive native aura proc children and their missile/trigger descendants; class procs without an item retain their existing context.
- An Antoran Tyrant uses the scheduled Burning Cleave action as its only swing; native1251000 must not also emit a separate white-swing replacement (`sc_warlock_pets.cpp1510–1573`).

- The native encounter fixture faces the actor toward its actual primary before engagement and restores maintained melee-intent facing after forced movement finishes, including a charge that ends inside melee range. Do not turn active channels or override forced movement. Keep native directional selectors, widths and radii unchanged.
- Melee swings, melee-range spell admission, literal target readiness, threat selection and fixture return use the same native attacker and target combat reaches plus 4/3 yards, with a 5-yard minimum. Preserve native model data and range modifiers.
- A fixture actor with native melee AutoAttack intent walks back into its actual melee range at its native run speed after movement abilities. Preserve forced movement ownership, native movement/cast restrictions and position=back; never suppress displacement, snap the actor or grant resources. This shared controller applies without specialization or spell-ID cases (SimC76f8118 sc_demon_hunter.cpp1559–1611).

### Unholy Magus

- The real Magus's Withering Grasp periodic damage can critically strike with its owner's live critical chance and its own critical damage modifiers (SimC76f8118 `sc_death_knight.cpp3070–3079/4505–4511`).
### Unholy Commander of the Dead

- Actual Dark Transformation grants the selected native Commander owner buff. While it is active, eligible Magi, Lords, lesser ghouls, Gargoyles and Abominations receive its native pet damage bonus, including guardians summoned during the window; permanent ghouls and Horsemen do not (SimC76f8118 `sc_death_knight.cpp2823–2833/3627/3853/4473/5483/8761–8762`).

### Owned pet target damage

- A target aura's native effect381 modifies damage from its caster's controlled pets, while effect380 applies to that caster's guardians. Other owners and non-pet actors do not inherit either effect (SimC76f8118 `sc_death_knight.cpp17013–17025`).
### Caster-owned critical damage

- A target's native effect501 increases only the critical bonus of matching spells from that aura's caster, respecting its family mask and per-effect stack rule. Trickster Surprising Strikes modifies Fazed's effect4 for combo-point generators (SimC76f8118 `sc_rogue.cpp2602–2613`).

### Pet melee haste

- Statless controlled pets and guardians use their owner's live attack haste for melee swings; stat-bearing pets retain their own haste clock (SimC76f8118 `pet.cpp222–228`).
### Unholy Devotion

- Each Putrefy arrival grants the permanent ghoul an independently expiring Unholy Devotion stack. Each stack increases its melee swing speed and Energy regeneration by the native effects (SimC76f8118 `sc_death_knight.cpp3390–3405,3719–3741`).

### Unholy Outnumber

- Each actual lesser ghoul contributes one owner Lesser Ghoul counter stack from arrival to dismissal. Selected Outnumber supplies the counter's native per-stack controlled-pet damage bonus, including temporary Putrefy ghouls (SimC76f8118 `sc_death_knight.cpp3673–3689`).
### Fire Scald

- Selected Scald guarantees Fireball and Fire-specialization Frostfire Bolt critically hit targets at or below its native effect3 health threshold; it does not guarantee Pyroblast (SimC76f8118 `sc_mage.cpp:2299–2305,3850–3857,5769–5772`).

### Umbrelskul's Fractured Heart

- The equipped Heart's native proc applies item-level-scaled Crystal Sickness. Each tick raises its stack up to ten; on a non-boss at or below the native threshold, it executes that target and jumps to another enemy, subject to the native jump-counter cap (SimC76f8118 `unique_gear_dragonflight.cpp:7575–7685`).

### Fire Spontaneous Combustion

- Casting Combustion restores the number of Fire Blast charges in selected Spontaneous Combustion's native effect1 (SimC76f8118 `sc_mage.cpp:3527–3536`).

### Fire Master of Flame

- Selected Master of Flame increases Fireball and Fire Frostfire Bolt damage outside Combustion by the native 1217750 effect1 amount (SimC76f8118 `sc_mage.cpp:3823–3824,3860–3865`).

### Fire Ignite

- Each new Ignite deposit receives selected Slow Burn's native effect2 during Combustion or selected Master of Flame's native effect1 otherwise; existing unpaid Ignite is not rescaled (SimC76f8118 `sc_mage.cpp:1935–1946`).

### Fire Burnout

- When Combustion ends, selected Burnout damages each target with the native talent fraction of that caster's remaining Ignite bank (SimC76f8118 `sc_mage.cpp:1458–1465`).

### Periodic refresh

- An ordinary refreshed periodic damage, leech or healing effect retains its next-tick phase and adds the time to that tick to its fresh duration. Native Pandemic, sourced rolling and extension behavior keep their own duration rules (SimC76f8118 `action.cpp419/4612–4620`).

### Unholy Nazgrim

- Apocalyptic Conquest belongs to the owner while the actual Nazgrim is alive and expires on his demise. Selected Nazgrim's Conquest adds its native effect2 Strength percentage per paid rune, except Scourge Strike uses its base rune cost even when free (SimC76f8118 `sc_death_knight.cpp5405–5414/7332–7373/13100–13120`).

### Frost Death Knight target damage

- Spell modifiers with native effect attribute `0x40` do not multiply their points by aura stacks; other effects on the same aura retain their stack scaling. Killing Machine's fixed Frostscythe critical bonus uses that attribute, while its Killing Streak bonus scales per stack.
- Northwinds gives the second Howling Blast target the same Rime-specific bonuses as the primary; later targets use the pinned reduced-AoE and secondary factors. Cryogenic Chamber banks the native fraction of each earned Rime impact up to its stack cap and releases that bank on Winter.
- Frost Bind in Darkness follows actual Rime acquisition and removal; it must not proc independently. Its active aura converts Howling Blast to Shadowfrost. Preserve other specializations' native proc behavior.

### Native Pandemic duration carry

- Native `PERIODIC_REFRESH_EXTENDS_DURATION` selects Pandemic carry: a refresh retains up to30% of fresh duration from the previous owned application, never reducing a longer remaining lifetime (`action.cpp684–685/4612`). Use the native attribute, not a Rip/Rake whitelist. Existing explicit Extend policies and rolling refresh retain their source precedence (`action.cpp2729–2734`); preserve native periodic phase, amounts and asynchronous deadlines.

### Controlled-pet periodic haste

- Haste-flagged periodic effects cast by a controlled pet use its owner's live spell haste, just as guardian periodic effects do. Unflagged periodic effects keep their native cadence (SimC76f8118 `pet.cpp`217–228, `action.cpp`671).

### Periodic power

- Periodic damage, leech and healing use the caster's current AP/SP at each tick unless a periodic effect has `EX_COMPUTE_ON_CAST`. Preserve the application-time cast and persistent factors, stored payload adjustments, stack rules and tick schedule. Scourge Strike eruptions use the current owned disease tick amount (SimC76f8118 `action.cpp:2787–2818`, `sc_death_knight.cpp:12625–12638`).

### Shared channel timing

- Quantize native channel tick intervals to the nearest millisecond. Attribute 273 hastes channel duration; when periodic haste also applies, rebuild the duration from that rounded interval and the original native modified tick count. Attribute 173 alone hastes ticks without shortening duration, including the final triggered tick at expiry (`action.cpp:670–674,4297–4314,4390–4432`).
- Native DB2 difficulty-0 channels affected by the 173/273 distinction (including NPC and utility spells): 173 without 273: `698, 45789, 54518, 54900, 61994, 66097, 115294, 165721, 175498, 175516, 196997, 197432, 205065, 211474, 212084, 212947, 223922, 261631, 261789, 262607, 262616, 262878, 263165, 272131, 288267, 296823, 315419, 342601, 392451, 410172, 417537, 427688, 432569, 438973, 452486, 463840, 468942, 468979, 470973, 1216571, 1250936, 1252860, 1264367, 1284063, 1287761, 1293918`; 273 without 173: `92380, 141612, 149891, 151566, 163414, 164588, 185152, 203313, 203375, 209315, 209874, 234501, 359073, 367226, 382411, 382731, 385804, 387839, 396286, 408092, 417132, 436358`.
- Cast-time spell modifiers must not alter a channel's duration. Channel span and period retain the pre-execution charged-modifier snapshot when the admitted use is consumed, including later tick interval recalculation; legitimate Duration/Period modifiers remain active in that snapshot. Live uncharged Period modifiers and non-channel periodic behavior remain unchanged. Instant non-channel casts retain cast-time modifier behavior.
- The actor's channel cadence and its periodic aura use the same native effect cadence. Declared damage/healing periods own this cadence ahead of faster auxiliary control pulses; channels without declared health emissions retain their native utility/script period. Constant haste must not create an extra expiry damage event or full proc callbacks from rounding residue. Preserve tick-zero behavior and genuine fractional native tick counts.
- Preserve non-channel periodic quantization, channel modifier/haste snapshots, and un-hasted channel lifetimes, including four-second Convoke. For hasted non-channel periodic effects, use current caster haste when scheduling the interval after each paid tick; do not retime the already-pending tick (SimC76f8118 `action.cpp:2038–2041,2792–2809`). Do not add spell-ID branches or replace native coefficients/periods/durations.

### Elemental Inferno Arc

- Selected Inferno Arc1259047 amplifies Chain Lightning, Lightning Bolt, Elemental Blast, Earthquake damage, Earth Shock and Tempest and their overloads against each recipient with this caster's active Flame Shock. Read the percentage from the native talent (SimC76f8118 `sc_shaman.cpp`6324–6330,6618–6624,7330–7336,7411–7417,7589–7595,7678–7684,7810–7812,8192–8198,8221–8227,9858–9864,10069–10075).

### Shaman pet passive modifiers

- Shaman pets retain eligible owner passive spell modifiers and native base values. Mastery-scaled Shaman action effects use the pet's own mastery, not the owner's mastery; do not remove other owner passives (SimC76f8118 `pet.cpp`217–228, `sc_shaman.cpp`3822–3833,13183–13224).
### Literal interrupt readiness

- Literal interrupt actions require their selected enemy to be casting. Match the pinned class-specific SimC `target_ready` rules; Skull Bash bypasses the Druid casting check for fluid-form shifting (`sc_rogue.cpp4682–4687`, `sc_druid.cpp3573–3578,8174–8181`).

### Stormbound Emblem of Dazar

- A completed or interrupted native1295275 channel grants haste1294745 for one native buff duration per completed tick. Its amount comes from the equipped native1294744 item effect (`unique_gear_midnight.cpp3863–3954`).

### Devourer Soul Immolation

- Each native Soul Immolation1241937 self-HoT tick executes the native Fury payload1242475. The native aura owns its tick timing and payload values (SimC76f8118 `sc_demon_hunter.cpp`5930–5958). Fragment spawning is separate.
### Devourer Focused Ray

- Selected Focused Ray1240203 amplifies Void Ray damage children1213649/1214595 by native effect1 when the real resolved enemy count is at most native effect2 (SimC76f8118 `sc_demon_hunter.cpp`6304–6321).
### Devourer Metamorphosis entry

- Entering native Void Metamorphosis1217607 grants 200 Fury, capped by the actor's native pool (SimC76f8118 `sc_demon_hunter.cpp`12445–12452). Drain and exit behavior are separate.
- Void Metamorphosis damage effects4/5 use Monster Within mastery once, not both their raw aura amount and the mastery-scaled amount (SimC76f8118 `sc_demon_hunter.cpp`2174–2185).

### Devourer Reap

- Each Reap1226019 executes the delayed damage child1225823 once, without an additional legacy1225827 hit (SimC76f8118 `sc_demon_hunter.cpp`5982–5999,6081–6107).
### Devourer Singular Strikes

- Selected Singular Strikes1272770 amplifies only the primary Hungering Slash or Reaper's Toll damage hit1239127 by native effect2; secondary hits remain unchanged (SimC76f8118 `sc_demon_hunter.cpp`6697–6728).

### Devourer The Hunt

- A landed The Hunt or Predator's Wake impact applies the native Hunt DoT1246168 to nearby enemies, including the primary, up to native370967's target cap (SimC76f8118 `sc_demon_hunter.cpp`5488–5511).
- With Demonic Intensity, Collapsing Star resets both The Hunt and Predator's Wake cooldowns, including their shared category (SimC76f8118 `sc_demon_hunter.cpp`6551–6557).

### Devourer Burning Blades

- Selected Burning Blades452408 applies native residual1245654 to landed Devourer weapon swings and Voidblade/Hungering Slash damage children. Preserve the native periodic timing and the existing Havoc residual453177 behavior (SimC76f8118 `sc_demon_hunter.cpp`3088–3111,5830–5847,6697–6710,6875–6878).

### Twisted Horror's Tendril

- Equipped1310404 earns native Devastating Power1310414 stacks from its proc driver, using the driver's item-scaled rating amount. Driver1310446 grants three stacks on its first successful proc for each actual target; its native cooldown remains in force. Applications keep their native independent stack durations (SimC76f8118 `unique_gear_midnight.cpp`4818–4875).
### Hunter Unnatural Causes

- Selected Unnatural Causes459527 increases eligible Hunter and owned-pet damage using the native459529 family masks and labels. Below the native health threshold, increase its native damage bonus by the native talent percentage. Pinned `bugs=true` excludes direct Wildfire Bomb, Volley and Laceration (SimC76f8118 `sc_hunter.cpp`1482–1509,2640–2664,5161–5164,6934–6938,7056–7059).

### Literal wait

- A selected `wait` evaluates `sec` once, holds subsequent actions for that duration without interrupting attacks, and reevaluates the APL afterwards. The default is one second; nonpositive `sec` uses the source `player.available()` delay (SimC76f8118 `player.cpp`7130–7133,9706–9803).

### Literal spec drivers

- Register each literal spec once, in spec-ID order. Adding a spec changes only its own rotation module and its registration line, not shared fields, initialization, dispatch, reporting or preparation branches. See [Arcane Mage parity](arcane-mage-parity.md) for spec62's execution contracts.
- Registered literal drivers apply to both Player and Follower identities unless the spec explicitly opts out. Preserve native actor identity, ordered precombat and existing report labels and keys, including null inactive reports.

### Retribution literal fixture

Instrument of Retribution 404752 grants the native Avenging Wrath 31884 aura for nine seconds when another player in the paladin's party or raid dies within 40 yards in the same map instance. The paladin must be alive and know the passive. Pet, non-group and own deaths do not trigger it. Reapplication follows the native aura refresh rules, including Crusade's existing no-duration-refresh rule; Radiant Glory does not replace the proc aura. The 2026-10-17 Sszorak log records five nine-second 31884 applications near player deaths; it does not establish overlapping-proc or Crusade behavior.

`RET_INSTRUMENT_UNEXPLAINED_DEATH_MISSES`: archive comparisons found some deaths without an observed proc even when recent horizontal positions were within 40 yards, the last logged paladin health was positive, and the death was not flagged unconscious. Positions lack height, talent snapshots can be stale, and aura visibility is incomplete; the data does not establish an additional proc condition. Do not infer a cooldown or subgroup restriction from these samples.

Spec70 uses its pinned literal APL for native Player and Follower identities. Preserve the native actor identity, ordered precombat and existing report contract. With selected Seething Flames405355, Wake255937 and its two flame impacts405345/405350 deal 60% damage to secondary targets (SimC76f8118 `sc_paladin_retribution.cpp764–771,815–819`).

### Feral literal fixture

Spec103 captures the native actor before setup and executes all74 pinned76f8118 Feral rows through shared `Program`, `Driver`, `Prepared` and `Equipment`, for either Player or Follower identity. Preserve ordered native Cat Form/Prowl precombat, the shared potion/item path, native action admission and source-recipient geometry. Do not replace missing periodic snapshot facts or damage mechanics with assisted priorities or synthetic values. Controller qualification is separate from full-duration numerical parity.

Feral `buff.clearcasting.react` observes actual135700 acquisition through the shared reaction clocks configured before precombat. Use the exact pinned103 player's0.25s mean,0.05s deviation,0.1s offset,0.15s exponential term and1.4s maximum; neither buff presence nor query time substitutes for the earned reaction deadline.

### Feral Thriving Growth

Actual owned Rip1079 and Rake155722 periodic damage feeds the selected439528 accumulator using native per-source amounts, native active-dot exponent and selected Feral Green Thumb1270565. Keep the pinned random initial phase, uniform increments and1000-point threshold. Success casts native439531 on the actual tick recipient; other periodic sources cannot roll it (`sc_druid.cpp2416–2448,12311–12406`). Vines applications retain independent native-duration stack deadlines through the existing asynchronous-stack scheduler, following the explicit source override (`sc_druid.cpp13755–13764`); refreshing a later stack cannot extend an earlier one. Native490-flagged auras retain their existing path. Selected Bursting Growth440120 casts native440122 for every actual expired Vines application and after a landed primary Bite on this caster's ticking Vines (`sc_druid.cpp4295–4307,13755–13821`). Preserve native area recipients and coefficients. Actual Vines periodic damage independently rolls selected Rampancy1270586's native effect0 chance and dispatches440122 at native effect2 effectiveness (`sc_druid.cpp4015–4020,11982–11988`). Other ticks cannot roll it; Bite/expiry relationships remain independent. Twin Sprouts and the greater-than-five-target reduction remain separately unqualified.

### Feral Unseen attacks

An actual landed primary Bite with selected1263657 rolls its native chance per frozen effective combo point. The existing cast-resource context retains those points independently of payment; the readonly Bite accessor reads its retained extra-Energy fraction, not its Saber-amplified damage effectiveness. Select an actual enemy inside the talent's native range, and preserve the chosen recipient and native CP-fraction×extra-Energy factor through the pinned400–500ms delayed Unseen1263890 impact and native1271863 bleed. Selected1263658 earns native1263939 for its native duration times the retained points (`sc_druid.cpp2776–2804,4321–4327,4891–4977`). The Unseen bleed pays one actual damage event per scheduled dot tick, using native effect0's coefficient and period. The second native periodic row is not a second SimC dot clock (`action.cpp838–865`); do not compound both rows into duplicate damage. Other registered spender consumers retain their existing resource snapshots. Selected1263660 makes actual Tiger's Fury5217 grant native1263962 at its native maximum stacks. Each eligible ordinary Cat action consumes one stack through its native ICD and emits a guaranteed Unseen attack using maximum effective combo points and full extra-Energy fraction, without granting or spending either pool (`sc_druid.cpp3745–3749,4879,11313–11316`). Apply this once per cast, not once per AoE recipient or triggered child; Cat Form alone cannot grant the uses. Greater-than-three-target Swipe and live-versus-snapshotted Tiger's Fury on Unseen remain separately unqualified.

### Feral Cat Convoke

Selected Cat Convoke391528 dispatches one native child per actual periodic-driver tick for its native channel lifetime. Use the pinned Cat card counts/distributions, persistent exceptional deck, primary-target Bite/Frenzy and random encounter-enemy filler/off-spec selection (`sc_druid.cpp9340,9372–9385,9469–9522,9630–9674,12415`). Use native Enemy admission, not hostile reaction alone; neutral attackable encounter units remain eligible. An already-owned Rake or Cat Moonfire replaces that card with Shred or Wrath, respectively. Free child Bites retain native maximum effective combo points and extra-Energy effectiveness without granting or spending either pool; ordinary spenders keep paid snapshots. Child casts must retain Cat Form. Native aura removal clears channel state; other forms and multi-friendly-target healing remain separately unqualified.

### Feral Lunar Inspiration

Selected native155580 applies its owned155627 override aura, so ordinary Moonfire8921 resolves to Cat Moonfire155625. Removing the talent removes only its owned override; reapplication restores it without changing learned spells. Native Cat Form must admit the resolved ordinary cast. SimC76f8118 `sc_druid.cpp4457–4465` supplies selected155625 behavior; TC `spell_dru_lunar_inspiration1543–1567` supplies the native aura lifecycle.

### Shared merged critical feedback

Native merged buff outcomes apply or clear the caster-owned aura after the hit batch, retaining native amounts, caps and duration. A grant is not another spell cast and must not emit cast/hit proc inputs or earn unrelated equipment buffs. Arcane/Frost Overflowing Energy and Fire feedback share this path (`sc_mage.cpp7272–7293/5585–5605`).

### Shared Spellslinger producers

- Selected Splintering Sorcery443739 emits the native specialization payload (Arcane443763/Frost443722) after native Arcane Blast, Prismatic Bolt, Frostbolt, Flurry and Glacial Spike executions. Signature Spell supplies its authored extra conjures. All queued recipients retain the actual hostile primary; individual launches are staggered100ms as pinned `sc_mage.cpp7324–7361` specifies.
- Successful native Ray of Frost execution opens selected native1247908 Splinterstorm once, before its periodic updates. Each damage tick conjures the native Splinterstorm effect3 amount without refreshing the window. Conjures during the actual earned Splinterstorm/Arcane Surge lifetime use the selected native doubling chance. Native spell records retain duration and damage coefficients (`sc_mage.cpp4992–4996/5021`).

- Each actual native Splinter impact with selected Spellfrost Teachings444986 reduces the specialization's running Orb cooldown/recharge by its native effect (Arcane index0/Frost index1, milliseconds). The shared reducer updates active native spell, category and charge timers without starting inactive timers; cooldown packets retain the native category and full duration. Unrelated payloads do not reduce recharge.

- Native Arcane/Frost Splinter impacts with selected Controlled Instincts copy the native specialization fraction of the real resolved hit to secondary enemies around that hit, excluding the primary and using the native target cap. Copies do not roll critical hits or reapply caster damage modifiers. Pinned default `bugs=1` uses a hard cap (`sc_mage.cpp5340–5362/5400–5403`).
- Selected Infused Splinters rolls its native effect2 chance on actual Frost Splinter impacts and adds its native effect4 Freezing count to the owned recipient debuff (`sc_mage.cpp5380–5381/1891–1892`).

- Selected Splintering Orbs schedules native effect6's count of Frost Splinter conjures, one every500ms after Orb activation. Selected Augury evaluates its native ICD at each successful Splinter execution, including failed PRD attempts, with the pinned21-attempt cap and native mean chance/count. A successful Augury conjures after100ms. Pinned null-target conjures explicitly use `mage.target` at callback execution; random selection is commented out in76f8118 (`sc_mage.cpp4181–4209/5419–5423/6638–6641/7346–7352`).

### Frost Mage native Freezing and Shatter

Frost player reaction clocks are configured before food setup advances native frames, so early passive Icicle acquisition keeps its actual reaction deadline.

- Actual Frostbolt228597, Flurry228354 and Glacial Spike228600 damage adds the native1246769 Freezing counts to the caster-owned1221389 debuff. Selected Frostbite adds its native count on critical Frostbolt/Spike impacts. Each actual Ray tick adds one stack (`sc_mage.cpp3947/4055/4094–4095/4248/4304–4305/4995/7105–7127`).
- Ice Lance228598 consumes up to native1246769 effect4 stacks from that mage's own recipient debuff and launches one native1246949 payload scaled by the consumed count. Remaining owned Freezing refreshes. Effect0 hits the primary once; selected Frostbite enables effect1 on secondary enemies only, with its native reduced-AoE cap (`sc_mage.cpp4324–4342/4428/7129–7181`). Ordinary Ice Lance snapshots native Fingers of Frost and Thermal Void at admission and retains them through its missile and228598 payload. Spend those buffs at execution. Earned Fingers pays the full native Shatter count without consuming recipient Freezing; earned Thermal doubles the count (`sc_mage.cpp4445–4473/7129–7170`).

- Native Frozen Orb grants selected Everlasting Frost's authored Fingers count and rolls native Wintertide for Brain Freeze; guaranteed Brain Freeze restores all native Flurry charges. Selected Permafrost Lances opens its native buff. An actual Brain Freeze Flurry consumes that proc and opens selected Thermal Void (`sc_mage.cpp4186–4190/4012–4015/7430–7433`). Actual Frostbolt and Glacial Spike executions roll their native Fingers/Brain Freeze sources in pinned order; Spike also rolls native Flash Freeze. A random Brain Freeze restores one Flurry charge and delays only an already-active buff refresh by150ms. Successful Brain Freeze immediately grants selected native Brainstorm and Shifting Shards conjures (`sc_mage.cpp4085–4086/4281–4283/7412–7437`).

- Actual paid Shatter rolls native Hand of Frost once using selected talent1 chance plus its native talent2 per-shattered-stack contribution. Native Hand impacts add their authored Freezing count. Selected native Hand buff defaults to one tenth of talent2 effect2 (`sc_mage.cpp7155–7159/5445/5449–5451/6458–6462`). Native MID2 two-piece rolls its effect2 chance independently for each shattered stack, earning actual205473 and enabling1222865 at the native cap (`7171–7200`).
- Selected Hand of Frost on Ray uses the native duration/period and talent3 count to calculate its tick interval. Each tick retains the actual scheduler ordinal; Hand executes on the pinned odd/interval schedule without reconstructing ticks from damage amounts or wall time (`sc_mage.cpp5005–5013`).

- Earned Brain Freeze Flurry snapshots native190446 effect2 on the real44614 execution and carries that effectiveness across the native costed228596 callback and its missile to228354. Spending or expiring the live buff cannot remove that launched benefit. Preserve native Effect64 costs, carrier speed, delay and payload coefficients (`sc_mage.cpp3957–3966/4002–4015`).
- Deep Shatter's native periodic Dummy recalculates its zero-valued critical-damage spell modifier from native effect1's factor and current caster spell critical chance, including initial application. Keep its native spell mask and operation; do not replace Shatter coefficients (`sc_mage.cpp4365–4373`).

- Actual paid Ice Lance primary stacks conjure selected native Force of Will Splinters using its effect3 stacks-per-conjure value. Selected native Glaciate effect2 milliseconds reduce running Ray recharge per shattered stack, never starting absent timers (`sc_mage.cpp4471–4481`).

- Each real Frozen Orb bolt resolution that actually hits at least one unit rolls selected Everlasting Frost effect2 for one Fingers stack. Use the native resolution ledger to run this once per area pulse, not once per recipient or only when a preselected primary hits (`sc_mage.cpp4141–4147`).

- Selected Overflowing Energy merges actual Frostbolt228597 and Glacial Spike228600 hit outcomes through the shared native buff-outcome queue: noncritical hits earn native394195 stacks and eligible critical hits expire them after the hit batch. Ice Lance neither produces nor consumes this state; native spell masks and stack amounts supply the Frost critical-chance benefit (`sc_mage.cpp4050/4244/1729–1730/1897–1898`).

### Frost Mage native Icicles and Glacial Spike

- Selected Icicles1246832 uses its actual hasted PERIODIC_DUMMY clock to apply native205473 without a spell cast. The first native interval has the pinned random phase (`sc_mage.cpp5559–5581`); later intervals keep native timing. Reaching the actual counter cap enables1222865.
- Only an actual native Glacial Spike execution rolls equipped MID2 four-piece Rapid Refreezing. Suppress the native broad damage proc hint for1296586 while retaining its normal damage modifier. Use pinned custom accumulated chance `0.2 + failures × 0.020116149192034555`, reset on success; Lance/Shatter and unrelated successes must not roll or grant it (`sc_mage.cpp4296–4297/6643–6646`).
- Earned Rapid Refreezing1310248 uses its actual native200ms PERIODIC_DUMMY clock to grant one owned205473 per tick and enables1222865 at the native cap. Unlike the passive Icicles clock, its first tick is not randomized (`sc_mage.cpp6468–6471`).
- A successful native Glacial Spike199786 dispatches native228600 to its preserved hostile primary, then consumes205473 and1222865 (`sc_mage.cpp4243–4245,4275–4279`). No reference damage or replacement coefficient is applied.

### Frost Mage native Orb

- Frozen Orb84714 schedules native84721 area damage through the shared ground-pulse scheduler. Its source-qualified activation minimum and interval are500ms (`sc_mage.cpp4181–4206`); native root duration and child radius/damage supply all remaining values. One pulse hits each in-radius enemy once, without multiplying the area cast by the number of recipients. Native15s duration pays30 pulses, including the last tick.

### Frost Mage literal fixture

- Spec64 captures the native player before setup and executes the unchanged exported76f8118 Frost action rows through shared `Program`/`Driver`/`Prepared`/`Equipment`, not AssistedCombat4512.
- Precombat completes real Intellect and its first usable harmful precast before engagement. Native clocks, channel decisions, knowledge, costs, cooldowns and70448 spell values remain authoritative. Buff and recipient Freezing reaction clocks are configured before acquisition, including new add waves.

### Beast Mastery literal fixture

- Selected Huntmaster Call459730 increments native459731 once per actual Dire Beast, Dark Hound, or Bear. At its native stack cap it clears the counter and selects Fenryr459733 or Hati459739 equally; the selected owner buff459735/459738 supplies the real companion lifetime. Each companion uses live owner ranged AP and its own attacks; Fenryr launches native Ravenous Leap459753 (SimC76f8118 hunter1869–1873,1973–1982,2008,2049–2052,3715–3734,3865–3874).

- Selected Dire Command378743 rolls its native chance using SimC accumulated PRD after actual BM Kill Command and summons native219199. Selected Ursine Fury472476 summons its native number of the same Dire Beasts when an earned Bear is summoned. Their native base lifetime is converted to the source's haste-scaled probabilistic swing count; Beast110063 inherits live owner ranged AP (SimC76f8118 hunter1736–1755,1935,3797–3803,6610–6613,8396).

- Selected Wildspeaker1232739 applies native1235388 to existing DireCritters when owner Bestial Wrath succeeds. New DireCritters joining an active owner BW inherit only its remaining time, before summon-time Rend executes; stable pets keep their separate native Wrath path (SimC76f8118 hunter1866–1867,6729–6736).
- Actual DireCritters inherit the owner's live critical chance for melee and spell attacks even when they have no autonomous guardian spell list (SimC76f8118 pet.cpp217–220). Their spells also inherit the owner's selected passive spell modifiers when native masks match, including Savagery on Wildspeaker1232922 (SimC76f8118 player.cpp15268–15324).
- Selected Xalan's Cruelty and Ferocity multiply owned Warlock pet spell critical chance for matching native labels, before target modifiers (`sc_warlock_pets.cpp`265–301).

- Selected Fury of the Wyvern472550 extends an existing earned Cry471881 after successful BM Kill Command34026, including the consuming command. Native effect2 supplies each extension and effect4 the total budget. Actual Cry removal resets the budget; white-impact stack growth does not (SimC76f8118 hunter6626–6630,8279–8287).

- Selected Pack Mentality472358 reduces already-running Barbed217200 and Wildfire259495 recovery once per actually consumed Howl beast. Native effects2/3 supply the times; simultaneous Wyvern/Boar consumption pays both reductions before Stampede begins. Unused clocks remain inactive (SimC76f8118 hunter3808–3811).
- Every real owned-pet83381 hit multiplies 1 + selected Alpha Predator269737 native effect2, on top of the owner's passive spell mods (which separately apply its effect4, Savagery and Specialized Arsenal). When any Howl beast is ready, selected Pack Mentality adds its native first effect to that same term before multiplying. The term does not change Wildspeaker1232922 (SimC76f8118 hunter2768–2789).
- Selected Dire Frenzy385810 multiplies actual DireCritter melee and spell damage by its native second effect. Real DireCritters also start native Pet Damage1284992, including the selected specialization's native modifier. Neither applies to other pets or summons (SimC76f8118 hunter1854–1869,1877–1882).

- Each actual main-pet, companion or stable Nature's Ally white impact adds one stack to that owner's already active Wyvern Cry471881, including avoided outcomes. Preserve the native cap and the existing expiration; white impacts cannot create inactive Cry. Keep landed-hit producers and ordinary proc masks unchanged (SimC76f8118 hunter2974–3010).

- Ordinary main-pet Bite, Claw and Smack use native62762 Wild Hunt damage and cost bonuses only above its authored Focus threshold. Capture damage effectiveness before payment; at the threshold or below, ordinary cost applies. Free background basics and non-main stable bodies do not receive this main-action behavior (SimC76f8118 hunter3046–3079).

- Selected Wildspeaker commands existing native Dire Beast, Dark Hound, Fenryr, Hati and Bear summons with native1232922 at the real Kill Command primary. Command dispatch precedes Howl consumption, so a Bear created by that cast cannot receive it retroactively. Stable bodies retain their separate command path (SimC76f8118 hunter3175–3189/6587–6604).
- Selected Go for the Throat459550 multiplies the critical damage bonus of actual owned-pet Kill Command83381 and DireCritter Wildspeaker1232922 by one plus its native second effect times the owner's live attack critical chance. It does not change hit rolls or unrelated actions (SimC76f8118 hunter2791–2799/3192–3199).

- A real Howl Bear immediately casts its native Rend Flesh471999 at the consuming Kill Command's hostile primary and nearby enemies. Native effect1 supplies the bleed radius and coefficient; effect2 supplies the target cap. Each Bear uses native471993's finite lifetime for its actual471990 summon;471990's own indefinite duration must not create permanent command bodies. The Bear owns the applications and periodic damage, using its source-defined live owner ranged-AP share0.6 without changing native bleed coefficients (SimC76f8118 hunter2043–2047/2057–2060/3218–3234).

- Native Aura493 triggers its authored companion spell after Call Pet; native Effect260 creates a second stable body without changing the current-pet record. The companion has white swings and owned-pet commands, no autonomous Focus spender, and ends with its main pet. NPC pet orders reach all active owned pets.

- Selected Howl starts its native cooldown at combat entry and earns Wyvern, Boar and Bear readiness in order. Bestial Wrath with selected Stampede independently advances that cycle. Ordinary BM34026 and Survival259489 Kill Command consume every earned ready beast: Wyvern and Bear require actual owned native summons; Wyvern Cry follows the real Wyvern. Consuming readiness starts the native cooldown only when absent. All earned readiness and Incoming Stampede are script-owned: unrelated successful casts and triggered children cannot spend them. Only actual Kill Command consumes earned beasts; consuming Howl spends Incoming Stampede. An armed Stampede starts only after a consumed Howl and retains the selected primary, native tick-zero/periodic clock and600/900/1200ms payload delays; native201594 damages the pack without a target cap (SimC76f8118 hunter3776–3841/4864–4900/6780–6781/8240–8270/8698–8699). Bear attacks and Wildspeaker remain separately unqualified.

- Selected Dire Summons472352 reduces only the owner's running Howl471877 clock after actual BM/SV Kill Command, Cobra Shot and source-qualified Survival melee Focus spenders, using native effects0/1/2/3 respectively. An exhausted clock expires through its real next-beast callback; an inactive clock stays inactive. Howl consumption precedes recovery (SimC76f8118 hunter4966/6127/6604–6624).
- Selected Hogstrider472639 earns native472640 stacks from real Boar damage. The next Cobra Shot snapshots its native damage bonus once, selects additional enemies using the earned stacks, and consumes the aura at launch (SimC76f8118 hunter4817–4846/4944–4980).

- Selected War Orders reduces already-running Kill Command cooldown/recharge after a successful Barbed Shot by native393933 effect2's time value. It does not start a cooldown or replace native charge recovery.

- Barbed Shot triggers selected native201754 Stomp from each active owned pet, followed by the native1296631 two-piece effectiveness echo. Native1296632 four-piece Stomp impacts earn the owner's actual1299389 Cobra Fang stacks. Without active Beast Cleave, Cobra Shot snapshots the native per-stack damage effect and consumes the aura on successful launch; flight retains that earned effectiveness. Only real ItemSet-owned auras admit these tier relationships.

- Main-pet Focus regeneration is continuous at1.25 times the owner's native potential Focus rate, including its native haste and modifiers. It is independent of owner resource cap and entity iteration order. Stable guardians do not regenerate Focus. The legacy24-Focus-per4s path is removed.

- Bestial Wrath applies native186254 to permanent owned pets after their instant strike. Nature's Ally receives native1285912 after450ms and native344572 against the selected primary after1.5s. These are queued native casts with normal caster/target lifetime checks, not synthetic damage.

- Selected Bloodshed applies native321538 from every active owned stable pet after Bestial Wrath, including a newly summoned Nature's Ally. Each pet retains its own periodic aura, native amount and lifetime.

- Native Effect188 summons a stored pet as a guardian using the authored count and lifetime without replacing the current stable pet. Selected Nature's Ally triggers native1282474 after Bestial Wrath; the guardian receives an explicit native attack order without changing its saved stance, performs white swings and receives owned-pet commands. Commanded spells alone do not prove autonomous melee. Expiry/death removes only the guardian.

- Stable hunter pets inherit60% of their owner's weapon-inclusive native ranged AP under pinned SimC behavior. Native spell coefficients remain unchanged. Controlled-pet direct spells inherit the live owner's native critical chance while retaining the pet as the damage source.

- Selected Nature's Ally1273126 arms native1276720 after successful Barbed Shot or Cobra Shot; successful Kill Command consumes it. Native after-cast effects must compose with these sourced policy rows, retaining Barbed Focus. Literal `.react` observes native acquisition with the pinned profile's reaction settings.

- Literal precombat provisions the existing native default-duck stable policy and executes ordinary Call Pet. The pet attacks the current encounter primary through native pet control, without restoring follower rotation decisions.

- Spec253 keeps the requested native actor identity and disables follower-kit decisions before setup. Its pinned76f8118 `hunter_beast_mastery.simc` rows run through the shared `Program`/`Driver`/`Prepared`/`Equipment` path, including native pet summon, snapshot, Auto Shot and target-scored Barbed Shot. Native70448 supplies spell values, readiness, resources and damage; the adapter does not substitute reference damage.
- Selected Thundering Hooves makes each permanent pet execute its own Stomp on Bestial Wrath using the talent's native effectiveness; Nature's Ally does not execute it. Its actual impacts can earn Cobra Fang, but do not trigger the Barbed Shot set echo (SimC76f8118 `sc_hunter.cpp3110–3125,3335–3339,6753–6761`).

### Outlaw literal fixture

- Selected native Unseen Blade441146 dispatches441144 after successful Sinister Strike/Ambush and selected Clever Combatant Pistol impacts. Preserve its native ICD and separate459485 quiet cooldown; selected Flashing Steel may emit its native extra strike. Actual Unseen executes earn Escalating Blade when Coup is selected; actual impacts apply owned Fazed and selected Flawless Form. Real Spree earns native Disorienting uses and Flawless before tick-zero. Selected permanent441274 is not an earned runtime use; uses bypass only the quiet cooldown, never the ICD (`sc_rogue.cpp4812–4821,5989,6689–6712,8821–8850,11102–11110`).

- Selected native Rogue Supercharger470347 fills its authored count of first inactive counters in pinned470398/470406/470409/470412/470414/470415/470416 order after ordinary Adrenaline Rush, Shadow Dance or a successful Kingsbane direct impact. Eligible finishers add its native effective-point bonus and selected Forced Induction after other effective-point sources, without changing payment. Consume one highest active counter after success and open selected Echoing Reprimand470671. Slice and Dice and unselected talents do not consume or grant counters (`sc_rogue.cpp2021–2038,2215–2216,3578,4959,5576,8913–8961`).

- Native Main Gauche76806 rolls its authored proc chance from successful main-hand white swings and the pinned eligible special impacts, including Sinister extra attacks and Killing Spree's main hand. Emit native86392 at the actual hit recipient. Pistol Shot, Between the Eyes, off-hand whites, Spree's off hand and Main Gauche itself cannot produce it. Native mastery coefficients remain unchanged (`sc_rogue.cpp3468–3469,4726–4728,5942–6001,7948–7973`).

- Pistol Shot spends one earned Opportunity use per execution, not all stacks or a stack-multiplied damage/cost bonus. An ordinary shot schedules up to native Fan the Hammer's count of remaining earned uses100ms apart, using its native damage reduction; children cannot fan. Each earned shot grants native Quick Draw points when selected. Background shots freeze charged modifiers at launch and consume their own use before impact (`sc_rogue.cpp4981–5070`).

- Landed native Sinister Strike193315 rolls selected Opportunity279876 once per hit. Read native chance and the RtB extra-chance contribution; success earns native195627 stacks and dispatches197834 to the same enemy after300ms. Extra strikes cannot recursively roll Opportunity (`sc_rogue.cpp1286–1292,5942–5991,8255–8280`).
- Blade Flurry13877 activation dispatches one native429951 area attack, including the primary enemy. Selected Deft Maneuvers grants its native combo-point amount once per actual activation hit. This is independent of Blade Flurry22482 secondary copies (`sc_rogue.cpp3907–3922,3993–3999`).

- Spec260 preserves the requested player identity and captures literal control before setup. Execute the profile's complete action rows with shared `Program`/`Driver`/`Prepared`/`Equipment`, not AssistedCombat. The pinned profile requests Instant Poison without a nonlethal coating. Read combo points, Energy, native readiness and earned Roll the Bones states from the actor; never substitute reference damage or synthetic buffs.
- Shared Rogue operands expose the native combo-point spend capacity and Energy deficit/time-to-max excluding temporary capacity, while retaining permanent talent capacity.
- Native Rogue-family spells with a ComboPoints cost scale all direct damage effects by frozen effective points. Between the Eyes must not be omitted by an action-ID list; Envenom and Dispatch retain their existing point scaling. Feral Bite keeps its separate maximum-point-normalized rule.
- Roll the Bones resolves the native specialization action1214909; the obsolete315508 talent endpoint must not suppress it as unknown. A roll replaces old secondary states with exactly one native One of a Kind/Double Trouble/Triple Threat/Jackpot aura at55/30/10/5 odds. Selected Sleight of Hand may raise states below Triple Threat by its native chance; Loaded Dice raises once afterward and is consumed. The new state inherits the native container deadline.
- The native four-piece source may produce Fang Strike only from Ambush or Sinister Strike. Sinister Strike's background extra attack197834 must not roll the tier proc (`sc_rogue.cpp3597,5944–5964,5991`). Read chance and payload from its native effect, rather than treating its broad101% proc hint as an unconditional damage trigger. Dispatch and white swings must not produce their own free-finisher window.
- A finisher affected by earned Fang Strike snapshots maximum native combo-point capacity for its resource-dependent values, even when actual payment is zero. Consume that Fang Strike use after the successful ordinary cast. Match affected actions through the native aura's family mask, not a finisher-ID branch.
- Selected Thousand Cuts rolls its native RPPM on eligible melee swings and expires Unseen Blade's earned quiet cooldown without bypassing its internal cooldown (`sc_rogue.cpp8821–8847,11539–11564`).
- Active Blade Flurry transfers eligible landed spell/white source damage to native-capped nearby secondary enemies, never the primary. Preserve the source critical bonus before target-specific modifiers and mitigation, apply native13877 transfer fraction and learned Dancing Steel/Nimble Flurry conditions, and resolve22482 against each secondary exactly once without recopying the payload. Each landed copy can trigger selected poison coatings on its own recipient using their native chance and cooldown (`sc_rogue.cpp3899–3904,7890–7913`). Apply the recipient's owned Fazed damage effect to22482 even though its native family mask does not include that spell (`sc_rogue.cpp2530–2533,3894`).
- Killing Spree retains its native resource-sized channel and aura lifetime from prepayment effective combo points, capped at the spend maximum. Excess effective points increase each hand's damage by native51690 effect4 instead of adding ticks; preserve that full prepayment value for the channel's lifetime (`sc_rogue.cpp4702–4714,4794–4805`). Its native tick-zero and periodic schedule independently dispatch both hand attacks to nearby hostile enemies; only scheduled ticks after tick-zero trigger native1235074 combo-point returns. Native haste and Adrenaline Rush period/duration modifiers remain authoritative (`sc_rogue.cpp4749–4838`).
- Learned Restless Blades reduces its pinned running spell/recharge timers after an eligible finisher using frozen effective combo points, native79096 tenths-of-seconds per point and the native high-roll-state bonus. It must not start absent cooldowns or require a hostile hit context for self-targeted Slice and Dice.
- Coup de Grace rolls selected Palmed Bullets and qualifies selected Scoundrel Strike using its bonus effective combo points (`sc_rogue.cpp6863–6891,8879–8909`).
- Learned Cut to the Chase extends or starts owned native Slice and Dice only after the pinned eligible finisher lands. Freeze effective combo points before payment; use native51667 seconds per point and315496 base duration, capped at the maximum-point pandemic duration. Dispatch, Envenom, Eviscerate and Coup de Grace are the pinned consumers.

### Subtlety literal fixture

- Deepening Shadows extends earned Shadow Dance duration using the caster's raw haste-rating fraction and native effect value (`sc_rogue.cpp7457–7475`).
- Selected Lingering Darkness457056 grants native457273 when Shadow Blades expires. Subtlety keeps its effect2/effect4 Shadow mods, label and Dark Brew effects; Assassination's poison effect1/effect3 never apply (`sc_rogue.cpp10924–10932,10982–11010`).
- Selected Shadow Focus108209 grants native112942 when Stealth, Vanish or Shadow Dance applies and removes it once none remain (`sc_rogue.cpp7345–7362,7381–7390`).
- Secret Technique's intrinsic delayed clones snapshot the caster's native guardian damage multiplier; the player attack does not (`sc_rogue.cpp5377–5385`).
- Eligible shadow clones and landed intrinsic Secret Technique clones can earn Shadow Techniques storage and Energy using Ancient Arts' native proc chance; Shadowcraft still controls stacks during Dance (`sc_rogue.cpp3650–3659,5405–5413,8159–8172`).
- Shuriken Storm and its shadow clone snapshot the native rank-two stealth-stance damage bonus (`sc_rogue.cpp5817–5838,5872–5881`).
- Eligible shadow clones can earn Shadow Techniques storage and Energy using Ancient Arts' native proc chance; Shadowcraft still controls stacks during Dance (`sc_rogue.cpp3650–3659,8159–8172`).
- Secret Technique's landed player attack triggers earned Ancient Arts collection and can launch its native delayed shadow clone, using the parent finisher's paid points (`sc_rogue.cpp5435–5503,8668–8701`).
- Eligible shadow clones can earn Shadow Techniques storage and Energy using Ancient Arts' native proc chance; Shadowcraft still controls stacks during Dance (`sc_rogue.cpp3650–3659,8159–8172`).
- Secret Technique's landed player attack triggers earned Ancient Arts collection and can launch its native delayed shadow clone, using the parent finisher's paid points. With pinned SimC `bugs=true`, that shadow clone deals full damage to secondary targets (`sc_rogue.cpp5333–5352,5435–5503,8668–8701`).
- Maximum-point Eviscerate's Shadowed Finishers child and shadow clone snapshot Darkest Night's native damage and critical bonuses before the shared Deathstalker buff expires 1 ms after consumption (`sc_rogue.cpp2435–2440,2569–2578,4394–4414,8740–8746,10583–10584`).
- Consuming stored Shadow Techniques can launch a delayed builder shadow clone; Ancient Arts can also launch an Eviscerate or Black Powder shadow clone with the source finisher's combo points and critical chance snapshotted (`sc_rogue.cpp3627–3665,8175–8194,8229–8249,8668–8701`).
- Consuming stored Shadow Techniques through Ancient Arts can launch a delayed Eviscerate or Black Powder shadow clone, with the source finisher's combo points and critical chance snapshotted (`sc_rogue.cpp3627–3665,8229–8249,8668–8701`).
- A landed combo-point damage hit, including its Shadowed Finishers and clone payloads, distributes Goremaw's native residual fraction across targets with the caster's active Goremaw DoT (`sc_rogue.cpp2840–2845,6444–6463,9011–9028`).
- Combo-point gains emit one native Energize receipt with actual overflow, including scripted per-hit gains. Shadow Techniques recognizes actual foreground combo-point generation, including capped gains, and collects after all native hit batches for that cast, not before delayed impacts (`sc_rogue.cpp2705–2716,8176–8204`).
- In-combat Shadow Dance, Vanish and Stealth earn native Master of Shadows196980 when196976 is selected. Refresh preserves the periodic phase; native periodic and first-application Energy values remain authoritative (`sc_rogue.cpp5572–5575,6114–6117,6146–6149,8532–8537,11196–11204`).
- Selected Deathstalker Shadowstrike establishes the native mark; eligible finishers consume it and Black Powder launches native Mass Casualty against secondary targets (`sc_rogue.cpp5653,6570–6594,8732–8817`).
- Selected native Lingering Shadow earns its native maximum counter on Shadow Dance expiry, with the pinned tick-zero decrement and rolling periodic decay from native talent effect2. Replacing the counter restarts its lifetime; it must expire rather than retain a final stack (`sc_rogue.cpp7489–7501,11182–11195`).
- Eligible landed builders and actual MID2 four-piece finishers emit native386081 at the same recipient from the mitigated source result, preserving source crit once. Use native counter stacks, set fraction and selected Shadow-school/Lingering Darkness modifiers; no residual without earned stacks or recursive residual emission (`sc_rogue.cpp2083–2089,3701,4400–4425,5709–5851,8554–8575,10568–10580`). Native regression: `subtlety_shadow_tests.rs`; numerical parity remains separate.

- Spec261 preserves the requested native actor and executes every pinned MID2 Subtlety action row through shared `Program`/`Driver`/`Prepared`/`Equipment`. Precombat uses shared Rogue native coatings and Stealth before engagement. Shared Rogue facts retain native Energy and combo-point values; Subtlety bindings read earned native buffs. Missing damage relationships remain measurement gaps, not substituted reference damage.

### Equipment

- Hunter's Ritual Stone discovers its native equipped1297382→1295897 linked proc. The source-qualified four-child selection relationship is explicit1295898/1295899/1295900/1295901 because native DUMMY has Trigger0 (`unique_gear_midnight.cpp1385–1428`). Choose uniformly and retain native callback deduplication; take rating amount from the equipped root's scaled effect2, not the linked dummy BP0. Children retain the equipped root's item-level snapshot; masks, duration and native proc timing remain authoritative. No stat grant while actually out of combat or fabricated DBC edges; real hostile pre-pull casts can earn combat procs. Native RED→GREEN gates every equipping profile before landing.

- Player white swings use SimC76f8118's retail miss table: no legacy5% base miss, with a19% dual-wield penalty. Subtract attacker and victim hit modifiers after combining the base and dual-wield chances; retain the existing NPC miss table (`attack.cpp`, `attack_t::miss_chance`).

- Shared named/slot on-use item selection binds native enemy/any-unit spells to the selected primary by default; ally/self payloads retain the actor. Preserve explicit target contexts, native admission, item level and cooldowns. An absent hostile primary is unavailable, never a self-target substitute (SimC76f8118 player.cpp10056–10068).

- Font of Venomous Rage1297908's actual item-use channel snapshots native1297911 effect0 divided by its native base tick count (including the authored initial tick). Each channel damage tick launches native1307222 at the retained primary, with native1297911 effect1 per tick; its area excludes that primary and follows pinned reduced-AoE8 semantics. Native item level, role expression, channel timing and damage pipeline remain authoritative (SimC76f8118 unique_gear_midnight.cpp3803–3859).

- `trinket.N.has_stat.<stat>` and `has_buff.<stat>` use the same native item stat-buff existence query, including `any_dps`; direct-damage-only items are false.
- `trinket.N.has_use_damage` requires an actual native on-use ItemEffect whose spell/reachable native trigger effects deal direct, weapon or periodic damage/leech. Passive-only or stat-only trinkets are false; no item-ID lists or guessed script damage.
- `trinket.N.cooldown.charges_fractional` reads native charge-category state or ordinary spell/category timers; fractional progress uses authored/resolved recharge duration, never a synthetic cooldown. Aggregate multiple item cooldown sources with pinned max semantics; no cooldown sources returns0.

- Generic `set_bonus.<tier>_<n>pc` accepts both source tier spellings (`set_opt_name` and `Tier`) projected from pinned SimC item-set data, e.g. `midnight_season_2`/`mid2`. Native membership, threshold and spec remain authoritative; unknown aliases error. No hand-authored name-to-set substitutions.

- Shared literal `trinket.N.ilvl` reads the original equipped native item's resolved instance level, including passive items, bonus lists and explicit overrides; it is not average character ilvl or a stat-donor identity.
- `trinket.N.proc.any_dps.default_value` follows pinned SimC buff.default_value expression semantics, never stat magnitude. Only source-qualified native drivers may bind a numeric default; unqualified scripts return `NATIVE_ITEM_PROC_DEFAULT_VALUE_UNSOURCED`. An empty effect set evaluates to0. The source-qualified unset defaults in Dev/Fire/Assassination gear aggregate to0 by pinned max-with-zero semantics.

- [ ] Instances retain bonus lists, an optional explicit item level, permanent enchant and gem item IDs by GUID, with backward-compatible redb storage. Crafted stat selections survive where the profile specifies them. A pre-existing database without the additive modifier table loads as unmodified gear; unrelated storage errors and malformed modifier bytes still fail explicitly.
- [ ] Equipment stats resolve native70448 ItemBonus, ItemSparse, RandPropPoints and Midnight scaling/configuration tables, including stat allocation, sockets and enchant/gem effects. Physical-resistance enchants (type4/school0) contribute scaled, truncated, minimum-one armor, preserving their independent primary effects (TC a352b1fa Player.cpp13509–13531). Other resistance schools remain explicit gaps. Unsupported effects are named gaps, never invented stats.
- [x] The native-table producer preserves signed values, replaces only the requested table, supports repeated imports and rolls back a duplicate-ID failure without losing the prior table.
- [x] Shipped profile manifests preserve exact item IDs, bonus lists, item-level overrides, enchants, gems, crafted stats, talents, consumables and action text. Missing item IDs fail explicitly. Temporary enchant declarations apply only to equipped slots; an empty off hand is inactive.
- Native class actors use live `CharacterStats` melee crit even with NPC identity. Native Hunter pets inherit their current owner's crit; preserve their own applicable bonuses and target modifiers. Generic no-stats creatures retain native template/base crit. Do not replace class/owner crit with generic creature5% or suppress it through a generic NPC `NO_CRIT` flag (SimC76f8118 `player.cpp5125–5134`, `pet.cpp217–220`).
- Equipment-source aura procs require actual, non-suppressed native combat references before chance, RPPM or recovery state changes. Equipment initialization and idle aura reconciliation cannot earn temporary item buffs. Ordinary class-aura eligibility remains unchanged (SimC76f8118 `action/action_callback.cpp79–82`).
- RPPM clocks start without preloaded time; each eligible attempt adds at most ten seconds to bad-luck protection, and a successful proc resets it (SimC76f8118 `proc_rng.cpp95–115`).
- Player-level ScalingClass−7 spell effects multiply the native scaling budget by the trinket combat-rating multiplier at the effective scaling level before rounding (SimC76f8118 `spell_data.cpp315–345`); Blooming Feast grants65 rating at level90.
- Item-scaled ScalingClass−7 effects retain the actual source item's inventory type and use its armor, weapon, trinket or jewelry combat-rating column; an invalid inventory type has no multiplier (SimC76f8118 `spell_data.cpp358–399`, `sc_item_data.cpp1824–1857`).
- Venomcursed armor drivers1307906/1307923/1307928 proc their native crit/mastery/haste buffs. Both signed stat amounts come from the equipped driver's native effects1/2; retain each buff's native rating masks, lifetime and refresh behavior, and the driver's ordinary proc eligibility/RPPM. No invented stats or rate changes (SimC76f8118 `unique_gear_midnight.cpp5600–5614/6600`).
- Jan'thrazet, the Soul Fang driver1298085 procs native Soul Fang Alacrity1305360 under its own proc eligibility/RPPM. The haste rating is the equipped driver's item-scaled effect2; the driver's dummy effects do nothing else (SimC76f8118 `unique_gear_midnight.cpp5207–5217`).
- Arcanoweave Lining1283697's linked native RPPM aura1229511 procs Arcanoweave Insight1229746. All equipped copies share one callback. The buff value sums, over equipped drivers, native effect0 times effect1 percent at each driver's item level; effect1 is a non-aura dummy, so read it from the catalog. A missing native value logs an error and grants nothing. The buff keeps its native stat type, duration and refresh; the driver keeps ordinary proc eligibility and RPPM (SimC76f8118 `unique_gear_midnight.cpp677–730`).
- Powerful Eversong Diamond1258209 sets native effects1/2 (crit damage, crit healing) to its effect0 base value times the number of unique equipped Peridot/Garnet/Lapis/Amethyst gem descriptions (14262/14276/14277/14278), read from native gem rows when the equipped aura applies. Missing gem data logs an error and leaves the native zero (SimC76f8118 `unique_gear_midnight.cpp45–55,469–477`).
- Gebbo's Bottomless Bag1292291 keeps its native RPPM and proc recovery; each proc applies one of six artifacts chosen uniformly. Ratings come from the item-scaled driver effects (native index = SimC effectN − 1), and only rating effects apply. Salmon1308013 gives +effect5 to all four secondaries. Voidfin1308014 gives −effect6 to all four. Gralstone1308012 gives +effect3 haste. Scroll1306870 gives effect1/12 mastery per stack; a trigger sets 12 stacks and each native 1s tick removes one. Seashell1292299 gives effect0 crit per stack; a trigger adds one stack and each 1s tick adds one, up to 12; its self-damage is ignored. Totem1292300 gives effect7 versatility per stack; a trigger sets round(effect2/effect7) stacks, and each native totem proc removes one (SimC76f8118 `unique_gear_midnight.cpp3657–3743`).
- Loa Worshiper's Band1251904 rolls one eligible branch from native equipped gem descriptions: Capybara1252524 is always eligible; Peridot enables1252818. Capybara uses the equipped driver's second effect amount; Akilzon uses its own native item-scaled effect. Preserve unique-color weights, native rating masks, durations and item level. Other Loa gem branches remain unqualified.
- Rite of Hash'ey1297654 dispatches native DUMMY-trigger rating children using the scaled BP0 already supplied by its linked equipped rank. The pinned default favors the current highest secondary60%, otherwise uniformly selects a native child; retain source single-callback dedup, native masks, values and lifetimes. Both item families reject precombat before proc chance/recovery changes, including the Cast-source linked Rite driver (`unique_gear_midnight.cpp614–671/995–1122`, `player.hpp827`, `action_callback.cpp79–82`).
- Ordinary native haste auras multiply with converted rating haste and each other: `(1 + rating%) × Π(1 + aura%) − 1`. Retain native aura amounts/stacks, non-stacking selection, independent Crusade and autoattack-only342 factors. Native WW1205 rating plus5% ordinary haste yields33.7557%; no coefficient or rating adjustment (SimC76f8118 `player.cpp4982–4995`).
- `liquid_luster_2` uses native item271887/ilvl295. Apply one real Lustrous Gleam stack immediately and another at each native six-second period, capped by native stack metadata; derive each stack's rating from the scaled driver effect. All stacks expire with the driver. Literal `pre_pot_time` ages the real aura, pays elapsed native periods, ages its actual cooldown and preserves the later combat use (SimC76f8118 `unique_gear_midnight.cpp288–366`, `consumable.cpp500–565`).
- Temporary oil `if=` expressions read native selected talents; a false condition is modeled but inactive. `disabled` and empty consumable values produce neither effects nor `CONSUMABLE_UNMODELED` gaps.
- `void_touched_augment_rune` and `void_touched` apply the same native primary-stat rune. `flask_of_thalassian_resistance_2` grants native versatility; `lights_potential_2` configures its actual primary-stat potion use. Quality2 flask/potion identities retain native item level295 and native spell scaling, without specialization branches.
- `flask_of_the_blood_knights_2` applies native haste. `flask_of_the_shattered_sun_2` applies native crit from item241326 (quality2, native ilvl295); its applied aura and derived rating retain native item scaling. `harandar_celebration` and `silvermoon_parade` share the native primary-stat food buff. Flask quality uses native item level; food uses native food scaling. `blooming_feast` grants only the highest native secondary rating, selected after gear/passive initialization and before configured pet casts. Do not import reference stat amounts.
- [x] Native fixture equipment honors `redirected_base_stats`: use the donor item's native ItemSparse stat types/allocations with the original profile's item level and bonuses. Keep original equipped identity, tier/effects, armor and weapon inputs; never mutate catalogs or import reference stat values. Unknown/malformed donors fail explicitly; pinned explicit0 means no redirect. Non-redirected slots remain unchanged. Pinned76f8118 `item.cpp35–57` supplies option semantics; native70448 supplies amounts.
- [x] Explicit `embellishment` names resolve through the pinned SimC 76f8118 embellishment table to native item bonuses and equipped effects. Unknown names remain reported as unmodeled; `none` adds no effect. Crafted stat allocations continue to use native item-instance rules.
- [x] Reference selection uses MID2 for the eight available DPS/tank profiles and prior-tier MID1 for Evoker, labelled `SIMC_DEVASTATION_MID2_PROFILE_ABSENT`; it never silently substitutes another tier.
- [ ] Native base-item, bonus-added and enchant equip spells use the production aura path; unequipping an item or removing a modifier removes only its owned equip aura and linked child. Item-level changes replace stale aura amounts. Native LINKED/LINKED_2 zero amounts preserve child BP0; nonzero parent amounts override child BP0, with matching removal (TrinityCore a352b1fa `SpellAuraEffects.cpp5299–5335`).
- [x] Native equipped-set bonuses activate through the same production equipment aura path as ordinary items/enchants: count original Retail identities by native ItemSparse.ItemSet, enforce ItemSetSpell threshold/spec, skip TC legacy-inactive sets, and reconcile on equip/unequip or active-spec changes. Set-owned auras persist while eligible and are removed below threshold without removing unrelated equipment auras. No spell-ID cases or fabricated bonuses. The initial603 slice left skill/subtree selectors as named gaps; consolidation retains rt1086's already sourced native profession-rank and selected-subtree behavior rather than downgrading it. The complete native conditional-set inventory and proof boundaries live in the character-stats wiki.
- [ ] Instance-sensitive stack merging must not weaken catalog-based unique-equipped restrictions; different enchants do not permit a duplicate unique item.
- [ ] Selected-talent metadata preserves SimC's allocated node IDs, entry IDs and ranks, including each entry of a tiered node. `active_ranks` separately records official `find_talent_spell(entry).rank()`: pinned player.cpp11357 keeps allocated hero traits but disables them when their control tree is inactive. Native application uses positive effective ranks without deleting inactive allocation metadata. Explicit parity actors use `SIMC_PARITY_ALLOCATIONS` below; ordinary actors retain native DB2 spec constraints. Missing effective-rank metadata fails explicitly, never defaults to all-active or a default build. The pinned debug log consumes tiered ranks before printing, so metadata comes from actual `player_traits` via the separate diagnostic CLI wrapper, not that log.

### Shared cast target context

- [x] Native trigger effects whose resolved recipient is self must forward the immutable selected primary when their child requires an explicit enemy. Preserve queued effect64 delays/costs and immediate effect32 dispatch; self/friendly children retain native recipients. Native201427 must damage the selected enemy through both227518 and delayed201428 even after the live combat target changes.

- Shared literal readiness checks each selected native enemy primary, including attackable neutral units, against native minimum/maximum range and live Range modifiers for harmful actions, even when the native cast unit is the caster. Default and target-scored contexts retain their own range result; native cast units, resource limits, cooldowns and priorities are unchanged (SimC76f8118 action.cpp2418–2430).
- [x] Native execution preserves the selected action primary separately from the spell's cast unit. A self-target spell must keep its native self cast while forwarding the actual selected hostile primary to child relationships. Preserve that context across queued requests, cast bars, missile snapshots and periodic aura application/refresh/tick snapshots; an absent or unknown context must not select a substitute. Resolve caster-only targeting from the actual native endpoint after action replacement, not only the requested base action; ground destinations retain their declared targeting. Apply the same contract to Player and NPC actors without spell-ID branches, damage multipliers or mutable-target fallbacks. [Proof](../handoffs/hostile-primary-proof-ledger.md).

### Frost literal rotation

- Successful native Frost Strike/Glacial Advance executions roll Rime using native59057 effect2. Actual Frost Strike/Glacial Advance/Howling Blast during Pillar with selected Obliteration grants Killing Machine and rolls native281238 effect2 for native281327 rune restoration. White critical swings use pinned accumulated KM PRD (0.3 dual wield;1.0 two-handed with Might of the Frozen Wastes); ordinary/noncritical hits do not roll. Native passive trigger effects supply the buff identities and payloads (`sc_death_knight.cpp`7716–7719,10889–10900,11177–11189,11614–11622,14824–14825,16163–16165).

- Successful native Pillar with selected Frozen Dominion377226 starts native1233152, whose real periodic clock triggers196771 area damage (`sc_death_knight.cpp`12050–12060,15341–15342). Do not synthesize periodic damage in the literal driver.

- Native Frost Runic Power payment rolls Runic Empowerment from the larger native base/actual cost. Native81229 effect1 supplies the per-cost chance and193486 restores the rune; raw RP is converted from tenths only for the source chance calculation (`sc_death_knight.cpp`13128–13146,13424–13439). The literal adapter grants no runes.

- Ordinary Obliterate and Frostscythe retain earned Killing Machine through native weapon impacts, then consume all current stacks with selected Killing Streak, otherwise one. Each consumption grants native Killing Streak, extends active Breath by native effect3 per stack, advances selected Bonegrinder, rolls native Murderous Efficiency rune restoration, and schedules selected native Arctic Assault after500ms (`sc_death_knight.cpp`10585–10588,11908–11915,13372–13412,13460–13485). No child damage coefficient is replaced.

- An ordinary Howling Blast using earned Rime spends that stack, retaining its native launch modifiers. With selected Frostbound Will, only that earned-Rime cast adjusts running ERW recharge by native1238680 effect1's signed time value (`sc_death_knight.cpp`11637–11665). Do not turn one proc into fifteen seconds of free casts.

- Frost `.react` observes actual native buff/debuff acquisition using the pinned251 player's reaction settings, including new add waves. Predictable Obliteration KM is immediately reacted; white-crit KM retains reaction delay (`sc_death_knight.cpp`13356–13364). No proc is fabricated by reading a fact.

- [ ] Named Frost Omnium tokens, including Lynxlike Reflexes and Burning Haste, and their numeric TraitNodeEntry forms apply the same native auras; do not import reference stat amounts.
- [ ] Spec251 player measurements execute the pinned76f8118 Frost source-order precombat and combat action rows through the shared literal driver, not the follower or assisted priority. Native70448 supplies readiness, resources, spell values and damage; missing mechanics remain separate parity work.
- [ ] `death_knight.first_ams_cast` honors the profile delay and pinned20s default without inventing incoming damage or Runic Power.

### Fire literal fixture

- Actual Heating Up→Hot Streak conversion grants selected native Brainstorm461531 (`sc_mage.cpp2154–2157,6509–6512`). Heating Up alone and wasted critical hits with Hot Streak already active do not grant it; native data owns its Intellect amount, stack cap and duration.

- Fiery Rush's native permanent payload ends when its owner's Combustion ends (`sc_mage.cpp1453–1457`); its recovery-rate benefit must not persist between Combustion windows.

- An owned Controlled Destruction target stack multiplies only new Ignite contributions with its native per-stack amount (`sc_mage.cpp1945–1946`). Never rescale the unpaid existing bank or read another caster's target stacks.

- Selected Deep Impact amplifies only the closest live hostile recipient to the Meteor destination, with the native talent amount (`sc_mage.cpp4664–4668`); secondary recipients retain their unmodified native coefficient.

- Each admitted Meteor or earned Meteorite schedules one native impact action selecting its area once, not one complete area impact per missile recipient (`sc_mage.cpp4721,4741–4744,4802–4808`). The simulation installs the production area-field catalog, so native Meteor Burn entry/expiry and periodic damage use actual authored data.

- Native Inflame multiplies actual Hot Streak Ignite contributions by its selected talent amount (`sc_mage.cpp2397–2399`). Native Pyrocosm Fireball hits roll effect2 for one real Meteorite (`3845–3846`); each Meteorite execution reduces active Fire Blast recharge by native effect4 milliseconds once, regardless of target count (`4783–4792`).

- Selected Overflowing Energy gains a native stack after noncritical Fireball/Frostfire Bolt and clears on their critical hits (`sc_mage.cpp1897–1898,3819`). Selected Fevered Incantation gains after direct Mage critical hits and clears on direct noncritical hits (`1900–1901`); Intensifying Flame419800 inherits plain `spell_t`, so its native Mage family does not make its copied damage a feedback producer (`2502`). Apply these outcomes after the current hit batch (`5585–5605,7272–7293`). Fireball and Hot Streak-spell hits produce their caster-owned native Controlled Destruction target stack with native talent effect1×0.1 (`2457,3843,5648–5650`).

- Fire Blast spreads the caster's pre-existing Ignite before its new contribution (`sc_mage.cpp2216–2279,4586–4590`). Use native mastery effect2 as the weaker-bank threshold and effect5 as the secondary cap after selected spell modifiers. Sort eligible hostile banks ascending, exclude the source, copy its tick amount rather than add a deposit, start a fresh clock on empty targets and clone source progress on existing targets (`dot.cpp241–332`).

- Selected Ignition multiplies each new Flamestrike Ignite contribution by its native effect2 percentage (`sc_mage.cpp3896–3898`). Do not apply this to other Ignite producers or rescale the unpaid existing bank.

- Selected Molten Fury amplifies actual recipient damage at or below its native effect1 health threshold by native effect2 (`sc_mage.cpp6821–6827`). This is the Mage's target-versus policy, not an unconditional damage bonus or a new globally supported aura303 implementation.

- Selected Scald amplifies Fireball at or below its native effect3 health threshold by native effect1, and Scorch at or below Scorch's native effect2 threshold by Scald effect2 (`sc_mage.cpp2293–2304,3866–3868,5066–5069`). An admitted Heat Shimmer also enables Scorch execute after its buff is spent. Never apply these execute amounts to every target unconditionally.

- Selected Heat Shimmer attempts the native talent RPPM at actual Ignite tick boundaries, except while Scorch is being cast (`sc_mage.cpp2557–2559,6439–6441`). Do not substitute a fixed chance or grant the buff from direct hits. Its earned buff makes ordinary Scorch instant, enables execute crit at any recipient health and is spent once after launch (`5048–5080`). Preserve the admitted native damage/cast-time values and guaranteed crit across impact.

- Selected From the Ashes restores active Fire Blast recharge progress after actual eligible Fireball/Frostfire Bolt, Fire Blast, Pyroblast, Scorch and Dragon's Breath hits. Use the native talent effect1 time adjustment and native internal cooldown, including the Fire Blast charge just spent; same-frame hits cannot bypass that cooldown (`sc_mage.cpp2116–2120,3689,3821,4557,4905,5040`).

- Ordinary noncritical Hot Streak producers preserve Heating Up during its first200ms (`sc_mage.cpp2196–2210`). Qualify this with an ordinary Fireball already in flight when ordinary Fire Blast earns Heating Up; triggered depth1 damage is not a Hot Streak producer.

- A selected Spellfire Sphere success also produces native Glorious Incandescence without another roll (`sc_mage.cpp7316–7319`). Only Fire Blast may consume its native charge; Pyroblast and other harmful spells preserve it. An actual Fire Blast hit with that aura launches the native Glorious count of Meteorite missiles, staggered75ms; native449560 supplies travel and449569 supplies impact (`2003–2012,4577–4580,4802–4808`). Queued destination casts retain their selected target and destination.

- The shipped Fire fixture applies the pinned default five initial Sphere applications through native casting, retaining the native three-stack cap (`sc_mage.cpp366,6893–6894`). Selected Codex spends stored spheres on Phoenix invocation before its first immediate action (`1249–1256`).

- Selected Invocation summons authentic native Phoenix223453 through448659 for the actual Combustion lifetime, including Fired Up extensions (`sc_mage.cpp3531–3543`). It inherits live owner spell power, critical chance and haste; chooses the pinned single-target/area action pools at the native effect2 period; and spends stored Codex spheres on alternating exceptional actions (`1097–1400`). Native Ashes effects supply Cascade stacks for actual actions, including the exceptional Meteorite repeats. Import reachability must include the Invocation→summon relationship without creating templates or world spawns.

- Admitted Hot Streak spends produce selected Fired Up and Spellfire Spheres before damage, then selected Mana Cascade after consumption (SimC76f8118 `sc_mage.cpp2406–2430`). Sphere generation uses the pinned eleven-attempt capped PRD with native effect2 chance (`6633–6636,7310–7320`); Fired Up uses the pinned Combustion curve and otherwise native chance, recharge reduction and duration extension (`7200–7224`). Memory of Al'ar produces native Hyperthermia when the Combustion/Phoenix lifetime ends; its duration comes from the selected native talent (`1268–1275`).

- Meteor's admitted delayed continuation survives a later foreground hardcast (SimC76f8118 `sc_mage.cpp4721,4741–4744`). Flamestrike processes exactly one independent Fuel the Fire Hot Streak outcome per successful cast, not per recipient (`3913–3920`). Its direct damage also applies native Fuel effect2 per admitted hostile target, capped by native effect3 (`3904–3909`); count the cast's captured target plan, not the encounter's global enemy count. Pyroclasm retains the native talent chance (`6446`); MID2 four-piece modifies label5947 on its buff payload, not label5949 on the chance talent. Mana Cascade haste uses native effect2×0.001 per stack, not zero-valued effect1 (`6497–6501`).

Spec63 measurements use the exact profile action rows with shared `Program`, `Driver`, `Prepared` and `Equipment`, for either native actor kind. Capture fixture control before setup frames; complete native Arcane Intellect, Mirror Image and prepull Pyroblast in source order before starting the combat clock. Fire-only facts read selected-recipient health thresholds and Heat Shimmer from native70448 data; `hot_streak_spells_in_flight` counts this caster's actual eligible missiles, using the same producer identities as native hit handling. Ordinary dungeon follower control is unchanged.

- Flamestrike binds the selected native talent:2120 first, otherwise1254851, consistently for facts and dispatch.
- Meteor with selected Sunfury Execution and Pyroclasm applies one native269651 stack without the ordinary Hot Streak expenditure chance roll.
- An actual Pyroclasm hardcast with the earned native MID2 two-piece adds the source100% crit bonus. Preserve the admitted modifier through proc consumption and missile impact; instant Hot Streak casts do not qualify.
- Guaranteed critical hits predict their actual Heating Up acquisition; an observed Heating Up conversion predicts Hot Streak. A critical hit with Hot Streak already active preserves and predicts it without creating Heating Up.

### Havoc literal rotation

- Havoc Immolation Aura258920 grants its native Havoc initial Fury only; its periodic child258922 grants native Havoc tick Fury only while Burning Hatred320374 is selected. Do not apply the Vengeance energize effects to Havoc (SimC76f8118 `sc_demon_hunter.cpp`4651–4677,4784–4800).

- Immolation Aura258920 deals native initial damage258921 on each in-combat application or refresh. The precombat fixture does not emit its initial hit. Selected Undying Embers1272405 rolls native effect0 on natural expiry or replacement of an active aura and, after 300ms, reapplies the aura without a cast or resource grant; replacement resets its tick phase (SimC76f8118 `sc_demon_hunter.cpp`8927–8954,8980–9040).
- Completed Eye Beam/Gaze with selected Final Breath1266500 emits one additional native child tick scaled by the talent's effect0; interruptions do not emit it (SimC76f8118 `sc_demon_hunter.cpp`3285–3327).

- Selected Isolated Prey388113 increases native Eye Beam and Gaze child damage by its effect1 percentage when exactly one enemy is resolved; it guarantees Immolation Aura initial and periodic child critical hits under the same condition (SimC76f8118 `sc_demon_hunter.cpp`3939–3946,4651–4670,4705–4717,4799–4810).
- Landed direct Demon Hunter damage applies native Chaos Brand1490 to each affected enemy after the triggering impact; its native aura increases magic damage taken (SimC76f8118 `sc_demon_hunter.cpp`2452–2457,2594–2605).

- Never Say Die427794's native periodic driver enables its effect0 damage percentage on effects3/4 when health reaches effect2's threshold. Preserve the pinned source's below-threshold behavior: an already-active damage bonus is not expired by that branch (SimC76f8118 `sc_demon_hunter.cpp`2197–2199,10044–10066).

- Earned Serrated Glaive390155 supplies selected talent390154's effect0 percentage to its native spell modifier; the buff's class mask and lifetime still determine which real actions benefit (SimC76f8118 `sc_demon_hunter.cpp`2195–2196,7534–7539).

- Know Your Enemy388118's critical-damage modifier uses half its native ranked percentage of the owner's critical chance (SimC76f8118 `sc_demon_hunter.cpp`11998–12006).

- Selected Chaotic Disposition428492 rolls its native number of independent chances on each affected Demon Hunter chromatic-school direct or periodic damage action; each success adds its native damage percentage. The Hunt and its DoT are excluded, and Felblade damage is included (SimC76f8118 `sc_demon_hunter.cpp`2105–2108,2317–2346,2370–2396,5493–5497,7854–7858).

- Eternal Hunt1270901 applies its native effect0 modifier once to affected Blade Dance and Death Sweep strikes. Do not copy SimC's extra final-strike application; see [577](577.md#simc-double-applications).

- A melee fixture returns to its selected target at native run speed after self-displacement, including instant teleports. Continue polling its APL: skip actions outside their native target range so eligible lower-priority self-casts, off-GCD and in-range actions can execute (SimC76f8118 action.cpp2418–2430). Arcane Torrent202719 is a non-harmful self-cast for APL readiness even though native data has an area dispel (player.cpp9045–9057). A hostile channel waits for active displacement to end so its target-range check uses the settled position. Demon Hunter channels also remain unavailable while returning outside melee range (SimC76f8118 sc_demon_hunter.cpp2477–2486). Targets stay fixed and actual casts retain native range, movement control and channel self-slow. Real Retreat followed by Eye Beam must land all twelve native ticks and the selected Final Breath child.

- Havoc Retreat uses native 70448 movement and the actual return controller. SimC's hardcoded leap and direction-factor geometry produce a shorter return; the measured terms and limits of the damage estimate are recorded in [577](577.md#retreat-comparison). This difference does not change the requested raw ±5% parity target.

- Selected Chaos Theory389687 grants native390195 on Blade Dance/Death Sweep. Each affected damage action samples an additive crit-chance bonus between native talent effect0 and effect1; the buff's native proc modifier applies to Chaos Strike/Annihilation's off-hand refund roll. That off-hand execution consumes the buff whether or not a refund occurs (SimC76f8118 sc_demon_hunter.cpp2098–2103,2413–2420,7234–7237,7481–7515).

- Demon Hunter soul fragments retain their type, position, activation and expiry. The literal `pick_up_fragment` action selects a live fragment by its source mode and type, rechecks its condition at arrival, then consumes it through the spec's native payload. Demonic Appetite rolls native206478 on real Havoc Chaos Strike/Annihilation execution (SimC76f8118 sc_demon_hunter.cpp1656–1908,5069–5325,7579–7608).

- A landed Havoc Hunt impact applies native370969's periodic payload to enemies within its native hostile range, with the primary included in the native370967 effect0 target cap (SimC76f8118 sc_demon_hunter.cpp5488–5495,5505–5511). SimC's bugs-mode Soulscar double-applies Demon Hide (sc_demon_hunter.cpp8486–8501); do not copy that double application or apply it to Hunt.

- Selected Volatile Instinct1272453 triggers native Demonsurge on every hard Metamorphosis and on Demonic only when it enters, rather than extends, Metamorphosis. Hard Meta uses native1238696's delay; Demonic uses native453323's delay (SimC76f8118 sc_demon_hunter.cpp5008–5012,9160–9166,12646–12660).

- Selected Focused Hatred452405 scales native Demonsurge452416 damage by native effect0 minus effect1 per additional resolved enemy, for one through five enemies; six or more receive no bonus (SimC76f8118 sc_demon_hunter.cpp5608–5634).

- Selected Exergy206476 grants native208628 before Hunt370965 and Vengeful Retreat198793 execute. Refresh adds the native buff duration to its remaining time, capped at30s (SimC76f8118 sc_demon_hunter.cpp2822–2841,5501–5504,8539–8541,10010–10013). Burning Blades must not apply Exergy again to an already-modified source hit; the pinned SimC double application is recorded in [577](577.md#simc-double-applications).

- Native Chaos Strike199547 and Annihilation201428 off-hand execution roll the same197125 Fury refund passive. No refund occurs at the parent's earlier dummy hit. The real passive supplies its chance, Critical Chaos modifiers and500ms internal cooldown;193840 supplies the Fury payload (SimC76f8118 sc_demon_hunter.cpp7472–7510,7673–7685). Do not omit the transformed spender's refund.
- Selected Relentless Onslaught389977 rolls its native effect chance on actual landed off-hand hits. Its delayed callback uses that off-hand effect's native delay, selects Chaos Strike or Annihilation from live Meta at execution, then starts the native talent ICD. Copies use real free triggered casts and native weapon children; the live ICD prevents their off-hand hits from recursively producing copies (SimC76f8118 sc_demon_hunter.cpp7422–7446,7462–7473,7524–7532). The shared native queue retains its entity-lifetime and depth checks.

- Selected Shattered Destiny388116 accumulates native base Fury for genuinely paid literal Havoc actions only while owned162264 is live. Each native effect1 threshold extends Meta by native effect0 milliseconds; preserve the remainder across casts and exclude free/background copies and pre-Meta spending. The existing frozen resource context establishes positive payment (SimC76f8118 sc_demon_hunter.cpp2529–2555).

- Selected Blind Fury203550 supplies its native effect2 amount to Eye/Gaze's existing aura85 Fury regeneration effect. The ordinary native per-five-second regeneration path supplies Fury only during that actual channel, with no weapon-hit dependency or adapter credit (SimC76f8118 sc_demon_hunter.cpp9986–9993).

- Havoc Immolation Aura258920 grants its native Havoc initial Fury only; its periodic child258922 grants native Havoc tick Fury only while Burning Hatred320374 is selected. Do not apply the Vengeance energize effects to Havoc (SimC76f8118 `sc_demon_hunter.cpp`4651–4677,4784–4800).

- Hard Meta191427 adds native162264's duration to live Havoc transformation instead of discarding Demonic time. The ordinary native reapplication path retains aura effects and phase (SimC76f8118 sc_demon_hunter.cpp4984–4985).
- Actual Hunt370965 arms selected1270898's native1271144 Empowered Eye Beam. Completed Eye/Gaze with selected1270901 earns native1271092 Eternal Hunt; channel removal consumes empowerment. An actual dance consuming Eternal Hunt expires it and resets native dance recovery after500ms, not on unrelated expiration/cancellation (SimC76f8118 sc_demon_hunter.cpp4008–4017,5555,7296–7302).

- Actual Eye Beam/Gaze execution with selected452412 grants native453239 Student of Suffering. Actual Retreat damage-child198813 execution with selected389688 grants native389890 Tactical Retreat. Native periodic effects, durations and453236 energize child supply all Fury; the adapter never credits resources directly (SimC76f8118 sc_demon_hunter.cpp3339–3348,8567–8581).

- Native Consuming Fire452487/456640 retains Immolation Aura258920's payload and spends only that ability's Demonsurge use. Its dummy override cannot consume Annihilation's use or omit native Immolation ticks/Fury; Fel Desolation452486 is not Death Sweep (SimC76f8118 sc_demon_hunter.cpp181–189,4784–4839).

- Equipped native1296612 four-piece adds Essence Break to Cycle of Hatred: reduce its own recovery from prior stacks and earn the next native1214887 stack. While prior stacks are positive and below the native cap, Essence reduces the shared Eye/Gaze recovery once, and either beam reduces Essence recovery by native258887 effect0 milliseconds (SimC76f8118 sc_demon_hunter.cpp4033–4047,7814–7833). Without four-piece, Essence does not participate.
- Selected Chaotic Transformation388112 resets actual Eye Beam/Gaze and Blade Dance/Death Sweep recovery on ordinary Meta191427; selected Violent Transformation452409 restores Immolation charges and Sigil recovery. Completed native Eye/Gaze grants selected343311's actual343312 haste buff, never on interruption. Selected Cycle of Hatred258887 reduces each executed beam's recovery by its prior owned1214887 stacks × native effect0 milliseconds, then grants one native stack (SimC76f8118 sc_demon_hunter.cpp4002–4005,4033–4047,4987–4998). MID2 cross-cooldown reductions remain separate.

- Native Felblade232893 dispatches213243 at its actual selected primary; native child damage and effect2 Fury supply the result, not an adapter resource grant. A landed Essence Break258860 applies owned320338 after250ms. Eligible native Chaos Strike/Annihilation and Blade Dance/Death Sweep hits against that caster-owned mark dispatch1245759 at the native per-target internal cooldown (SimC76f8118 sc_demon_hunter.cpp7073–7080,7542–7550,7837–7843,7870–7893,11186).

- Selected First Blood206416 replaces Blade Dance/Death Sweep's primary with native391374/391378/393055/393054 at each authored effect64 delay. Base area strikes exclude only the immutable selected primary. The native selected talent effect0 supplies its primary damage bonus (SimC76f8118 sc_demon_hunter.cpp7040–7110,7252–7275,7324–7385).

- [ ] Native Burning Blades453177 receives the selected452408 percentage of landed swing, Chaos Strike/Annihilation, First Blood primary-strike and Throw Glaive damage. Native duration, period and rolling-refresh rules determine ticks; residual damage cannot feed itself (pinned sc_demon_hunter.cpp3088–3110).
- [ ] Selected Initiative388108 grants native391215 on the first positive direct enemy hit, without refreshing on repeated hits against that enemy. Vengeful Retreat's damage child resets those target trackers before its hits (pinned sc_demon_hunter.cpp2608–2620,8567–8576).

- [ ] `SIMC_HAVOC_DEMONIC_TRANSFORMATION`: native Eye Beam198013/Abyssal Gaze452497 with selected213410/spec577 enters native162264 before channel execution, using the live native talent effect0 duration plus the actual hasted channel duration (pinned sc_demon_hunter.cpp4051–4057). If the actor already owns live Meta, extend it without resetting effects or tick phase. No resource grant or damage adjustment; unrelated spells do not trigger the relationship.

- [x] Spec577 player measurements interpret the pinned76f8118 Havoc action lists through the shared literal driver, including source-order precombat, native selected hero tree, and native equipment facts. No replacement priority or resource grants. The final precombat Immolation Aura starts at pull without advancing its first tick or delaying the first combat action for its GCD (`player.cpp6153–6189`).
- Selected Havoc Monster Rising and Enduring Torment apply their native non-Meta stat buffs before combat; entering Metamorphosis removes them immediately, and leaving it restores them immediately (`sc_demon_hunter.cpp9204–9215,9233–9238,12136–12143`).
- [ ] Per-action Demonsurge availability reads the native producer's per-ability uses. Meta arms all source-defined abilities, Demonic re-arms Annihilation/Death Sweep, and each cast consumes only its own use. Native453323/1238696 supply explosion delays and452416 supplies damage/geometry; Meta expiry clears uses and native intensity/stack buffs (pinned sc_demon_hunter.cpp4976–4982,9153–9172,9265–9277,12645–12671). Missing soul-fragment pickups and inventory potions remain named gaps.

### Ret Dawnlight policies

- Selected Light Within1261111 supplies its ranked native effect1 as Avenging Wrath's effect12 direct-damage modifier, restricted by the native Wrath effect's family mask (SimC76f8118 `sc_paladin.cpp4175–4180`).
- Selected Light Within1261113 modifies Art of War406086 and Righteous Cause402916 via native label5745; foreground and triggered Blade of Justice retain those native damage modifiers when they consume the buffs (SimC76f8118 `sc_paladin_retribution.cpp:390–430`).

- [x] `SIMC_RET_DAWNLIGHT_WAKE_PRODUCER`: selected spec70 Dawnlight431377 plus actual ordinary Wake255937 produces native counter431522 from the live modified talent effect0. Native raw2 plus specialization+1 supplies3; retain the native maximum6, not an initial6. No consumer is implied by the producer proof.
- [x] `SIMC_RET_DAWNLIGHT_SPENDER_CONSUMER`: an eligible successful foreground HolyPower spender applies native431380 on the sourced target and consumes one actual431522 use. Retain native periodic payload, amounts, timing and stack cap. Pinned SimC76f8118 `sc_paladin.hpp1512–1535` owns the missing consumer relationship; `sc_paladin_retribution.cpp846–876` owns Wake production. TC has no script and DB2/TDB have no child edge/proc row. Both are explicit named policy-table entries with generic execution, never fabricated native metadata or core spell-ID branches. [Source and ordered proof](../handoffs/dawnlight-generic-proof-ledger.md).

### Ret reference knowledge

- [ ] `RET_HAMMER_OF_WRATH_KNOWN_FROM_SELECTED_TALENT_SIMC` permits action24275 only on explicitly prepared spec70 parity/reference actors whose validated active TraitLoadout includes1241288 (including native granted roots). Ordinary native player/follower spellbooks and NeverLearned rules remain unchanged. Removing eligibility must not retain the exception; other specs receive no grant. [Source/proof ledger](../handoffs/ret-paladin-parity-proof-ledger.md#explicit-hammer-of-wrath-knowledge-policy).

### Ret Divine Purpose producer

- [ ] `SIMC_RET_DIVINE_PURPOSE_TRIGGER`: only validated selected408459 may roll its actual native effect0 amount after an ordinary HolyPower-cost cast (including a zero final cost). Trigger native Ret408458 after the old cast's modifier charges are consumed (SimC76f8118 `sc_paladin.cpp4342`); generic223819 is not its replacement. Aura-only unselected actors, non-HolyPower actions and full-mask background casts do not qualify, except the explicitly sourced Empyrean Legacy Storm producer. TC registers older223817; the selected408459 relationship is explicitly pinned SimC-derived. No chance, resource or coefficient override. [Proof ledger](../handoffs/ret-paladin-parity-proof-ledger.md#ret-mid2-tier-relationship-source-policy).

### Ret Aurora producer

- Selected439760 grants native408458 after ordinary Wake255937 execution and the sourced Dawnlight budget grant. Aurora alone owns its native internal cooldown; generic DUMMY proc hints cannot consume it. The grant is guaranteed, not another spender-chance roll, and does not create Holy Power or precombat Arbiter uses (`sc_paladin_retribution.cpp878–881`, `sc_paladin.cpp73–74`).

### Ret MID2 tier relationship

- [ ] `SIMC_RET_TIER_MID2_DIVINE_PURPOSE_RELATIONSHIP`: consuming actual foreground Divine Purpose408458 with native2062 two-piece grants native Divine Power1305230. Four-piece arms the spender's native Verdict/Storm/Light Arbiter kind only when none is pending; the same kind preserves it, a different kind consumes it and casts native1306923 at the preserved hostile primary, not a self-centered spender's cast unit. Native gear eligibility, effects, coefficients and durations remain authoritative. No fabricated DB2 trigger edges. Hammer of Light runtime qualification remains a separate open case. [Source/proof](../handoffs/ret-paladin-parity-proof-ledger.md#ret-mid2-tier-relationship-source-policy).

### Ret Storm root supply and secondary relationships

- [x] Native Aura361 replaces an eligible main-hand swing with its native TriggerSpell, or an off-hand swing with its native MiscValue spell, following TC first-applicable ordering. Preserve native timers, recipients and attack predicates; do not also deal ordinary white damage, substitute a target or branch on a Ret spell ID.
- [x] `SIMC_RET_SECOND_SUNRISE_STORM_ECHO` follows the source-qualified ordinary native Storm eligibility, live chance/effectiveness and native ICD. Store the relationship as explicit policy data with generic execution. Read native4442 values at runtime, not hard-coded20/50 or guide15/30. Do not add a200ms delay. A background echo retains the actual hostile primary, has no additional Holy Power cost and does not recursively echo itself. Qualify ordinary, relevant background-source and recipient boundaries separately; do not fabricate a proc row or apply a blanket DPS multiplier. SimC's TODO200ms is not established native timing.
- [x] Strict native tests must prove real swing damage or actual Wake funding/paid Storm damage before the missing boundary; retain unselected controls. Commit tests before RED and generic execution before GREEN.
- [x] Report root supply, Second Sunrise, Tempest and background-source coverage separately. [Scoped native/compiler proof](../handoffs/ret-storm-gap-proof-ledger.md). Missing echo damage does not explain the full ranked Storm deficit.

### Ret Hammer Second Sunrise

- Ordinary HoW successful primary impacts may produce a native same-ID background echo (`sc_paladin.cpp1689–1710/1757–1783`). Missed/immune primaries, secondary recipients, background HoW and echoes do not produce it.
- Read selected native431474 chance/effectiveness and share its native ICD with Storm. Preserve native payload damage, recipients and existing background execution relationships, including native Sun Sear from critical HoW echoes. Do not tune values, grant synthetic resources or implement the source's unverified200ms delay.
- Native actual-combat RED→GREEN must show ordinary root damage and correctly scaled echo damage, with unselected/background controls; retain all Ret/Dawnlight gates and remeasure three-seed ST/3T.

### Ret Divine Resonance producer

- [ ] `SIMC_RET_DIVINE_RESONANCE_PRODUCER`: validated selected384027/spec70 and an actual ordinary Divine Toll375576 cast grant native1266308 at its native maximum3 stacks, through existing AfterCast after cast-success is raised. SimC76f8118 `sc_paladin.cpp1880–1905,3686–3688` supplies the missing relationship; `action.cpp1971–1985` dispatches cast-success callbacks before that grant. Keep native30s duration; no legacy5s pulse, new hook, per-spell proc suppression or copied timer. Echo, stack spending and500ms ICD remain excluded. [Source/proof](../handoffs/aura-proc-event-candidates-proof-ledger.md#ret-producer-phase).

### Ret Execution Sentence accumulation

- Actual343527 launches native1260251 damage/gather effects at its hostile primary. Own nonphysical damage to any owned1260251-marked recipient contributes to that caster's primary343527 pool, excluding every Physical school bit, including hybrids. Native effect1 owns the stored percentage; no fabricated proc flags/chance.
- Expiry alone casts387113 with stored BP0 through partial-trigger admission, never FULL_MASK; cancellation must not discharge. SimC Ret35–75 applies normal source/target multipliers after adding stored damage, overriding only native multiplier-suppression flags while retaining native no-crit. Exact controlled-pet attribution remains unqualified. [Existing expiry-admission qualification](../handoffs/ret-qualified-sentence-wake-toll-proof-ledger.md).

### Ret damage children and mastery

- Blade snapshots Art of War/Righteous Cause before consumption and schedules native1261160 after350ms, with native1261159 primary effectiveness and reduced AoE beyond8. Selected stacking1261113 consumes one Righteous Cause stack. Learned404834 also produces native26573 Consecration with the pinned10s producer ICD.
- Judgment-family enemy damage impacts roll383921 from native mastery effect3, Boundless Judgment and Highlord's Wrath values. Caster/self-aura recipients never enter the damage-only hit hook. Ordinary24275 participates; background same-ID24275 does not. Highlord's Wrath enhances only the hidden native mastery whitelist; never duplicate already-applied base mastery.
- Storm produces224239 Tempest, or a53385 Tempest for a Sunrise echo, at native383396 effectiveness. Retained primary/echo/secondary cast roles prevent recursive children and extra resource consumption.
- Critical ordinary/Legacy Storm and HoW impacts apply native431414 Sun Sear. Each actual Dawnlight tick produces431911 Sun's Avatar with native area/cap values. Critical still-ticking403695 extends its owned duration by native446663 effect0 without resetting tick phase.
- Divine Arbiter1306923 uses its native first damage effect on the primary and its native second effect on other recipients, never both on the primary. Spell coefficients remain native70448 values.

### Ret Judgment target uses

- Selected Greater Judgment grants native197277 initial uses on successful Judgment/HoW-family enemy impacts, including sourced background variants. Selected Lingering Radiance grants one use when owned Dawnlight ends naturally.
- The native target effect supplies its amount, class mask, caster ownership, cap and duration. Stored uses expire independently; matching successful impacts consume one use. The damage bonus remains one native percentage regardless of use count; Tempest and Sunrise Storm do not consume uses. Unrelated incoming damage effects retain ordinary stack scaling.

- Pending for the unselected Radiance choice: cancellation invokes `last_tick` (`dot.cpp75–81`), but its cancellation→Judgment relationship is not implemented.

### Ret Crusade

- Selected1253598 makes31884 start at1 stack and cap at10. Foreground Holy Power consumers add their actual paid amount, or the native base HP equivalent when Divine Purpose, Empyrean Power or Shining Light makes them free. Pinned Hammer of Light grants9. Legacy, echo and Tempest background actions do not add stacks; no HP is granted or altered.
- Native1253598 effect0 supplies haste per stack, multiplying existing haste. Base Wrath effect amounts, including crit and damage, ignore stack count. Stack gains and reapplications preserve the active Wrath expiry (`sc_paladin.cpp3644–3651,3911,4617–4618,4647–4648`; `sc_paladin.hpp1541–1555,1614–1623`).

### Ret Divine Toll relationship

- [ ] `SIMC_RET_DIVINE_TOLL_RELATIONSHIP`: selected spec70 normal375576 hits dispatch native406957, or1279408 while actual1277026 mode is active, once per real hostile target. Native effect0 cap and effect1 area geometry/admission bound recipients; preserve existing combat-only implicit admission and count the explicit primary once. Native child coefficients and parent effect5 percentage remain authoritative; no invented DB2 trigger edge. Child funding, Resonance, Exaction and Divine Hammer remain separate qualifications. [Source/proof](../handoffs/ret-qualified-sentence-wake-toll-proof-ledger.md).

### Wild Thrash owned-pet child policy

- [ ] `HUNTER_WILD_THRASH_PET_CHILD_SIMC`: native1264359 effect0 commands each actual living owned controlled pet whose native1264355 target plan reaches the selected enemy. Damage is cast from that pet through the existing owned-pet command path, preserving native area/coefficients and copy/ownership/range rules. No pet means no fabricated child caster. TC has no script and parent DB2 Dummy effects have zero triggers; SimC76f8118 hunter7562–7563/6796–6837 supplies only this named dispatch policy. Beast Cleave, conditional300% multiplier/cap semantics and unsupported companion/guardian models remain separate qualification gaps.

### Hunter pet survivability

- [ ] Actual controlled Hunter pets use native ExpectedStat CreatureHealth at their own level and template health/tuning metadata, rather than legacy class-level placeholder HP. TC Pet.cpp847–925 supplies base armor of level×50 when pet_levelstats is unavailable; no fake intrinsic primary-stat constants are imported.
- [ ] TC Guardian stat scaling adds30% owner Stamina (integer native stat)×10 HP and70% owner armor. Before summon snapshots its owner, the existing shared Stats producer resolves current gear/passives; a fresh pet therefore starts full under that resolved maximum. Spawn initializes this maximum before stored current-health hydration; persisted wounds remain wounded. Real owner stat changes refresh inherited deltas without healing, clamp currentHP on maximum reduction, and preserve unrelated pet health bonuses. Demon/Mage pets and ordinary NPCs are outside this slice. SimC stable_pet70%Stamina/1.05armor remains named divergence `HUNTER_PET_SIMC_SURVIVABILITY_DIFF`, never mixed into runtime values.

### Training dummy combat exit

- [ ] Nondamaging spells do not refresh an existing dummy damage timer. Expiry clears only that PvE pair's threat/references; other attackers stay engaged until their own timers expire, and out-of-range attack intent remains intact.

- [ ] Native31146 uses TC a352b1fa `npc_training_dummy`/`NullCreatureAI`: each attacker leaves combat5s after its last non-DoT hit; DoT damage does not refresh that timer. Combat exit removes only LeavingCombat-interrupt auras, not every foreign aura, and does not enter evade. A still-live native DoT must tick beyond5s. Ordinary CreatureAI `_EnterEvadeMode` retains its conditional aura cleanup (`CreatureAI.cpp298–314`). Source: `npcs_special.cpp1382–1421`, `PassiveAI.h55–67`, `Unit.cpp9257`; no harness timer/evade exemption.

### Hunter pet melee owner RAP

- [ ] Actual controlled Hunter pet auto-attacks include the TC StatSystem.cpp1344/1389/1423 owner contribution: integer22% owner total ranged AP (including equipped weapon AP through the ordinary producer), divided by3.5 and multiplied by the source two-second pet base attack time, additive before existing damage modifiers, armor and hit outcomes. Real later owner stat changes are read live; a caster-only melee-AP buff must not change pet swings when owner RAP is unchanged. Spell-command AP coefficients, Spirit Wolf, Demon/Mage pets and ordinary NPCs retain their existing behavior.
- [ ] Hunter pet ability damage inherits the same integer22% owner total RAP source as melee, including equipped weapon AP; native1264355 damage qualifies the actual effect boundary before the shared input changes. Ability coefficients/variance and native target geometry stay unchanged.
- [ ] Intrinsic pet AP/weapon damage remains an explicit unqualified gap; do not invent pet_levelstats or import TC's fake primary stats/approximate weapon range. SimC stable_pet60% ownerAP is named divergence `HUNTER_PET_SIMC_OWNER_AP_DIFF`, not mixed with TC22% RAP. Contribution proofs do not certify total pet DPS or Azure survival.

### Arms literal controller

- With selected Bloodletting383154 and Rend772, actual Mortal Strike impacts (including background Unhinged) apply native Rend388539 only below Bloodletting's native effect3 health threshold. Ordinary higher-health impacts cannot grant it (`sc_warrior.cpp3122–3123`).

- Equipped MID2 Arms4pc1296644 grants native Winding Up1300670 once per Mortal Strike/Overpower execution, including background MS, preserving its native ICD/magnitude/cap. Slam-family executions (1464/1269383) cannot produce it and expire all current stacks only after their damage resolves (`sc_warrior.cpp3111–3112,5494–5495,3537–3538`).

- Selected Heroic Might1269307 accumulates native-capped Heroic Strike uses separately from its active Colossus Smash window. The next CS transfers pending stacks into native1292058 for the wound's native duration and clears pending uses; Heroics after CS cannot change its current window. Native label2443 modifiers supply both spell/white-swing vulnerability amounts (`sc_warrior.cpp3571–3573,3761–3764,1729–1730`).

- Selected Just Warming Up1261048 grants its native effect1 raw Rage once after completed Colossus Smash167105. AoE victims do not multiply that resource grant, and automatic swing Rage is independent (`sc_warrior.cpp3781–3782`).

- Selected Master of Warfare1269306 binds the native damage/critical-damage modifier effects of its earned1269394 buff from the talent's rank values divided by10. Each actual stack applies those modifiers; the buff's separate native armor penetration is unchanged (`sc_warrior.cpp7888–7891,7902–7907`).

- Equipped MID2 Arms2pc1296643 makes each Slam-family execution (1464 and inherited Heroic Strike1269383, including background Slam) launch native Concussive Slam1300660 once. Its damage excludes the original primary; native payload selection retains radius/enemy eligibility (`sc_warrior.cpp3443–3453,3534–3535,3551–3562`).

- With selected Fatality383703, actual Execute damage260798 resolves native383706 against the caster-owned383704 marks. Damage scales with the actual mark stacks, cannot crit, and consumes those marks on its impact without consuming another caster's marks (`sc_warrior.cpp4251–4252,4404–4423`).

- With selected Tactical Edge1261051, each completed Colossus Smash167105 grants Sudden Death52437 uses from the native talent's effect1. AoE recipients do not multiply the grant; natural Sudden Death RPPM remains independent (`sc_warrior.cpp3758–3759`).

- Slayer's Malice444779 applies its first native effect for Arms and its second for Fury, not both overlapping damage modifiers (`sc_warrior.cpp7877–7879`).

- Selected Mortal Wounds1261062 applies native Deep Wounds262115 only after an actual Mortal Strike12294 or Slam-family critical impact (Slam1464 and inherited Heroic Strike1269383), including background Unhinged/Fervor attacks. Normal hits cannot grant it. Shared1261060 remains the separate Execute producer and Fury's native relationship is unchanged (`sc_warrior.cpp3153–3154,3518–3519,3551–3562,4248–4249`).

- Selected Arms Anger Management reduces only running Avatar and Colossus Smash native endpoints from actual paid Rage and native effect0's displayed-Rage divisor. Ignore Pain and zero-payment casts do not contribute (`sc_warrior.cpp1450–1455,1519–1527`). Fury's separate effect2 policy is unchanged.
- Selected Tactician rolls its native effect0 chance once per positive foreground Rage payment. Selected Brute Force adds its native chance only for Slam/Heroic Strike. A success restores one Overpower charge and grants selected native Opportunist456120 (`sc_warrior.cpp1458–1471`).

- Cleave rolls selected Reap the Storm446005 once per execution only when its native selected-recipient count meets444775 effect1's threshold, using effect2's native chance. A real Sudden Death Execute (including native Massacre281000) can dispatch selected Reap through Imminent Demise's native effect1 chance. Paid Execute alone cannot dispatch it. Reuse the native444775 internal cooldown and unchanged payload values (`sc_warrior.cpp4347–4359,4547–4559,7980–7981`).

- [ ] Sweeping Strikes grants the current native effect4 use count (12 in70448). Each affected single-target attack uses the native extra-target count and secondary damage percentage; one use is consumed only when it hits at least two targets. Preserve the native primary strike, costs and cooldowns (`sc_warrior.cpp1230,1256–1273,1324–1325,5287–5294`).

- [ ] Ordinary Execute163201 and native Massacre281000 snapshot actual paid Rage before taking costs. Their native260798 payload scales by twice paid/native maximum Rage; a genuinely free cast retains the full two-times multiplier. Resolve the maximum from the applicable native base-plus-optional cost row, not remaining power at child delivery. Preserve native coefficients and apply the native surviving-target refund to both parents (`sc_warrior.cpp4212–4238,4335–4348`).

- [x] `SIMC_ARMS_MASTER_OF_WARFARE_REPLACEMENT`: ordinary Heroic Strike1269383 requires the actual1269391 proc and consumes it after successful execution. The source-qualified producer uses the shared PRD helper at pinned15% only for eligible foreground Mortal Strike, Overpower, Slam and Execute damage while the proc is absent; native101 metadata is a script placeholder, not a guaranteed proc. White attacks, Cleave and the consuming Heroic Strike do not produce it. Keep native proc duration, replacement payload and costs (`sc_warrior.cpp1666,3164–3168,3512–3516,3565–3581,4254–4258,5468–5472`).

- [x] `SIMC_ARMS_CLEAVE_REND`: an Arms Cleave hit applies native388539 to each landed target only with selected Rend772. Pinned76f8118 `sc_warrior.cpp3685–3686` supplies the missing relationship; native payload amounts, duration and periodic rules remain unchanged.

- [x] Spec71 parity executes the exact reference manifest's action lists through the shared literal evaluator, native readiness/execution and equipment facts. Capture control before warmup, cast the source precombat Battle Stance once, and use native selected targets for Slayer/Colossus lists. When no native follower profile exists, use the real player actor; never fabricate a follower. Stockade/native follower rotations remain unchanged. Potion inventory and external Power Infusion are explicit gaps.

### Fury player parity

- Selected Storm of Blood1299025 applies native Rend388539 to each enemy hit by Whirlwind's first main-hand impact199667 (`sc_warrior.cpp6526–6540`). Later weapon hits do not reapply it.

- With selected Cold Steel, Hot Blood383959, critical Bloodthirst23881 and Bloodbath335096 impacts apply native Gushing Wound385042 to the struck target, including Unhinged impacts (`sc_warrior.cpp2679–2682,2915–2919`). Noncritical impacts do not.

- Landed Bloodbath335096 applies its native bleed113344 to its actual target; subsequent landed Bloodbaths extend the existing owned bleed by its native duration (`sc_warrior.cpp2777–2785,2915–2923`).

- Selected Cruelty392931 and Wrath and Fury392936 damage modifiers apply only while native Enrage184362 is active (`sc_warrior.cpp1106,4939–4947,5099–5113,7834–7835`). Their native rank values and other effects remain unchanged.

- Fury's selected Imminent Demise444769 and Sudden Death29725 accelerate native Bladestorm periodic attacks according to earned445606 stacks, using the native modified duration. Haste the spin duration and expire those stacks at its natural end (`sc_warrior.cpp3279–3299,3344–3349`).

- Selected Spite383852 applies its native rank values in tenths of a percent to damage and critical damage, as specified by the native display and SimC76f8118 `sc_warrior.cpp7894–7899`; preserve its native masks and rank curves.

- Selected Focus in Chaos383486 removes white-swing miss chance while the caster's Enrage184362 is active; dodge, parry and special attacks remain separate (`sc_warrior.cpp2074–2080`).

- Hack and Slash383873 increases only the next complete Raging Blow/Crushing Blow weapon sequence. Both native delayed weapon hits retain its native bonus; the final off-hand execution consumes one stack, even though its native proc-charge count is zero (`sc_warrior.cpp5061,5227`).

- Rampage184367 may schedule its native effect5 Rampaging Ruin child only with selected Rampaging Ruin1265357 and live Whirlwind85739. An unselected talent or absent Whirlwind cannot add the AoE hit. Retain the original native effect64 payload, delay, admission and damage values (`sc_warrior.cpp5727–5748,5800–5801`).

- Shared Unrelenting Onslaught uses owned Executioner stacks after a genuine Sudden Death Execute to reduce the current native Bladestorm endpoint by444780 effect0 seconds per stack and apply native445836 to the actual primary using effect1 stacks per Executioner stack. Paid Execute alone does not qualify; native producers and counters remain unchanged (`sc_warrior.cpp4361–4365,4561–4565`).

- Slayer's Strike445579 uses one actor-wide PRD attempt counter, native Slayer's Dominance444767 chance (effect0 Fury, effect1 Arms), and its native ICD. Only pinned eligible landed impacts roll; autos never roll, and only Cleave rolls per target (`sc_warrior.cpp1651–1661,1693–1703`). Native spell payloads supply all damage.

- Arms/Fury ordinary Execute consumes one live Sudden Death52437 stack after its foreground producer roll. A consumed stack grants native Imminent Demise445606 when both native passives29725/444769 are selected; background casts do not consume it (`sc_warrior.cpp4347–4352,4547–4554`).

- Native Sudden Death29725 rolls its own RPPM and ICD once per foreground Warrior special execution, not on autos or background impacts; a success grants52437 and resets Execute. Selected Imminent Demise444769 independently counts Slayer's Strike445579 executions, including background ones; its native effect0 threshold grants Sudden Death and resets Execute without rolling the ordinary producer's ICD (SimC76f8118 `sc_warrior.cpp1710–1718,2428–2435`).

- [x] Spec72 uses `--actor player` and the complete pinned MID2 Fury APL through shared `literal_rotation`. Capture literal control before native setup, execute ordered precombat and submit normal native casts. Do not invent a follower profile. Native overrides select Reckless Abandon and Unrelenting Onslaught variants; native stats, costs, coefficients and trait legality remain authoritative.
- Fury Enrage184362 grants its native haste/speed effects only with Frenzied Enrage383848, and its native mastery/leech effects only with Powerful Enrage440277 (`sc_warrior.cpp9623–9626`). Selected Rampaging Berserker1269309 reduces Rampage's paid Rage cost and increases its child damage while Recklessness1719 is active, using its native rank values (`sc_warrior.cpp5528–5531,5756–5762`). Fury MID2 two-piece1296645 extends active Recklessness on Raging/Crushing Blow up to its native per-hard-cast cap (`sc_warrior.cpp5053–5058,5219–5224,7166–7167`). Improved Raging Blow383854 can reset one Raging Blow/Crushing Blow charge, adds selected Wrath and Fury392936's chance while Enraged, and grants selected Surge of Adrenaline1265560 on a successful reset (`sc_warrior.cpp4995–5034,5170–5199`).
- [x] Fury Deep Wounds1261060 accepts only actual Execute main-hand280849 impacts; Whirlwind cannot grant it (SimC76f8118 `sc_warrior.cpp4452–4456`). Preserve native262115 damage and rolling periodic behavior; do not alter Arms71 producers. Source-qualified Deep Wounds rolling refresh retains the pending full tick plus a fresh native duration, redistributing old unpaid and new native damage budgets across that schedule (`sc_warrior.cpp3798–3805`, `action.cpp4614–4617,4995–5016`). Native rolling spells use the same remaining-plus-fresh budget across the carried tick schedule.
- [x] Anger Management152278 reduces running Fury Avatar/Recklessness cooldowns from actual paid Rage, using native PowerType display units and effect2's Rage-per-second divisor; free background casts do not contribute (`sc_warrior.cpp1450–1455,1506–1517`).
- A landed native Bladestorm main-hand50622 tick dispatches its native effect1 off-hand area95738 once for Fury72 with an actual off-hand weapon, not once per main-hand recipient. Arms cannot acquire that off-hand attack (`sc_warrior.cpp3249–3254,3335–3340`). Native child damage and area selectors remain authoritative.
- [x] Selected Unhinged386628 dispatches Arms71 Mortal Strike12294 or Fury72's live native Bloodthirst23881 variant on odd Bladestorm227847/446035 effect0 ticks, including catch-up ticks and excluding tick0, using the actual selected hostile target and native effect1 damage multiplier. Arms Unhinged Mortal Strike does not cleave or consume Sweeping Strikes; foreground strikes retain Sweeping. Preserve ordinary parent pulses (`sc_warrior.cpp3057–3072,3257–3275,3313–3335`).
- Frenzy335077 grants one335082 stack per final Rampage201363 execution, not per impact or arbitrary melee event. Its native passive effect0 supplies each stack's value (`sc_warrior.cpp5581–5588,8411,9616`). Source-owned proc policies must prevent the broad native proc hint from independently firing the same aura or consuming its ICD.

### Destruction Echo of Sargeras

- Selected Embers1265770 advances source PRD only on real Incinerate29722 (including FnB) and Infernal Bolt434506 hits, using native label-modified chance. Selected1265774 actual Chaos Bolt/Shadowburn hits and ordinary in-combat Rain of Fire execution share its native internal cooldown. Pinned `bugs=true` uses full native Echo payload for spenders (SimC76f8118 actions450–455,4337,4605,4813,5614–5649,6039–6042; init1428–1434).
- Native1265884 primary/secondary effects hit each recipient once; actual primary is excluded from its secondary effect. Preserve their native coefficients. Earn native1265939 through Echo and selectedII, and1305711 only from actual Echo impacts with genuine ItemSet-owned1296572. Do not replace authored label values or use the broad tier proc hint. Frozen267 selects EmbersII rank2: native Vision is8% haste/crit, matching pinned reference `default_value=0.08`, not the raw per-rank4 (init846–849). ST/3T qualification does not certify the above-eight-target reduction.

### Destruction Chaotic Energies

- Native77220 mastery affects only the pinned `affected_by.chaotic_energies` action set. Actual unresolved direct hits and periodic updates sample1+mastery×U(0.5,1). The source-marked Chaos Bolt, Shadowburn and Rain of Fire payload instead use selected native387275 effect0 as the lower fraction. Read native mastery values; do not adjust damage coefficients (SimC76f8118 `sc_warlock_actions.cpp480–489` and each action's explicit flags).
- Read-only direct-damage projections use the analytic mean without advancing native RNG or casting. Already-resolved copies do not receive a second multiplier; pets, items, racials and unmarked actions receive none from this relationship.

### Controlled Imp inheritance

- The controlled Imp (native CreatureFamily23) engages its target for Firebolt but never starts auto-attack. Other controlled demons keep their melee. No 70448 DB2 field marks caster pets, so the native family selects the rule (SimC76f8118 `sc_warlock_pets.cpp360–385`; Felhunter/Voidwalker/Felguard melee at 342/431/503).
- A controlled Felguard17252 applies the pinned 1.44 multiplier to its actual white swings, not its spell damage (`sc_warlock_pets.cpp661–668`).
- Statless actual Warlock demons inherit their owner's attack haste for melee swings, while retaining native creature timing and their own melee modifiers (`pet.cpp222–228`).
- The actual controlled Imp416 inherits live owner spell power at5.625 for SP and2.8125 for AP (SimC76f8118 `sc_warlock_pets.cpp372–375`). Controlled Sayaad1863 inherits live owner spell power at5.175 for SP and2.565 for AP, including melee AP (`sc_warlock_pets.cpp421–430`). Preserve native spell coefficients, UNIT_PET identity, pet orders, stance, costs and storage. Do not replace either pet with a guardian or fixed damage payload. Unlisted controlled pets retain their existing power policy.
- A target's owned Whiplash6360 stacks (native aura270, mask127) increase all of that Sayaad's damage, including real white swings; another caster's stacks do not (`sc_warlock_pets.cpp447–452`).

### Shared pet prerequisites

- Headless native Player actors must have runtime-only native pet storage before a real summon. Do not fabricate an authenticated character, follower identity, actor or resource grant. Hunter default-record provisioning reuses that same storage.
- Generic implicit27 TARGET_UNIT_MASTER selects the actual native controlled-pet or temporary-summon owner in the same map, independently of the supplied explicit target. Never substitute the caster, controller actor or a resource grant (TC a352b1fa Spell.cpp1753–1755).
- Generic effect148 must forward its actual hit destination/unit, native costs and value to every child effect on the per-cast clone. Do not mutate catalog data (TC a352b1fa SpellEffects.cpp725–788).
- Generic implicit85 is DEST_DEST_FRONT_LEFT: use native radius/direction, native caster origin when no destination is supplied, and existing collision/allowed-Z handling. Do not use spell-ID branches (TC a352b1fa Spell.cpp1690–1738).
- Qualify the native267 summon regressions and protected255/254/253/265 probes before landing shared behavior. Pet attack lists, coefficients and class producers remain separate work.
### Destruction guaranteed-crit damage

- Selected Conflagration of Chaos387108 scales Conflagrate17962 and Shadowburn17877 damage by one plus the live caster spell-crit fraction. The ability-specific guaranteed crit is not the multiplier input. Actual damage and pure forecasts must share this rule without rerolling damage or rescaling resolved copies (SimC76f8118 actions4511–4517,4838–4844).
- Preserve the existing Chaos Bolt crit-chance scaling; do not apply it twice. Native crit changes, not a reference-derived damage coefficient, must drive this relationship.

### Seed of Corruption lifecycle

- Actual paid27243 snapshots its caster's native spell power times effect0 percent as the damage threshold. Its caster's actual landed damage consumes that owned threshold. Reaching zero or natural expiry queues real27285 after Seed removal, without nested explosions or catalog/coefficient changes (SimC76f8118 `sc_warlock.cpp380–405`, `sc_warlock_actions.cpp1927–1965`).
- Reapplication extends the aura but must not move its original periodic explosion deadline. The declarative `PreservePeriodicPhase` registry policy retains that native deadline; Seed registers only27243. Without selected Sow196226, one paid cast applies one Seed, not one to each raw area recipient (`actions1800–1802`). A real27285 impact chains an owned Seed and applies background native Wither445474 for selected445465, otherwise Corruption146739, without the initial direct Wither payload or a new cost (`sc_warlock_actions.cpp1710–1773`). Preserve shared Wither counters and tick phase; do not register or replace UA1259790 lifecycle callbacks.
- The primary prefers the original target without its caster's Seed; otherwise it selects an unseeded candidate and uses the original only if none remain. The read-only sibling helper `warlock_seed::selected_primary(world,caster)` returns that actual selection for the same successful Seed execution, before projectile/explosion. Other owned leaves reuse it without another target draw. Pinned265 bugs mode ignores seeds in flight (`actions1836–1892`). This helper does not apply UA.
- Qualify actual threshold, expiry, refresh/original-deadline and recipient ownership, with native265 Seed and267 shared-Wither controls. Native paid-recast redirection needs its own RED→GREEN. Selected-Sow effectiveness, death/dispel and MID2 seed-applied-UA relationships remain separately qualified; do not infer them from a zero-damage periodic tick.

### Eye of Infinity

Selected411165 applies its native effect0 to the primary recipient of ordinary and Scintillation Eternity Surge359077, not its secondary recipients (`sc_evoker.cpp`5541–5548).

### Absolute Corruption lifetime

- Selected196103 on actual Affliction265 makes owned Corruption146739 and Wither445474 permanent, including background Seed applications and reapplications. Preserve native effects, counters and periodic phase; do not change catalog duration or read fixture fight clocks. Pinned SimC models permanence as twice the expected fight length (`actions759–762/1072–1077`).
- Generic DoT facts for an active permanent aura are not refreshable and have unbounded remains/duration, before finite fresh-duration/Pandemic validation. Missing/finite DoTs and buff/debuff sentinel semantics stay unchanged. Do not suppress literal casts in the adapter to hide an expiring aura.
- Qualify the actual literal ST and recurring30s-add-wave ordering. Gate265 ST/3T and267, plus actual finite-DoT refreshability from103 or258. Count explosion hits separately from explosion actions; do not tune damage or target counts to repair excess paid Seed refreshes.

### Pet critical damage

- Actual pet and guardian spell critical hits inherit the owner's critical-damage multiplier in addition to the pet's own multiplier, including equipped effects (SimC76f8118 `pet.cpp:488–494`). Preserve each spell's critical bonus and independently applied pet modifiers.

### Guardian periodic haste

- An actual guardian's periodic aura with native spell-haste-periodic attribute reads the same actual owner's resolved spell haste as its guardian cast time. Apply the aura's own period modifiers on the guardian, preserve native tick phase and unflagged periods, and do not substitute owners for ordinary units (SimC76f8118 `pet.cpp217–228`, `buff.cpp752,1945–1954`). Qualify actual Infernal19483→20153 tick spacing plus 265,266,253 and generic guardian controls.

### Native Infernal guardian

- Actual111685 summons89 with live owner-SP shares2.2275 for SP and AP (SimC76f8118 `sc_warlock_pets.cpp2050–2119`). Native264364→264365 energizes the existing owner through target27; native19483→20153 emits its authored periodic area damage. Preserve native costs, coefficients, targets, clocks and actor expiry.
- Immediately after actual guardian initialization, the generic `SummonInitialized` callback receives that actor. The downstream Infernal leaf initializes existing action clocks once: a nonnegative normal arise delay(mean1s,stddev0.1s) starts Embers; Immolation starts after another uniform0–0.75s, without an Immolation zero tick. No per-unit summon wrapper, actor-query loop, synthetic ticks or new timer state. Unregistered guardians retain existing behavior; qualify BM Bear,265 guardians and DK Rune Weapon.

### Native Overfiend guardian

- Actual434587 summons217429 with live owner SP1.0 and native repeated434589. Its spell coefficient borrows native116858 effect0 times native Avatar1245089 effect0; guarantee its critical outcome separately from live owner spell-crit scaling and the sourced owner-mastery range (SimC76f8118 pets2358–2407).
- Actual ordinary Soul Fire execution summons one Overfiend; the native Avatar proc hint is too broad and must not summon on unrelated casts. The separate Dimensional Rift deck producer is not yet qualified. Actual creation grants native457578 to the existing owner through `SummonInitialized`, not an unreachable per-unit summon wrapper. Preserve native aura duration/period/raw energize, actor expiry and catalog values (actions4190–4205/init851–855). No synthetic bolts, resource grants or new timer state.

### Shared Hellcaller Wither counters

- Mark of Xavius440046 registers effects1/3 for Affliction and all effects except1 for Destruction; thus its native Wither damage bonuses apply only to Destruction (SimC76f8118 `sc_warlock_init.cpp:82–83`).
- Native Wither445474 keeps its actual counter stacks, but its periodic damage ignores that count. An ordinary recast preserves earned stacks without adding a refresh stack; explicit native stack applications retain normal counter advancement. Preserve native coefficients, tick phase and lifetime. Both265 and267 share this rule (SimC76f8118 `sc_warlock_actions.cpp1066,1174–1186`).
- Wither's counter is not an ordinary damage-stack multiplier. Its target-side Ashen Remains effect applies once regardless of counter stacks (`sc_warlock_actions.cpp:273–275`); preserve the separate Dark Titan's Mark effect. Other stacking DoTs keep their existing behavior; Blackened Soul producers/collapse are separate relationships.
- Shared265/267 Hellcaller producers must use actual Chaos Bolt/Shadowburn impacts and actual UA applications, including refresh, Fatal Echoes and seed-applied UA. Do not replace UA's independently owned expiry callbacks. Grant native Malevolence stacks/buff/damage only to its caster's active Withers; preserve each Wither lifetime and tick phase.
- Selected Devil Fruit1266805 attempts its native RPPM on actual owned Wither ticks and applies Malevolence442726 with the talent's native effect0 lifetime, without executing the button's damage/stack producers. Both earned procs and the actual button extend existing Malevolence; preserve native buff effects and ordinary button duration (SimC76f8118 actions1154–1159; init968–971,1475).
- Check Seeds gates on gains and Wither ticks. Blackened uses a permanent runtime counter with the native tick period, decrements Wither before damage/Mark of Xavius, consumes its separate collapse budget, and stops at Wither1 or missing Wither. Preserve source PRD/weighted-threshold behavior and native spec-specific payout buffs; do not tune damage or duplicate shard mechanics (SimC76f8118 actions1119–1149,1208–1317,1544–1565,4328–4331,4807–4810,5534–5611; init1441–1480).

### Destruction main-pet engagement

- The literal267 controller commands its actual configured main pet against the current encounter primary through native pet orders (SimC76f8118 `pet.cpp67–68,149`). Preserve UNIT_PET identity, saved reaction stance, native attacks, costs and lifetime; do not substitute a guardian or manufacture pet damage.

### Destruction Fiendish Cruelty

- Actual critical Incinerate29722 (including selected secondary impacts), Chaos Bolt116858, Conflagrate17962 and Infernal Bolt434506 attempt selected Fiendish Cruelty1245633 using the native effect0 chance and one shared accumulated PRD stream. Successful attempts earn native1245664, whose existing aura-state exemption and native single charge permit and pay for actual Shadowburn17877 above its ordinary health threshold. Consume the earned charge on actual Shadowburn execution; do not roll on unrelated hits, grant shards or tune the native chance (SimC76f8118 actions433–439,3860,3936,4293,4446,4795–4800,4832–4834,5209; init1323–1326).

### Destruction Crashing Chaos

- Selected Crashing Chaos417234 earns its native417282 reverse uses on actual Summon Infernal1122 execution. The native talent effect2 sets the starting use count; each Chaos Bolt consumes one use while its native direct-damage modifier applies once, not once per use (SimC76f8118 `sc_warlock_init.cpp:839–841`, `sc_warlock_actions.cpp:4401–4403,5080–5082`).

### Destruction Chaotic Inferno

- Each actual Chaos Bolt116858 execution with Chaotic Inferno1244788 selected advances an accumulated PRD from the native talent effect1 chance. A success grants native1244860. Its own cast-time modifier and single charge make the next Incinerate/Infernal Bolt instant and are consumed by that cast (SimC76f8118 `sc_warlock_actions.cpp:4000–4006,4391–4398,5244–5246`, `sc_warlock_init.cpp:830–831,1331–1334`). The pinned bugs-mode rule that keeps Backdraft during that instant Incinerate is not yet qualified.
### Destruction Soul Fire DoT

- Actual Soul Fire6353 execution casts the caster's selected Wither445474 (Wither445465) or Immolate157736 on its target as a free background DoT without direct damage. The DoT starts at execution, before the missile lands, and does not need the missile to hit (SimC76f8118 `sc_warlock_actions.cpp:5102–5138`).

### Destruction Roaring Blaze

- A selected Roaring Blaze1244310 primary Conflagrate17962 hit spreads the caster's existing Wither445474 or Immolate157736 to nearby enemies as a free background DoT, without direct damage. Prioritize targets without the caster's Immolate, as the pinned smart-target selector does even with Wither; the native effect1 count sets the limit, with one extra already-infected target in pinned `bugs=true` mode. Do not spread from Havoc copies or a primary without the caster's DoT (SimC76f8118 `sc_warlock_actions.cpp:4440–4500`, `sc_warlock.cpp:1272–1319`).

### Destruction Incinerate recipients

- Actual Incinerate29722 damages its selected primary once. Without Fire and Brimstone196408 and an owned Havoc80240, it deals no secondary damage. Selected Fire and Brimstone admits every native area secondary at effect0's native25%; it excludes primary and the Havoc-marked recipient. One actual live owned Havoc recipient, even outside the FnB area, receives one impact with Havoc effect0's native50% plus selected FnB25%. Preserve native hit/crit/damage pipeline and snapshot the actual recipient multipliers at cast (SimC76f8118 actions3852–3900,3952–4002).
- Native29722 effect1 [53,16] alone is not a talent gate: EffectAttributes0x20800 are radius flags; there are no SpellAuraRestrictions/SpellTargetRestrictions/conditions rows, and FnB label modifiers966/1886 do not match29722 labels19/16/292. Do not alter general target selectors or catalog data.

### Destruction Incinerate funding

- An ordinary native Incinerate29722 primary impact keeps its existing244670 base grant. Selected Diabolic Embers387173 adds its native effect0 percentage of that base; an actual critical impact adds0.1 displayed shard scaled by the same talent. Use native PowerType raw units and the shared energize/overflow pipeline, without another base grant or resource pool (SimC76f8118 `sc_warlock_actions.cpp3927–3931,4015–4025`). Unselected noncritical hits retain the native base amount. Secondary/triggered copy relationships are separate.

### Destruction periodic shard funding

- Spec267 landed Wither445474 and Immolate157736 ticks grant0.1 displayed Soul Shard through the shared native power/overflow pipeline. A critical tick rolls its native direct spell445468/348 effect1 chance for one additional fragment. Immune ticks do not fund shards; fully absorbed landed ticks remain eligible. Native PowerType metadata supplies raw units (SimC76f8118 `sc_warlock_actions.cpp1108–1117,4047–4059`, `sc_warlock_init.cpp1319,1443`).
- The periodic registry forwards the actual critical result. Other specializations, including Affliction Wither, receive no Destruction funding; Agony's separate threshold RNG is outside this policy.

### Diabolic Ritual, Demonic Art and Ruination

- Selected Demonology Diabolic Ritual starts on in-combat Hand of Gul'dan payment and shortens the held ritual by native effect1 seconds per paid shard (`sc_warlock_actions.cpp278–355`). The first ritual of an encounter is uniform; each ritual's natural end selects the next Overlord→Mother of Chaos→Pit Lord ritual (`sc_warlock_init.cpp927–931,1750`).
- A ritual's natural end grants its native Demonic Art after a gauss(200ms, 15ms) delay, replacing any held Art without summoning (`sc_warlock_init.cpp857–888`). A Hand whose cast started with the Art held, or any Ruination, consumes it. Consumption or expiry in combat summons that Art's native demon (`sc_warlock_init.cpp897–925`).
- The Mother of Chaos summon grants native Infernal Bolt433891 with selected Secrets of the Coven. The Pit Lord summon grants native Ruination433885 with selected Ruination (`sc_warlock_actions.cpp5445–5481`). Each replacement cast consumes one stack of its buff. Ruination's primary impact drops the native three-shard Hand meteor and its three Wild Imps after gauss(395ms, 35ms) (`sc_warlock_actions.cpp5262–5333`).
- Each Diabolist demon executes its native action once: Overlord428571 Wicked Cleave432120, Mother of Chaos428565 channeled Chaos Salvo432569 (each tick launches native432592→432596), Pit Lord434400 channeled Felseeker438973 (`sc_warlock_pets.cpp2574–2896`). They inherit base owner SP1.0, and the Overlord inherits AP1.0. Demonology multiplies their damage by 1 + native428514 effect3. Wicked Cleave applies the owner's Cloven Soul434424 with selected Cloven Souls.
- Infernal Machine429917 shortens the held Ritual once per paid damaging spender while an actual owned Tyrant is active; native429917 effect1 supplies the additional time. Gloom of Nathreza429899 increases Hand's native meteor damage by its effect1 per spent shard (`sc_warlock_actions.cpp302–315,2758–2770`).
- Selected Diabolic Oculi grants Demonic Oculi1269643 on Hand, capped by native stacks. Each unhasted tick emits the stack-specific Diabolic Gaze when Looks That Kill is selected. Consuming Art detonates the held Oculi, grants one Mind's Eyes stack per Oculi when selected, and removes the Oculi (`sc_warlock_actions.cpp370–403,2838–2841,5356–5376`; `sc_warlock_init.cpp940–958`).
- Other ritual spenders and Destruction remain unimplemented.

### Demonology literal execution and alternative Tyrant

- Dreadbite271971 hits only its selected primary unless Dreadlash264078 is selected; a landed Dreadbite triggers the owner's Blighted Maw1276960 using the native1276956 percentage of the actual damage. The Felguard prioritizes its native Felstorm89751 and its89753 ticks when ready (`sc_warlock_pets.cpp536–601,1068–1110`).
- Selected Flametouched453699 applies native Ferocity453704 to each actual Dreadstalker; its owner-sourced native modifiers supply the melee-haste and critical-chance amounts (`sc_warlock_pets.cpp146–162,1223–1226,1269–1286`).

- Actual owned Wild Imps supply native296553 availability while any remain. Ordinary196277 selects the native maximum from the pinned eligible demon pool, launches selected Imps by remaining Energy then spawn order, emits native196278 with Energy scaling only on the primary and retires the actual actors. Selected To Hell and Back derives replacement count from native1281511 and uses native279910/143622 with1250772/1281512, not fabricated actors. Source: pinned `sc_warlock_actions.cpp2980–3170` and `sc_warlock_pets.cpp919–985`.
- During actual combat, the spec266 literal controller maintains its default primary enemy for native guardians and commands its existing configured pet through ordinary pet control. Per-action `target_if` recipients do not replace that default enemy. Precombat does not command attacks, and no owner auto-attack is injected (SimC76f8118 `pet.cpp66–68/143–148`).
- [ ] Spec266 follower parity consumes every exported pinned profile action through shared literal execution, with native target/resource/trait/equipment facts, actual owned Imp/Tyrant counts and source-qualified readiness. Capture precedes warmup; normal default-pet and harmful-precast completion precede pull. Production dungeon kits are unchanged.
- [ ] Actual native265187/250289 binds native Burning Cleave1264093 and repeats on the pinned hasted two-second GCD (SimC76f8118 `sc_warlock_pets.cpp1510–1548`). Both Tyrant variants inherit owner-SP1.0 from the same source policy. Native damage coefficients, target plans and normal cast admission remain authoritative. Tests require actual child impacts and observed formula power, not registry/source-string assertions.
- The actual Antoran Inquisitor casts Mind Sear1280457 for its native duration scaled by owner spell haste, then starts its unhasted channel with an immediate native1280460 tick. It schedules its next hasted execute while the channel continues (`sc_warlock_pets.cpp1835–1880,1978–2002`).

- Each actual Summon Demonic Tyrant265187 execution applies selected Tyrant's Oblation1276767 and Abyssal Dominion456323 to the owner (`sc_warlock_actions.cpp3641–3645`). Native durations and modifiers apply; Abyssal Dominion additionally lasts Reign of Tyranny1276748 effect1 longer, as SimC adds it explicitly (`sc_warlock_init.cpp935–936`).
- [ ] Selected Dominion I starts its native1276166 counter on actual Tyrant completion; ordinary HoG/Ruination advance only an active counter. At its native cumulative limit, choose among the four native summon spells uniformly, redirect living duplicate unique demons as pinned, and retain one stack. Selected Dominion III refunds through actual native1276318, never by editing the pool. Native Aura Does Not Refresh keeps the original expiry while stacks advance (SimC76f8118 `buff.cpp740–743`).
- Expected cooldown remaining time follows pinned `cooldown.cpp603–615`: use actual elapsed time since the native timer started versus its observed progress, including reductions. Ready timers return zero; a timer at its complete duration returns that duration. Queries cannot mutate history.
- Native controlled pets and executing guardians read their owner's self-owned, uncharged passive SpellMods through native family/label matching. Preserve caster-local and frozen modifiers; active or charged owner buffs are not inherited by this path. This follows pinned `player.cpp15268–15324` owner delegation and `sc_warlock_init.cpp120–122` passive registration, not an unqualified owner-buff copy. Native334591/416230/1296573 supply Wild Imp cost, crit and damage values.
- Executing native guardians and controlled demons read live owner critical chance through the same shared `combat::read_pet_owner_critical_chance` helper as native Hunter pet melee. Spell critical-chance calculations reuse that helper; native caster/target modifiers remain separate (SimC76f8118 `pet.cpp217–220`).
- Controlled pets and guardians inherit half the sum of the owner's live attack hit and expertise for melee hit and avoidance; retain the pet's own and the target's native modifiers (`pet.cpp414–420`). Selected Demonic Brutality adds its native effect2 twice to actual demon spell critical damage under the pinned bugs=1 model (`sc_warlock_pets.cpp185–201`).
- The Diabolist Overlord does not receive either Demonic Brutality critical effect, including inherited native spell modifiers (SimC76f8118 `sc_warlock_pets.cpp2584`).
- Controlled pets and guardians inherit half the sum of the owner's live attack hit and expertise for melee hit and avoidance; retain the pet's own and the target's native modifiers (`pet.cpp414–420`). Selected Demonic Brutality adds its native effect2 twice to eligible demon spell critical damage under the pinned bugs=1 model (`sc_warlock_pets.cpp185–201`); Dominion of Argus guardians are excluded (`sc_warlock_pets.cpp1845`).
- The Diabolist Overlord does not receive either Demonic Brutality critical effect, including inherited native spell modifiers (SimC76f8118 `sc_warlock_pets.cpp2584`). Controlled Felguard white criticals inherit owner critical damage and selected Demonic Brutality (`pet.cpp488–494`, `sc_warlock_pets.cpp185–201,663–672`). Its native owner-SP-derived AP uses the pinned zero-damage weapon and AP/6 white base (`weapon.hpp24–32`, `attack.hpp13`, `action.cpp1434–1439`).
- Executing native guardians inherit the live owner's spell haste for cast, GCD and hasted channel/duration clocks, retaining spell-specific caster modifiers (SimC76f8118 `pet.cpp222–226`). Creature/class identity and base spell times remain native.
- Pet and guardian spell damage, healing and ordinary melee use live owner versatility (SimC76f8118 `pet.hpp138–142`). The controlled Felguard inherits owner spell power as its attack and spell power through the existing demon policy (`sc_warlock_pets.cpp663–668`).
- Wild Imp Fel Firebolt hits only its primary unless that Imp has Unstable Soul; its native effect2 supplies extra hits, repeating available recipients when needed (`sc_warlock_pets.cpp713–793`). Selected Infernal Rapidity rolls native1263941 effect1 independently per Imp, launching a free twin scaled by effect2; under bugs=1, a last cast that earns Isolated Implosion does not produce a twin (`pets.cpp713–849`).
- Selected MID2 four-piece rolls native1296574 effect3's accumulated chance on actual Imp exhaustion. A successful Imp travels to its actual target and launches native1306077, using native196278's damage coefficient and native1296574 effects1/2 for primary and secondary damage. Under pinned bugs=1, isolated deaths do not grant Demonic Core (`sc_warlock_pets.cpp833–849/963–983`, `sc_warlock_actions.cpp3200–3290`).
- Selected Inner Demons starts at actual combat entry with a uniformly random first phase, then summons native279910 at the native267216 period without haste (`sc_warlock_init.cpp766–771/1722–1725`). It stops outside combat.
- Selected Carnivorous Stalkers rolls native386194 effect1 on each actual Dreadstalker melee execution and queues a real271971 Dreadbite when the guardian is ready (`sc_warlock_pets.cpp1116–1143`, `sc_warlock_init.cpp1272–1274`).
- On a real Demonic Tyrant summon, selected Reign of Tyranny extends each living owned Dreadstalker by 15 seconds. The pinned bug model also counts living Vilefiends for the new Tyrant’s native Demonic Power and retains stacks when Wild Imps implode (`sc_warlock_actions.cpp3586–3620`, `sc_warlock_pets.cpp953–959`).
- Selected Mark of Shatug casts native455491 Gloom Slash on the attack target after each actual Gloomhound226268 melee and Headbutt267999 execution (`sc_warlock_pets.cpp1361–1373,1405–1418`).
- Selected Inner Demons starts at actual combat entry with a uniformly random first phase, then summons native279910 at the native267216 period without haste (`sc_warlock_init.cpp766–771/1722–1725`). It stops outside combat.
- Actual Wild Imp104317/55659 guardians cannot regenerate spent Energy (pinned `pets.cpp701/875–882`). Native pool sizes, spell costs and actual exhaustion/removal remain authoritative; other guardian regeneration is unchanged. Selected Imp Gang Boss applies native1250772 to the first actual Imp of each Hand of Gul'dan group (`actions.cpp5716–5723`).
- Player-controlled follower minions inherit the same player-affected attack permission as their owner, without changing either NPC identity or native summon-property ownership rules. Ordinary NPC summons remain subject to NPC-versus-neutral restrictions (SimC76f8118 `pet.cpp66–68`).
- [ ] Dreadstalker, Imp Lord and Dominion guardians execute source-bound native actions from their actual actors. Only validated source inheritance is adopted; native coefficients and power costs are unchanged. Fresh Imp Lord initialization and its125-Energy readiness gate follow pinned `pets.cpp1680–1705`; ordinary maximum-power folds retain its source-defined200 base rather than replacing it with generic100. This never resets an existing owner or pet pool. Prepared private data must publish the exact native summon-root templates through the existing importer.

### Shared live Period modifiers

- Current native Period modifiers apply to the interval scheduled after a paid tick, never by retiming the already-pending tick. Preserve the actual per-cast native period inputs, paid ordinals, channel haste snapshots and asynchronous stack deadlines. Final partial ticks have no following schedule (SimC76f8118 `dot.cpp815–821,1109–1170`, `action.cpp4297–4315`). Ordinary DoT live-haste updates remain separate and unqualified.

### Shadow Priest literal parity controller

- Ordinary Shadow Void Torrent263165 starts native459314 Rift at the actual hostile primary. Its hasted native driver clock skips its first travelling tick; subsequent ticks emit native447448 to enemies inside half the native447445 radius. Channel end restores full native remaining driver duration without resetting tick phase. The APL tracking fact reads459314 (SimC76f8118 `sc_priest_shadow.cpp1491–1514`, `sc_priest.cpp1606–1683/3543–3562`).
- With selected448403, ordinary Madness during Rift adds up to five growth stacks. Each stack increases Rift and final burst damage by the native talent effect and increases Rift radius by half a yard under pinned SimC bug behavior. Actual Rift removal emits native448405 once at its stored source primary before removing Voidheart; its native-radius selection splits damage among actual recipients and retains Shadow Weaving (SimC76f8118 `sc_priest.cpp1557–1595/1640–1650/3564–3571`, `sc_priest_shadow.cpp1329–1331`).
- With selected449880, actual Rift applies native449887 as an aura, not a helpful cast. Channel end restores its full native duration without resetting phase; actual Rift expiry removes it, then selected1266856 earns the native Touch tail. Native school/ability bonus values remain unchanged (SimC76f8118 `sc_priest.cpp1683–1685/3540–3541/3571–3576`, `sc_priest_shadow.cpp1491–1501`).
- With selected450405, actual Rift earns native450404, whose native3328092→450983 override and1602 charge-category modifier enable ordinary Void Blast without learned-spell grants. Literal Mind Blast is unavailable during actual Rift/Torrent; Rift expiry removes its override and restores Mind Blast (SimC76f8118 `sc_priest.cpp344–413`).

- Each real Void Blast execute with selected449912 extends the current owned Rift by native effect2 seconds, at most effect1 times per actual Rift instance. Extend an already-earned Voidheart alongside it without resetting pending tick phase; do not carry extension counts into a later Rift (SimC76f8118 `sc_priest.cpp387–394/4358–4391`).
- Selected Screams of the Void375767 makes every successful Madness execute, including Slam's background Madness, apply393919. Reapplication extends by the native fresh duration; native flat-label talent modifiers supply its period amount (SimC76f8118 `sc_priest_shadow.cpp1324–1327/2203–2204`).

- A Madness impact (including Slam's weak Madness, not its periodic ticks) launches a native148859 or1264175 projectile to each own live VT. Native1264104 effect1 owns Void selection/effectiveness; Void launches1264177 Void Bolt at the source primary, never unrelated373279 area Void Spike (`sc_priest_shadow.cpp543–547/2468`). Preserve source-parent identity through travel so only its apparition triggers1231479 Shadeburst, with native73510 soft cap. Native407469 boosts apparition damage on own Madness; these payloads use live Shadow Weaving (SimC76f8118 `sc_priest_shadow.cpp291–549/1296/2609–2650`).
- Selected Haunting Shadows1279355 makes each actual normal413231 or void1264176 apparition impact extend existing owned Pain589 and VT34914 by native effect1 milliseconds, without resetting periodic phase or applying absent DoTs (SimC76f8118 `sc_priest_shadow.cpp427–435`).
- Selected Shadowy Insight375888 makes positive actual Pain589 ticks advance the pinned8% threshold, scaled by selected native Dark Thoughts effect1 and divided by own Pain count^0.9. Uniform initial phase/increments and rollover earn native375981, not manually granted charges or cast-time values. Each actual ordinary Mind Blast/Void Blast execute consumes one live Insight stack; native aura500 preserves its underlying charge while411 owns temporary capacity (`sc_priest.cpp298–316`). Pain impact is not a producer; its Insight roll precedes Tormented's tick roll (SimC76f8118 `sc_priest_shadow.cpp626–638/2052–2084/2322–2339`).
- Selected Tormented Spirits391284 makes positive actual Pain589 hits and ticks advance its own threshold: random initial phase,13% times uniform[0,2) divided by own active Pain count^0.9, doubled for actual crits. Crossing1 earns one ordinary apparition cohort and retains overflow (SimC76f8118 `sc_priest_shadow.cpp598–645/2355–2371`, `proc_rng.cpp221–273`).
- Each real apparition-producing cohort with own VT advances selected373273's native373276 counter once, not once per missile. At the native cap, summon actual373277/189988 Thing From Beyond; selected1264096 also emits the pinned parentless extra cohort, which cannot Shadeburst. Keep the counter capped through that nested cohort before clearing it. Thing retains native creature type15 and casts native373279 using full owner SP, actual owner target DoTs for mastery, native secondary fraction and reduced-AoE cap (SimC76f8118 `sc_priest_shadow.cpp2206–2214/2628–2632/2850–2858`, `sc_priest_pets.cpp75–86/1061–1134`).
- Auspicious Spirits' Insanity threshold and Haunting Shadows' DoT extensions remain separate unqualified relationships; native projectile E1/E2 must not unconditionally substitute for them.

- Selected Psychic Link199484 copies its native effect0 fraction of eligible landed spell/tick damage to every other recipient with the caster's live Vampiric Touch. Native199486 supplies the damage payload; the echo cannot independently crit, hit its original recipient or use foreign VT. Actual caster-self damage, including native Death backlash sharing the offensive spell ID, is not a producer (SimC76f8118 `sc_priest_shadow.cpp1521–1619/2655–2669`, `sc_priest.cpp1430–1446`).

- Voidform228260 grants1242171 uses from native effect2 after its free Volley. Ordinary1242173 consumes earned uses and cannot cast without one. Primary/secondary bolts retain native counts and the pinned50ms interval. Native1296580 grants a capped tier use after Slam in1242171 during Voidform or1279437 outside it. Voidform expiry grants selected Crushing Void and transfers earned tier uses. Literal Voidform readiness is checked before native action aliases can turn it into Volley (SimC76f8118 `sc_priest_shadow.cpp960–1079/1380–1401/1829–1846/1960–2000`).

- [x] Implicit selector unions retain each unit once per effect in first-selection order; overlapping destination-area/line selectors cannot duplicate damage or use target-cap slots twice.

- [x] Spec258 executes the pinned Shadow APL through shared `literal_rotation`, preserving native spell identities, selected traits, resources, equipment and target selection. Missing native follower profiles use the real-player fixture; they never create follower data. Precombat runs once before engagement. No native kit or AssistedCombat driver runs beside the literal controller.
- [x] Native `target.has_absorb` reads live positive school-absorb capacity; an absent, expired or depleted shield is false (SimC76f8118 `player.cpp11813–11814`).
- [x] `SIMC_SHADOW_TENTACLE_SLAM_CHILDREN`:1227280 lands after the pinned500ms scripted travel time while its self-Energize stays immediate. Apply34914 first in1227621's native area, using root effect2's cap and preferring enemies without the caster's VT; damage all selected enemies, then apply335467 at selected1279353 effect0's native percentage without paying Insanity. Persist the multiplier in periodic-leech snapshots and retain native rolling carry (`sc_priest_shadow.cpp1733–1785,1819–1871,1272–1275`). No native records, coefficients or durations are replaced.

- [ ] Shadow Weaving343690 scales eligible Shadow damage by live mastery per owned active589/34914/335467 on the recipient, counting a DoT being applied. Voidform194249 assumes three DoTs. Selected Madness Weaving1240394 adds its native effect0 percentage when Madness is active or during Voidform (`sc_priest.cpp4285–4341`). Native mastery amounts and damage coefficients remain unchanged. Eligibility follows the pinned priest actions, excluding item/racial procs and reflected self-damage.

### Literal precombat execution

- The class-follower fixed-time dummy starts its encounter health timeline and damage accounting after the production APL finishes precombat. Preserve effects that arrive on the combat transition frame (SimC76f8118 `player.cpp:6138–6183`). Tank followers use that same precombat and replay the SimC dummy's incoming attacks and healing during the encounter.
- A recharge reduction for a charged spell does not reduce its separate spell internal cooldown (SimC76f8118 `action.cpp:711–719,2380–2382,2643–2647`).

- [ ] Explicit literal controller handoff occurs before any native setup/warmup frames. Nonliteral fixtures retain native kit behavior; literal fixtures cannot acquire kit-cast precombat auras absent from their APL. Never delete those auras afterward to conceal a late handoff. Accepted Encounter::Fixture capture retires the actor's native FollowOwner while preserving Follower/NPC identity and native literal AA chase; rejected Dungeon capture leaves owner-follow behavior unchanged.
- [ ] Execute precombat once in source order: eligible nonharmful actions and at most the first ready harmful action, following pinned player.cpp6138–6183. Continue to execute eligible nonharmful rows after the first harmful action; skip later harmful actions. It is not a repeating priority scan. Preserve WW snapshot/item behavior.
- [ ] Run native cast completion and GCD progression out of combat before explicit fixture CombatRefs engagement and the measured300s clock; Prowl5215 must meet native OOC readiness. Native-controller fixtures also finish their existing setup cast/GCD before pull. Demo ST must deal first native damage within10s after pull without add-wave or recurring re-engagement. No FULL_MASK, cast-time skips, resource resets or grants.
- [ ] Initialize profile-declared class start pools/default pet before precombat, using native pool units and configured profile values. A new parity Player runs the native class-pool initializer before validating those declared values; the first ordinary frame must not replace the configured pool. Class resource tokens resolve through native PowerType metadata/display units, not a spec-specific reset. Current pinned Warlock profiles declare soul_shards0 and sayaad/felguard/imp. Precasts build resources naturally; summon_pet runs once and no-ops when the requested native pet already exists. Demo's Felguard must exist at pull.
- [ ] Harmful precasts use the shared target-aware Prepared enemy seam, preserving exact selected identities and normal cast permission. Pool_resource uses the GREEN readiness-only requested-pool query, never live resource mutation.

### Literal action readiness and channels

- [x] Ordinary native action/item readiness precedes its `if` expression. Unavailable actions do not bind unreachable item/buff conditions; unsupported operands on an available action remain explicit errors (SimC76f8118 `action.cpp2578–2583,2619–2620`).
- [x] `line_cd` belongs to the exact literal source row, independently of spell cooldowns, targets, and identical neighboring rows. Only a successful execution starts it.
- [x] `action.<alias>.channeling` reads the actual matching native channel. `cancel_action` can cancel during casting/GCD. At native channel ticks, `interrupt_if`/`interrupt` requires a genuinely ready higher-priority action; `interrupt_immediate` bypasses only the GCD guard, and `interrupt_global` selects root versus current-list priority scope. `chain` rechecks the same source row at the penultimate tick and recasts only without a higher-priority ready action. Readiness probes do not mutate powers, casts or recovery clocks (SimC76f8118 `dot.cpp707–735,815–869,980–994`).

- Empower recovery tests must read actual native charge recovery when the spell has a charge category; an expired ordinary spell throttle cannot prove a cooldown reduction.

- Native channel decision ticks use the same modified, hasted interval as the real periodic aura, not the raw DB2 period beside a hasted duration (SimC76f8118 `action.cpp4390–4420`).

- Channel `interrupt_if`/`early_chain_if`, `cancel_if` and `line_cd` expressions read the same current per-driver APL variable state as ordinary row conditions. They must not route `variable.*` operands to native actor facts or create a spec-local variable copy.

- [x] `early_chain_if` works without `chain=1`: after a paid native channel tick, a true expression and ready GCD may re-execute the same ready source row before its penultimate tick, but never ahead of a ready higher-priority action (SimC76f8118 `dot.cpp841–845`).

- [x] Arcane62 executes its exact supplied profile action rows through shared literal control, including source-order precombat and native readiness predicates. Without an official follower profile, the fixture uses a real player; it must not run AssistedCombat concurrently, invent priorities, or change native coefficients/resources.

### Empyrean Legacy relationship

- [ ] `SIMC_RET_EMPYREAN_LEGACY_RELATIONSHIP`: actual Avenging Wrath31884/454351 grants native387178 only for validated selected387170. Actual FV/TV consumes it and schedules zero-delay free background53385 at1+native387170 effect1 percent. Preserve native coefficients, area selectors and flight. Only this background may consume opposite-kind Arbiter without consuming Divine Purpose or its proc charge. Suppress metadata-only every-cast grant/any-hit charge consumption only for this policy's driver/buff; other proc dispatch is unchanged. [Source/proof](../handoffs/ret-paladin-parity-proof-ledger.md#empyrean-legacy-relationship-policy).

- Legacy Storm also rolls selected native Divine Purpose after consuming an opposite Arbiter kind and before Second Sunrise. It can grant fresh Purpose without consuming or classifying existing Purpose; echoes and Tempest cannot inherit this producer (`sc_paladin_retribution.cpp576–602`; `sc_paladin.hpp1665–1726`).

### Proc recipients

- Explicit `tdb_spell_proc` family names and all four family-mask words constrain native spell-event dispatch, replacing generated aura-effect family predicates. Family0 is unrestricted; a nonzero family must match, and a nonzero mask requires any intersecting word. Native197147 Festering Wound must proc from eligible Festering Strike, never unrelated Frost Strike or another melee family. Generated proc definitions retain their existing masks (TC `SpellMgr.cpp`555–560 / `SpellInfo.cpp`1974–1984).

- Triggered native casts resolve their explicit unit identity before planning or launch. A queued proc whose enemy has despawned must reject that cast, not recreate combat references or select a substitute. Native300s Enhancement3T add retirement is the regression boundary (TC `Spell.cpp4243–4255`).

- [ ] Default aura-trigger dispatch uses the actual nullable proc action recipient, never substitutes the aura owner. A child whose native explicit selector requires an enemy cannot target its own caster or a missing recipient; skip it instead of inventing a target. Native implicit-caster buff children remain valid. Named item callbacks use the same recipient predicate before RPPM admission and dispatch. CastSuccess keeps TC's null actionTarget; do not replace it with the spell's unrelated explicit target. Reflection/other explicit triggered casts retain their separate rules. [Ret source/proof ledger](../handoffs/ret-paladin-parity-proof-ledger.md#generic-proc-recipient-self-damage).

### Native periodic spell haste

- Periodic auras and GroundPulses fields with native `SPELL_ATTR5_SPELL_HASTE_AFFECTS_PERIODIC` (Attributes5 `0x2000`) apply caster spell haste after the native period spell modifiers, truncating to integer milliseconds. Their unchanged duration admits the resulting extra ticks. Unflagged auras retain their native period. Source: TC a352b1fa `SpellAuraEffects.cpp999–1012`.

### Devastation literal rotation

- Spec1467 follower measurements execute all89 preserved MID1 `76f8118` APL rows through shared `literal_rotation`; retain the named missing-MID2 reference gap and complete exported allocations.
- Scarlet Adaptation stores the selected native percentage of Evoker healing before critical and partial-tick scaling, applies pinned SimC's 50% modeled overheal and Living Flame spell-power cap, and consumes it on the next Living Flame, adding the stored amount to each damage recipient (`sc_evoker.cpp`4347–4362/5195–5201/6930–6934).
- `empower_to` selects the real native release stage while retaining the selected hostile primary independently of self casts. Fire Breath dot facts observe native357209 owned by that actor. Disintegrate uses the shared channel decision seam, including action-bound `ticks_remain`.

### Gaze of the Alnseer

Equipped1256896 can grant Alnsight1266686; real eligible hits while it is active earn native1266687 stat stacks using the driver's equipped-item-scaled effect0. When Alnsight refreshes during its native750ms proc cooldown, the first hit after that deadline starts a new cooldown, the following hit bypasses it once, and that new deadline then resumes (`unique_gear_midnight.cpp`2020–2098).

### Measurement

- [ ] Native measurements execute an independent300s sample for every comma-separated `DPS_PARITY_SEEDS` value, default `70448,1,2`. `report.seed` lists the actual seeds, `seed_dps` retains each native DPS, and `combat.dps` is their arithmetic mean. Invalid seed lists or a failed sample fail the measurement; no sample is silently omitted.
- [ ] `combat.actions` averages actual per-spell damage, hits and cast counts over all samples, including zero contribution when an action is absent. Shares must preserve mean per-spell DPS for unchanged `dps_parity_gaps.py` consumers. Full reports remain in `<spec>.native.<count>.seed-<seed>.json`; `detail_seed` identifies first-seed buff, entity and other unaveraged details.

- Origin-centered parity arenas provide explicit open collision/LOS services for native destination selection. They have no dungeon walls or baked-eye requirement, matching pinned `action.cpp1738–1752` target selection. This fixture mode must not change production map LOS or bypass native summon/spell metadata.

- Venomous Wounds returns native Energy after a successful owned Garrote/Rupture tick on an owned poisoned target. Its native refund is divided by the same bleed's active poisoned-target count raised to the native diminishing exponent. Keep fractional gains until a whole Energy is earned. Resolve before the producing aura expires; do not count other Rogues' dots or roll the deferred proc again.

- Earned Instant Poison315584 coating procs dispatch their native effect0 child315585 to the actual hostile proc recipient for every Rogue specialization. Native chances, modifiers and damage remain authoritative (SimC76f8118 `sc_rogue.cpp3021–3025`).

- Earned Deadly Poison coating procs apply their native effect0 DoT to the actual proc recipient. A repeat while that caster's poison is still ticking also deals the sourced instant poison hit. Other casters' poisons do not enable that instant. Native proc admission/chance, damage and durations remain authoritative.

- A successful harmful Player ability carrying the native melee-combat-start flag starts weapon swings at its retained hostile primary. An existing weapon victim and swing timers remain unchanged. Fixture engagement does not inject the missing attack; no ability whitelist or damage multiplier substitutes for the native flag.

- Spec259 player fixtures execute the profile's literal pinned Assassination action rows through the shared APL evaluator, not AssistedCombat or follower-kit priorities. `apply_poison` uses the selected native coating casts in source order and waits for each real cast before the next precombat row. Production follower decisions remain unchanged. See [Rogue mechanic requirements](rogue-parity.md).

- Parity opponents face away from the actual actor at initial spawn, engagement after warmup, and add replacement, matching the reference's `position=back`. Preserve pack positions and native attack geometry/defenses. Rear position excludes parry, not NPC dodge. Player and class-parity actors also receive the pinned SimC baseline7.5% expertise in both weapon-spell and ordinary-swing dodge/parry calculations (`player.cpp1622`). Their controlled pets and guardians inherit the owner's hit/expertise mean in both ordinary swings and weapon-spell dodge rolls (`pet.hpp103–104`, `pet.cpp414–420`). Native target defense values and ordinary NPC expertise remain unchanged.

- [x] Auto-attacks remain eligible during native channels with `SPELL_ATTR5_ALLOW_ACTIONS_DURING_CHANNEL`. Normal casts and other channels block swings without discarding attack intent or timer progress. Preserve crowd-control blocking, victim avoidance and spell-native permissions; no class/ability whitelist.
- [ ] Native aura342 modifies BASE/OFF/RANGED attack timers only, using its signed multiplicative attack-time factor. It must not change spell haste, cast time, GCD or resource regeneration. Preserve melee-only319 and the native scope of other haste types; no spell-ID exceptions. All current342 users are inventoried in the [source/proof ledger](../handoffs/aura342-attack-timers-proof-ledger.md).
- [x] Native harmful spells retain selected enemies independently of effect-hit outcomes: every non-EVADE miss (including immunity) creates the caster's launch combat reference, while the target engages only at native projectile arrival. Missed effects still deal no damage/apply no hit effects; evade targets create no reference (TC a352b1fa Spell.cpp8538/2763).

Three-target references omit the fight-style preset: pinned SimC `sim.cpp2318` clears explicit raid events for Patchwerk after parsing. The opt-in CLI integration test must observe three actual targets and an adds event; argument-string checks alone did not detect this failure.

- [ ] A headless300s follower dummy harness consumes exact profile gear through the production stat path and supported talents/consumables, on one target and a specified three-target DungeonSlice-like setup. New DPS measurements use a private SQLite .backup of CURRENT canonical world.db and record canonical/backup SHA; old frozen databases remain historical reproductions only.
- [ ] The same test-only measurement harness supports real player characters without requiring a follower profile, using exact level90 reference item instances and pinned active traits. Existing native AssistedCombat70448 authored order/predicates drive player casts; unavailable native facts or unsupported rules fail explicitly. No invented priorities, stat changes or production mechanics edits are permitted.
- [x] The shared three-target parity fixture places the primary and both adds inside a60° frontal arc for every class, including replacement waves. Preserve the primary's existing distance, actual target identities and native spell cones/radii. A three-target reference assumes all three are geometrically eligible; never widen gameplay cones to compensate for fixture placement. This is a harness correction, not a gameplay parity fix.
- [ ] All specified fixture opponents begin each initial/replacement wave with active mutual CombatRefs to the fixture player/controller, using the ordinary combat-reference API. Passive31146 keeps its native no-ThreatTable template and passivity; follower fresh-pack/AOE_HITS_ONLY_IN_COMBAT_UNITS policy is unchanged. Native73/63/103 must actually cast an authored min_enemies≥2 AoE entry in the three-target fixture.
- [ ] Harness-only `SIMC_FIXED_TIME_TARGET_HEALTH` keeps the main target on the global linear100→0% fixed300s clock. Recurring adds use their own30s remaining lifetime for health percentage and per-target TTD, with30.01s starts and nine actual10ms absent intervals across300s (pinned76f8118 `sc_enemy.cpp1053–1101`, `raid_event.cpp2700–2701`). Every wave restarts its phase; main remains a boss and ENEMY_ADDs do not, using explicit fixture metadata without mutating31146 templates. Production training-dummy damage remains inert. Report actual native clock/lifetimes/health, preserve historical receipts and remeasure all3T parity owners after landing; never fabricate timing or change references.
- [ ] Reports compare primary/crit/haste/mastery/versatility/AP/SP first, then action counts/average hit/damage share and buff uptimes. Each ability separately counts actual misses, dodges and parries against measured targets, including abilities with no successful damage. Native receipts include actual damage/hits per measurement target, retaining retired and zero-output target identities, so multiple-target output is not inferred from spawn count. Flag every delta>5% and classify stats/gear, coefficients, missing proc/aura or rotation/decision.
- [x] Receipts expose measured actor and actual controlled-pet raw current/max pools and last native regeneration rates at existing state samples, plus before/after actor pools and final pet pools. Source/recipient/spell-attributed Energize receipts retain logged integer amount, overflow, effective gain and count. They never infer power type from ambiguous effects or label snapshots/logged grants as integrated realized regeneration; capped gains remain distinguishable from requested amounts.
- [x] SimC action normalization includes direct and periodic amounts without double-counting parent compound totals.
- [x] Stat comparison accepts identically-unitized attribute/stat maps, flags strictly greater than5% differences, distinguishes absent and zero-reference measurements, and rejects nonfinite inputs.
- [ ] Compare SimC70391 and native70448 effect rows before attributing a coefficient mismatch to a server bug. Rotation/decision edits remain lead-owned.
- [x] `SIMC_PARITY_ALLOCATIONS` applies to every explicit parity actor, preserving exact selected reference allocations. Pinned SimC76f8118 `player.cpp2696–2860` decodes node selections without parent-edge or currency-gate validation; the test-only native allocation builder skips those two checks. Pinned `player.cpp2775–2779` also exempts allocated HERO-tree nodes from specialization gates, including native98931/entry122279 Mass Eruption on spec1467. Only hero availability/visibility/rank-allowed spec predicates are exempt; granted-rank amounts and all other predicates remain native. Subtract granted ranks from the original exported totals under that same fixture policy, so newly eligible grants cannot duplicate an allocated rank. Node/entry membership, rank, non-hero specialization, level, choice, selected subtree and total currency budget checks remain. Production `build_config` remains strict. Spec251 uses a real player because no native follower profile exists; its former `SIMC_PROFILE_NOT_NATIVE_LEGAL` scoring exclusion is retired. Do not drop selected nodes, invent follower rows or edit official profiles/exports.
- [ ] Cover every non-initial native70448 player specialization (40 rows with ClassID1–13/OrderIndex<4), not merely the first follower cohort66/73/105/257/262/63/253/1467/269/266/103. Current pinned sources have29 baseMID2 profiles and11 native follower-harness profiles; absent harness profiles do not imply absent server class support. Missing shipped healer references remain explicit HPS/BiS gaps, never synthetic profiles. Report first-three and all-spec summaries to the lead.

Final30s sustained-damage qualification compares cumulative damage to every measured target identity, including retired waves. Engagement samples still inspect only current live opponents; add retirement must not subtract earlier damage and produce a false no-output failure.

### Per-spell oracle diagnostics

- [x] A Python3 stdlib CLI accepts a pinned profile path/spec name, repeated action names, target count and duration; removes original action lists and runs deterministic pinned SimC through `agent-run simc-oracle` with `TMPDIR=/var/tmp/osso`.
- [x] Inherit each source reference proof's simulation options, including `optimal_raid=0` and all other non-default flags. Only explicit short-probe controls/report destinations differ; missing or conflicting profile-proof matches fail, never silently enable default raid buffs.
- [x] Retain generated profile, debug log and proof containing exact command, binary/profile/source-proof hashes, inherited options, explicit probe overrides and revision. Parse printed direct/tick formula factors, result stages, child/proc spell IDs and source/effect-attributed passive versus buff-stack modifiers; preserve unsupported lines and mark unprinted factors null.
- [x] Emit per-spell coefficients, base/weapon terms, itemised passive products, crit/versatility/target factors and explicitly printed armor reduction. Never invent parent-child causality or count duplicate result stages as casts.
- [x] Source-backed prerequisite plans may use bounded longer durations and actual enabling casts/earned stacks. Explicit repeated precombat actions replace the stripped setup and are recorded separately in proof; no invented buff values or silent reference-option changes.
- [x] Select top-five reference damage channels per spec, retain channel shares/provenance and explicit untriggered-channel gaps; diagnostic probes do not certify native±10% parity. Native cast instrumentation remains owned by spec parity harnesses.

### Native per-spell differential harness

- [x] FoF child impacts record observed synchronous periodic-dummy dispatch identity (aura/effect/instance/trigger), without guessing from spell ID/timestamp or changing casts. Unscoped/delayed origins remain null; the diff preserves observed provenance independently of oracle action-name selection.

- [x] A shared test-side helper reuses reference-profile preparation, native31146 and literal fixture readiness/dispatch for exactly one requested cast or N observed damage ticks; no forked rotation or synthetic follower profile. First native fixture is follower269 Tiger Palm, then Arms12294 after shared Player dispatch supports its real actor.
- [x] The shared helper accepts distinct cast and observed damage spell IDs. An existing real fixture may be primed through shared native execution before capture; requested hit/tick counts exclude earlier priming events, without granting resources or inventing actor profiles.
- [x] Capture actual calculation-stage inputs/outputs without rerolling variance/crit or recomputing a second formula. JSON retains base, AP/SP terms, applied multipliers, crit, versatility and armor reduction in oracle-compatible per-hit events. Armor records expose actual post-penetration armor, selected K and the calculation's mitigation scalar; any rounded damage-stage ratio is separately named. Missing attribution is explicitly `unattributed product`, not a guessed aura source.
- [x] Native fixtures preserve reference gear/talents and exclude raid buffs. Explicit priming casts may earn required resources/state; no production damage tuning belongs to this diagnostic. Separately approved follower mechanics stay in the [follower contract](follower-dungeons.md#player-equivalent-class-combat-policy).
- [x] A stdlib Python CLI diffs matched formula/result/armor factors, retains both result states and provenance, and distinguishes actual differences from unprinted factors. Identical recorded factors and concrete counterfactual deltas have unittest coverage. Explicit native/oracle action-name selectors distinguish real same-ID tier/proc/regular hits; impact ordinals are within the selected series and absent requested names fail explicitly.

## How it works

- [Character stats and gear](../wiki/systems/character-stats.md#gear).
- [Inventory](../wiki/systems/inventory.md).

## Implementation inventory

- `scripts/item_instance_data.py` — native-table producer for a supplied existing SQLite copy; canonical application is lead-owned.
- `scripts/dps_parity.py` — explicit current-tier/prior-tier profile extraction, allocated-trait JSON validation and SimC action/stat normalization.
- `scripts/simc_selected_traits.cpp` — diagnostic CLI wrapper around unchanged76f8118 engine objects; not the reference DPS executable. It replaces only the CLI entry object to inspect actual `player_traits` after one0.1s metadata iteration. Retire when the official CLI emits lossless allocated-trait metadata.
- `crates/server/src/hunter_pet_stats.rs` — native Hunter HP/armor inheritance and the shared integer22% total-owner-RAP source used by melee and abilities.
- `crates/server-simulation/src/dungeon_simulator/dps_parity/` — opt-in ignored300s native follower fixture; reuses dungeon runtime and single-follower initialization, real31146 dummy, existing diagnostic-run ownership (so normal orphan cleanup retains the follower), exact instance gear, strict native trait validation and actual combat/buff receipts. Set `DPS_PARITY_OUTPUT`, `DPS_PARITY_WORLD_DB`, `DPS_PARITY_SPEC`, `DPS_PARITY_TARGETS=1|3`. Unmodeled race0/consumables/item options and existing kit-vs-APL remain named measurement gaps; invalid traits fail instead of resetting the loadout. Initial execution exposed the noncombat owner's first-tick attack cancellation: six specs never engaged, and73 dealt only two first-tick hits. These are fixture diagnostics, not qualified class DPS. Qualification requires actual damage in the final30s as well as a300s clock, preventing a first-tick-only hit from passing. Pet specs253/266 must execute learned casts and actual damage from their sourced pet. The parity App installs the ordinary HunterPet/PetControl systems and explicitly loads pet catalog/level-up metadata from its read-only prepared world snapshot. Missing native Duck174884 content is supplied by the existing class-summon producer, with stable-key `scripts/dps_parity_duck_174884.sql` and its receipt; no world spawn or fabricated creature statistics are added. Canonical application remains lead-owned; three-target qualification requires10 timed waves with20 distinct add identities and one persistent main dummy. The fixture reuses the existing native Player-owner initializer, places that quiet owner outside melee range and requires zero owner damage; the measured follower remains an NPC. No owner health/flags/timers are fabricated and no gameplay AI decisions change. Native8163 armor loading is separately RED→GREEN qualified;1467/103 strict trait rejections remain named gaps.
- `crates/server/src/item_instance.rs` — pure instance-level stat budget resolution, consumed by production equipment Stats.
- `crates/server/src/item_instance/{bonuses,effects,load,store}.rs` — native bonus application/SQLite loading and additive GUID-keyed redb state. Type23 preserves ItemEffect IDs; base/bonus resolution retains native spell IDs, trigger types and cooldown sentinels. Equip-proc execution is not yet integrated.
- `crates/server/src/stats.rs` — production stat consumer of slot-bound instance properties.
- `crates/server/src/{inventory,inventory_model,persistence_inventory,item_stats,persistence}.rs` — owned GUID hydration/projection, initial/delta delivery, startup catalogs and additive redb initialization.

Parity updates native `Health.current` before each50ms combat tick using `SIMC_FIXED_TIME_TARGET_HEALTH`, and applies the same global time immediately to newly replaced adds. The fixture removes only its dummy actors' native health-regeneration timer because source31146 has `RegenHealth=1`; otherwise ordinary two-second heals would change execute predicates between schedule updates. Native template flags, `Health.max`, damage logging and the production dummy script remain unchanged. Reports record the policy and actual health samples; qualification and action-count comparison remain execution-gated.

Native fixture execution uses `scripts/dps_parity_native.py <compiled-test-binary> [test args...]` under `agent-run`, with `DPS_PARITY_WORLD_DB` set. Linux bubblewrap supplies a per-process read-only bind of that snapshot onto the resolved absolute startup database path; both `WORLD_DB_PATH` and the parity-specific path therefore read the same file without changing host data. The helper rejects missing snapshots/non-test executables and never substitutes unsandboxed execution. The native input regression checks both startup file identities; the opt-in Python process fixture checks frozen data visibility, denied writes and unchanged host/snapshot bytes. Ordinary production loaders remain unchanged.

Measure a spec with one command, under `agent-run` and capacity admission:

```sh
python3 scripts/dps_parity_measure.py --spec 70 --world-db <private world.db> --output <dir with 70.profile.json, 70.selected-traits-active.json> [--targets 1,3] [--actor follower|player]
```

It builds the `server-simulation` test executable with Cargo's `test` profile (server crates opt-level 2, debug assertions and overflow checks kept). This is the same artifact that ordinary `build-host.py --test -p server-simulation` runs use, so one rebuild serves both. It then runs every target count in parallel, one process each, through `dps_parity_native.py`, and writes `<spec>.native.<count>.{json,log}` plus `<spec>.native.timing.json`. Do not use the old `--profile dev` harness: the server crate is opt-level 0 there. Measured on Ret 70 follower (2026-10-25, `17e60833`): dev harness ST 108 s + 3T 120 s sequential, test profile in parallel 18-19 s total. Each run spends about 10 s on setup (catalog loading) and 6-8 s in the 300 s tick loop. After a one-line core edit that shifts lines, the test-profile rebuild takes 139 s; the dev rebuild took 36 s. Reports are identical across profiles except `wall_seconds`, `timestamp_unix_ms` (wall clock) and the Debug order of `combat_refs` (a std `HashMap`). These fields also differ between two runs of one executable.

The test-only attacker seam also accepts `DPS_PARITY_ACTOR=player`: native specialization metadata selects class identity, the prepared database resolves the reference race, and ordinary player power/stat/passive systems initialize the actor. Human uses the pinned SimC `util::race_id(RACE_HUMAN)=1` and requires that exact native row:70448 also labels ThinHuman33 "Human", so a display-name match must not choose it or reject a valid Human1 profile. Plain `dracthyr` uses pinned SimC `util.cpp503–504`'s Horde identity, native70 (`util.cpp2152–2153`), not the same-named Alliance52. The same race binding applies to player and follower reference actors. Other absent/ambiguous identities fail explicitly. Strict reference traits use that player's actual race/spec identity. No follower profile or persistent character is fabricated. Player rotation execution uses the test-only `game-engine-core` dependency pinned to9ef2efe2d317b837d352fe8c5227086a4fdcd490, reusing `CooldownAssistCatalog::recommend` and `assisted_combat::evaluate_rule` unchanged. `DPS_PARITY_ASSISTED_DB2` must name the frozen70448 seven-table CSV directory. Native known spells/overrides, powers/runes, auras, target health, pet state, cooldown/charge clocks and ordinary target admission supply facts; the latter is explicitly labelled as the range-fact boundary rather than claiming identical client geometry. The50ms driver submits normal `cast_for_player` requests only inside the existing native queue window. An unknown potentially applicable rule stops the attempt and writes exact facts/rule IDs/native event evidence to `<spec>.player.assisted-gap.json`; missing metadata or cast refusal remains an explicit error. No alternate priority is substituted. Parity authority is the pinned literal SimC APL through shared `literal_rotation` and the owning spec adapter. Native AssistedCombat/one-button execution is diagnostic only, not a substitute parity rotation. AssistedCombatRule conditions11/12 remain named unverified numeric/predictive gaps; do not guess their semantics or derive a parity DPS band from a stopped diagnostic attempt. Arms adapter/FIRST/target route is owned by14775 and requires its exact qualified fixture/tips.

### Incoming caster damage scopes

Native aura271 effects match both their source spell family and their effect's SpellClassMask. Burning Core1299975 affects Fire Nova only; Sentinel's Mark1253601 applies its Aimed and Wildfire bonuses independently. Unrelated spells and other families receive neither bonus. Lashing Flames retains its native Flame Shock mask. The documented Colossus Smash208086 exception continues to grant owner damage through the common Warrior target multiplier. Sources: pinned SimC `sc_shaman.cpp7010–7028`, `sc_hunter.cpp1278,1493–1494`, and `sc_warrior.cpp1723–1734`.

Ret Truth's Wake403695 effect3 uses its native Consecration/Divine Hammer mask, not a universal owner-damage bonus. Its separate537 effects keep labels2511/2587, but its own periodic damage does not receive its label2511 Burn to Ash bonus (SimC76f8118 `sc_paladin.hpp1288–1295`); Judgment197277 keeps its native spender mask. Expurgation383346's270 Holy mask matches overlapping schools, including Radiant6. Native paired casts verify unchanged FV under Truth's Wake, the real Judgment bonus and Holy Flames on mixed-school damage; removing the former blanket271 amplifier must not be reversed to recover a DPS band.

### Shared armor classification

Only a pure Physical school mask uses target armor. Hybrid schools, including Doom's Physical/Nature and Havoc's Chaos children, retain their native school masks without Physical armor reduction. Native bleed mechanics and explicit spell bypasses remain unchanged (SimC76f8118 `action.cpp1686–1690`).

### Enhancement fixture driver

Lava Lash receives its native effect2 damage bonus only while Enhancement's offhand Flametongue imbue is active (`sc_shaman.cpp`5160–5220).

Actual Tempest grants selected Storm Swell455089 and one native Arc Discharge470532 stack. Ordinary spell events must not grant Storm Swell for263. Each eligible Chain/Ride chain's final actual recipient consumes one Arc use when its native400ms ICD permits and becomes the delayed325–425ms extra chain's primary. Extra chains may self-trigger outside that ICD; normal Arc benefits from live MW without spending it, while Ride Arc neither benefits nor spends. Arc uses do not multiply its native modifier amounts (effect attributes64). Preserve native masks, values, caps and duration, and Elemental262's separate policy. Sources: pinned SimC76f8118 `sc_shaman.cpp6587–6597,6631–6643,6709–6714,9994–10007,12437–12484`.

Selected Overcharge1251026 increases Nature critical damage using its native effect2 and the owner's live attack or spell critical chance (`sc_shaman.cpp`13798–13820).

Enhancement's selected Enhanced Imbues462796 applies only its second native effect (`sc_shaman.cpp`11455–11459).

Selected384355 Elemental Weapons supplies408390's native masked direct/periodic modifiers at talent effect0 ×0.1 per actually imbued hand. Real imbue gains/removals update the bonus; zero imbues or an unlearned talent remove it. Stormblast retains its explicit sourced exception without a second generic application. Sources: pinned SimC76f8118 `sc_shaman.cpp13212–13220`; native70448 supplies values and masks.

Doom activation grants the selected Feral Spirit and Rolling Thunder Crackling Surge stacks with independent native8s and12s lifetimes (`sc_shaman.cpp`8845–8856,11600–11613). Lightning wolf actor100820 remains unmodeled; the class-summon importer includes its authentic template through native summon198506, but existing world databases require regeneration.

Selected1262635 Thunder Capacitor rolls its native effect1 chance after each actual paid normal/Thorim LB, CL or Tempest execution and restores exactly the MW stacks consumed by that execution, through the existing native counter/modifier producer. Do not refund zero spends, Arc Discharge or Ride chains; do not alter the already native damage modifier. Spent-stack accounting uses the admitted native per-cast limit, not a net counter delta after refund. Sources: pinned SimC76f8118 `sc_shaman.cpp6687–6694,7462–7469,9978–9985`.

Actual Windstrike weapon children115357/115360 ignore target armor. Ascendance replaces main- and offhand white swings with native Windlash114089/114093, which also ignore armor. The offhand uses its own weapon and the native half-damage factor. Native Doom469270 retains its Physical/Nature school but does not use Physical armor; ordinary Physical Stormstrike children remain armor-sensitive (SimC76f8118 `sc_shaman.cpp4726,4734–4768`, `action.cpp1686–1689`).
Selected Stormwell1264762 runs a guaranteed equipped-native Windfury callback once on each landed primary Crash187874 impact, not once per AoE recipient. Preserve its native internal cooldown, payload and Unruly Winds behavior; ordinary imbue proc chances remain unchanged (SimC76f8118 `sc_shaman.cpp6141–6145`).

Native Unleashed1252431 remains Magic and cannot trigger weapon imbues. A main-hand weapon pointer and `shaman_attack_t` inheritance do not override SimC's data-Melee admission check (`sc_shaman.cpp3039–3055`, pinned SpellDataDump1252431).

Converging Storms grants one native198300 stack per landed Crash Lightning target. Stormstrike and Windstrike consume the stacks after both weapon hits (`sc_shaman.cpp`5591–5604,6094–6100).

Enhancement Tempest applies its native effect2 damage percentage to secondary targets, without reducing the primary (`sc_shaman.cpp`9875–9881; `action.cpp`1553–1555).

Spec263 parity executes the pinned MID2 Enhancement action rows through the shared literal evaluator, using native action admission, equipment and aura/target facts. A real player uses this driver without loading AssistedCombat diagnostic inputs; no follower profile is fabricated. Windfury/Flametongue setup occurs only in precombat. Thorim's LB/CL choice follows successful foreground LB/CL/Tempest dispatch and Tempest target count. This selects actions; it does not certify the underlying Enhancement proc, counter or damage mechanics.

Enhancement native melee procs generate the real344179 counter and187881 modifier together. Foreground affected spells snapshot their native modifiers at launch and consume up to the live187880 effect1 limit; expiry removes the modifier. Voltaic Blaze grants its native effect1 stack count once per execution. Windfury/Flametongue setup resolves the native weapon-enchant payload for the equipped hand; eligible landed melee runs native proc chance/modifiers and25504/10444 damage, without recursive Windfury procs. The third attack uses native390288 Unruly Winds, not384355 Elemental Weapons. Sources: pinned SimC `sc_shaman.cpp2640–2675,3039–3055,10123–10125,10919–11023,11870–11988`.

Native Enhancement strike roots dispatch one195592 Crash payload per actual landed root while their Crash buff is active, not one per weapon child. Selected1262713 uses native async1252415 rather than187878; payload stacks multiply its total damage before native-recipient splitting. Thorim invokes the actual last foreground LB/CL choice (initially LB), overridden by an earned Tempest proc, and spends its real MW modifiers on that background cast. Windstrike always invokes; Stormstrike requires Doom Winds; Crash requires Doom Winds or Ascendance. Foreground Tempest requires actual454015 and consumes that proc. Sources: pinned SimC `sc_shaman.cpp4536–4598,5597–5689,6102–6105,10041–10049,12554–12600,12758–12763`.

Native Enhancement Static Accumulation384437 ticks generate real MW stacks at their native amount/period. Overlapping Ascendance and Doom Winds links retain that same producer until the last matching caster-owned parent ends; the first parent's expiry must not stop its real ticks. Shared linked-aura removal reads native parent metadata and child-caster identity rather than spell-ID exceptions (SimC `sc_shaman.cpp12840–12860`). Selected Elemental Assault210853 grants its modified native effect1 stack count on ordinary strike/Lava Lash roots, including misses. Each actual consumed MW stack reduces running Strike/Lava Lash recharge by Elemental Tempo1250364 effect2 milliseconds. Descending Skies1264688 admits only actual Ascendance casts; Awakening Storms455129 admits only Strike roots, preserving native proc chance/RPPM and payload. Sources: pinned SimC `sc_shaman.cpp5225,5605–5610,10280–10283,11780–11787,12023–12039,12249–12261,12838–12847`.

Selected Enhancement Conductive Energy455123 uses the completed native hit frame for LB, CL, Ride the Lightning and Tempest. Existing caster-owned living Rods receive one accumulated197568 event per resolution, scaled by native210689 effect1; the native child cannot crit or reapply caster modifiers. Application of197209 is deferred by SimC's10–100ms: LB/Tempest retain their actual primary, CL chooses its first un-Rodded native chain target (otherwise primary). The creating cast cannot mirror into its own newly applied Rod. Native chain recipients include neutral attackable enemies, not only hostile faction reactions. Aura amounts on application/refresh use that same native coefficient.

Selected1252373 makes a landed native Crash Lightning root schedule native1252431 Unleashed weapon strikes at one-second intervals. Native1252373 effect1 supplies the pulse count. Each pulse reevaluates the native Crash target list and uses the pinned90% target-occupancy roll, not a damage coefficient adjustment. Each execution snapshots and consumes its own native1299991 Short Circuit stack value; overlapping executions retain independent pulse state. Sources: pinned SimC `sc_shaman.cpp1490,4519–4532,6110–6135`.

Actual466772 Doom Winds emits native469270 area damage on application and its native periodic effect2 ticks. It requires the caster's real living selected target; a missing/dead selected identity suppresses the pulse rather than selecting a substitute. Native destination/area geometry and damage metadata remain authoritative. Selected114051 also draws an independent three-success600-card deck per actual MW consumed outside its active aura, with greater-than10-card success spacing within and across refills. Spending while Ascendance is active neither draws nor resets the deck. A success triggers background384352, including its native466772/Static producer, without activating Ascendance or starting the ordinary Doom button cooldown. Sources: pinned SimC `sc_shaman.cpp11809–11827,13099–13110`.

Actual Strike weapon children32175/32176/115357/115360 roll native201845 Stormsurge with its native chance/mastery modifiers. A proc grants201846, restores all actual Strike charges, and grants470466 when319930 Stormblast is selected. Native319930's generated every-cast grant is suppressed. An ordinary Strike snapshots previously earned470466 before consuming one Stormsurge/Stormblast stack; both real weapon children emit390287 from their resolved damage times native319930 effect0. The copy cannot crit or reapply broad caster modifiers, but retains the pinned explicit live77223 mastery and384355 Elemental Weapons multipliers plus native target modifiers. Sources: pinned SimC `sc_shaman.cpp4449–4469,4702–4711,5597–5600,12125–12147,13184–13187,13212–13220`.

Selected344357 Stormflurry rolls its native effect0 chance after each actual Strike execution, including its own background repeats. A success schedules the same native Strike root after a Gaussian200±25ms delay, retaining its selected target and prior Stormblast snapshot. The repeat does not press another foreground button or spend ordinary charges/GCD/resources. Native effect1 scales its weapon children, Crash payload and Ride chain once; Stormblast copies the already-scaled weapon result. Repeats retain real weapon procs but do not grant normal-root Elemental Assault or invoke Thorim. Sources: pinned SimC `sc_shaman.cpp4545–4561,4682–4689,5540–5545,5597–5608,5647,5685–5690,6610–6613,12041–12073`.

Selected Ride the Lightning289874 emits native211094 once per actual Strike/Lava Lash execution, using its native chain targets. This background variant neither benefits from nor spends MW. Real Tempest casts spend one454015 stack, preserving the other earned proc; Thorim spends its current Tempest proc before its MW consumption can earn a new one.

Actual Enhancement MW consumption draws Tempest's pinned two-success100-card deck and selected1262713's five-success250-card Storm Unleashed deck once per consumed stack, preserving independent greater-than10-card success spacing within and across decks. At most one native proc per deck is granted per spell. Neither substitutes a Bernoulli roll or the talent's unused displayed chance. Native1262830 grants its extra Crash charge through411 and exempts only category2458 consumption through500. One landed Crash consumes one proc after Thorim; its free cast never appends or delays the normal recharge. Other charge categories still consume normally. Sources: pinned SimC `sc_shaman.cpp6107,11830–11845,12888–12904,13057–13098`; TC `SpellHistory.cpp843–844`.

Selected454391 Unlimited Power grants one native454394 stack per actual MW-spending execution, not per spent stack. Native amount, cap and duration stay authoritative. Further stack gains preserve the first application’s expiry; after expiry, a new spend starts a fresh one-stack epoch. Sources: pinned SimC `sc_shaman.cpp11766–11770,12655–12658`.

Selected1260666 makes actual Voltaic Blaze executions roll their native effect1 chance for Fire Nova. Each living attackable target with this caster's native188389 Flame Shock emits one333977 explosion, independently of Burning Core periodic ticks. Sources: pinned SimC `sc_shaman.cpp10126–10130,7043–7076`.

Equipped native MID2 Enhancement two-piece1296627 makes real Voltaic Blaze apply1299975 Burning Core to its selected primary. Its native periodic effect dispatches333977 Fire Nova from the original caster; its caster-specific native damage modifier remains authoritative. Each Fire Nova explosion execution with equipped four-piece1296628 reduces running Crash Lightning recharge by its native effect0 milliseconds and grants1299991 Short Circuit once, not per area target.

### Shared literal snapshot operands

Bare `time_to_die` reads the actor's fixed-time remaining fight duration. It does not inherit a `target_if` candidate's lifetime; explicit `target.time_to_die` retains that candidate's actual fixture lifetime (SimC76f8118 `action.cpp4276`, `player.cpp7731–7754,11866–11895`).

Bare `refreshable`/`remains` retain the action and recipient context. Qualified Feral periodic effects store their actual persistent factor at aura application: `pmultiplier` reads that field (absent dot0), while `persistent_multiplier` reads the same fresh native producers. Refresh replaces the stored factor; later buff loss cannot rewrite it. Native fresh-dot metadata with a combo-point cost uses the ordinary pre-payment effective-point snapshot without admission or payment, including when the minimum cast cost is currently unaffordable. Do not infer snapshots from damage or substitute live factors for missing stored fields (`action.cpp3940–3997`, `dot.cpp568–573`, `sc_druid.cpp4673–4676`).

`target.distance` reads the actual native actor-to-selected-target spatial distance, including action-scored target overrides and later position changes. Missing native positions fail explicitly; movement-event distance remains a separate configured-event fact.

`action.<alias>.damage` reads the proposed target's native direct school-damage snapshot, including current power, modifiers, mitigation and expected crit, without casting or sampling RNG. Weapon-damage predictions remain explicit errors. Action cooldown/duration and bare `ticking` retain action context. Target-owned `dot.<alias>.duration` is0 when inactive; an active DoT reads its native fresh composite duration, excluding remaining time and pandemic carry (SimC76f8118 `dot.cpp448–455`). `pet.<summon alias>.active` reads actual living, unexpired, same-map owned summons; absent summons are inactive.

Named `action.<alias>.charges`, `max_charges`, `charges_fractional` and `full_recharge_time` read the same native cooldown state as bare action-context operands. Named `.usable` reads cooldown readiness only, not resource/GCD admission (SimC76f8118 `action.cpp3269–3270,3496–3499`).
## Survival literal controller

- Spec255 uses the exact pinned `hunter_survival.simc` action rows through shared `Program`, `Driver`, `Prepared` and `Equipment` with the requested native actor identity. It does not require an invented follower kit.
- Precombat provisions the existing native default-duck stable policy and executes ordinary Call Pet, snapshot and the selected harmful precast before engagement. Native pet control supplies independent pet attacks. Shared Hunter facts supply equipment, potion and Howl observations; shared pet mechanics and live owner critical chance remain authoritative.
- Native70448 supplies stats, damage, costs, cooldowns and selected traits. Controller coverage alone does not qualify ST/3T DPS parity; missing native relationships remain measurement-gated.

### Survival native cast relationships

- Actual Kill Command grants one Tip of the Spear use plus selected Primal Surge's native extra uses. Uses do not multiply the per-use damage modifier; an affected action consumes exactly one at primary impact. Source: `sc_hunter.cpp1359–1362,1407–1408,6606–6608`.
- Ordinary Raptor Strike rolls the selected native Swipe rank's chance. An actual upgraded Raptor Swipe consumes its replacement and cannot produce itself. Source: `sc_hunter.cpp6188,6217–6236`.
- A real Wildfire Bomb explodes around its actual primary using pinned5yd geometry, native reduced-AoE cap and native primary-only bonus. Apply the corresponding periodic payload to every actual impact recipient with the same cap/primary effectiveness. Selected Shrapnel replaces, never duplicates, the ordinary fire DoT. Keep native damage coefficients and timing. Source: `sc_hunter.cpp7030,7039–7043,7062–7068,7083–7089,7118–7120,7134`.

- Each actual Kill Command with selected Wildfire Infusion460198 shortens Wildfire Bomb's running recovery by native effect1 milliseconds through the shared cooldown reduction. It never starts a recovery. Source: `sc_hunter.cpp6622,7776`.
- Each actual Wildfire explosion execute with selected Shrapnel Bomb triggers native1292687 once, not once per recipient. Its native periodic ticks grant owner Focus. Source: `sc_hunter.cpp7092–7095,8200–8205`.
- Each actual Boomstick periodic tick executes its native child first, then selected Mongoose Rounds1253945 grants the native number of Mongoose Fury259388 stacks once per tick, not once per recipient. Retain the native periodic clock. Source: `sc_hunter.cpp6333–6341`.
- Selected Shellshock1252931 modifies actual Boomstick1261215 hit damage by its native effect1, reduced by the execution recipient count times native effect2 when that count exceeds one, clamped at zero. Use native child targeting, not encounter-wide enemy count. Source: `sc_hunter.cpp6268–6281`.
- Actual Boomstick channel execution spends one earned Tip, grants native471536 through the channel, and reuses the native Stargazer/selected Sentinel mark/main-pet Strike consumers. Remove the channel buff on root-aura removal. Source: `sc_hunter.cpp6304–6324,6344–6350`.
- Actual Takedown grants selected Twin Fangs1272139's native Tip count before its buff and damage; owner Focus comes once from native1258571, not the pet child. Source: `sc_hunter.cpp3096–3105,6368–6405`.
- Actual Takedown executes native player1253859 and living owned main-pet1253862 against its hostile primary after the Takedown buff and shared force mark. The pet payload's native energize effect does not execute; use its damage effect and actual pet stats/crit. Source: `sc_hunter.cpp3096–3105,6359–6405`.
- Selected Sanctified Armaments1253831 banks its native second effect from each actual Raptor Strike/Swipe impact into the owned native1253836 rolling periodic on that recipient. Reuse the shared MM residual engine while preserving MM Rapid's distinct first effect. Source: `sc_hunter.cpp5708–5711,6140–6167`.
- Actual Moonlight Chakram1264949 spends one earned Tip, triggers selected Stargazer and the main-pet Strike consumer, and grants native1280140 until its final scheduled bounce. Do not roll Sentinel's Mark. 1280140 modifies bounce damage only through its native label5870 modifier. Source: `sc_hunter.cpp4410–4416,4456–4476,4490–4499`.
- With selected Twilight Requiem1264904, the final scheduled Chakram bounce executes native1266096 once around the actual Chakram primary, using native area targeting. The native reduced-AoE cap (effect1, sqrt above8 recipients) is not modelled. It does not affect encounters with eight or fewer enemies. Source: `sc_hunter.cpp4427–4434,4504–4507`.
- Selected Raptor Swipe2 applies its native second effect only to the actual hostile primary. Read the current native modified effect value, not raw35%; secondaries and base damage/soft-cap remain unchanged. Source: `sc_hunter.cpp6195–6196`.
- Selected Spirit Bond263135 includes459722's native percentage through the existing aura/mastery engine. Its full native modifier application follows the owning mastery aura's apply/remove lifecycle; do not replace CharacterStats or add a parallel mastery/damage calculation. Source: `sc_hunter.cpp1392–1395,2586–2589,7900`.
- Actual Raptor Strike/Swipe base executions grant native Mongoose Fury259388, including its full selected effect set, native stack cap and refresh. Do not require the named talent to create the buff. Actual primary Tip consumption grants selected Stargazer1253750 before its queued mark/ordinary pet strike; retain independent native stack expiry. Native aura values supply all damage effects; do not add manual damage factors or stat changes. Source: `sc_hunter.cpp1358–1361,6149–6155,7749,7880,8160–8165,8303–8305`.
- Actual primary Tip consumption dispatches native Strike as One1251779 from the living owned main pet and starts the existing native1251717 ICD only after execution. Actual tipped rank3 Swipe dispatches its separate strike before base execution with the rank's native second-effect effectiveness; that extra does not spend the ordinary ICD. Use the actual aura instance for the shared proc timer; raw melee hints must not spend its owned ICD. Selected Sic 'Em1253137 executes native1253138 from the same main pet only on its actual Strike as One crit. Reuse shared pet stats/crit, periodic clocks and native payloads, not a second pet engine. Source: `sc_hunter.cpp1367–1372,2875–2898,3381–3387,6181–6186,7944`.
- Real primary impacts that consume native Tip uses queue Sentinel's distinct native Survival chance after the impact batch, so tipped Wildfire can earn a fresh mark after consuming an old one. Native Sentinel already receives Moon's Blessing through its5585 label; do not add it twice. Selected Lunar Calling adds its Survival effect during actual Takedown. MM's distinct native chance remains unchanged. Source: `sc_hunter.cpp1358–1366,3694–3709`.
- Active Sentinel's selected Lunar Calling makes actual Takedown force an owned mark on its hostile primary. A real Wildfire explosion consumes only the caster's own marks and schedules native Lunar Storm payloads on those recipients. Shared mark grants reduce running Aimed/Wildfire recovery through the distinct native Moon's Blessing effects, without starting inactive recovery or resetting charges. Preserve existing MM producers and native values. Source: `sc_hunter.cpp3665–3667,3709–3710,3760–3773,6400–6401,7104–7107`.

## Marksmanship literal controller

- [x] Spec254 uses a real Player with exact reference gear/traits and unchanged pinned `hunter_marksmanship.simc` source through shared `literal_rotation`. It does not require a nonexistent follower profile or use AssistedCombat priorities.
- [x] Execute source precombat, preserve native cooldowns/overrides and item admission, and start ranged attacks through native75. Report the literal driver and native stat snapshot. Missing potion inventory and external providers remain explicit gaps, not fabricated effects.
- Sentinel's Mark `.react` clocks belong to each actual recipient. Configure the exact pinned254 reference player reaction settings before acquisition for initial targets and replacement waves; never infer acquisition at query time. Keep the literal action rows unchanged.
- [ ] Qualify remaining residual damage and both full-duration DPS references before claiming parity. Controller execution alone does not qualify native mechanics.

### Marksmanship native cast relationships

- [x] Active Sentinel Moonlight Chakram is shared by Hunter specializations: only actual Trueshot or Takedown enables it. A Chakram cast consumes that enabler and delivers the existing native bounce count, delay and selected ramp to actual targets. Unrelated harmful casts cannot produce the enabler. Gate shared relationships by active hero talent, not specialization lists.
- [x] Actual Aimed Shot and selected No Scope Rapid Fire produce native Precise Shots. Real spenders consume it, preserve its damage through projectile arrival and trigger selected Stargazer.
- [x] Selected Unstable Trigger permits exactly one native-duration, Focus-free Explosive Shot recast inside its existing cooldown; that recast does not create another window.
- [x] Volley delivers its native-duration periodic payload at the selected ground point to every actual in-radius target; all magnitudes remain native-data driven.
- Selected Stalk and Strike grants native Lock and Load on Chakram. Rapid Fire fills native Bulletstorm uses and Aimed Shot spends one. The native70448 Bulletstorm maximum is authoritative, even when different from the reference build.
- Volley grants Trick Shots for its field duration. Aimed Shot and Rapid Fire ticks hit actual nearby secondary targets with native Trick Shots limits and secondary percentages. Outside Volley, Aimed Shot consumes one use at launch and Rapid Fire at channel completion.
- Master Marksman banks actual critical Hunter damage into native269576 with its native period, duration and rolling unpaid carry. Selected Blood Fletching supplies the Aimed Shot share; neither producer invents a coefficient.
- `hunter_cadence_tests` exercises the four native cast, stack, critical-damage and real-recipient boundaries.
- Each executed Rapid Fire tick applies selected Take Aim's native Aimed recharge reduction. Actual Auto Shots roll selected Lock and Load's native chance. Each foreground Aimed Shot rolls selected Surging Shots' native reset chance; a success resets Rapid Fire and grants selected Focus Fire until the next Rapid Fire channel ends (`sc_hunter.cpp5489–5493,5553–5560,5715–5722,5832–5837`).
- Selected Feathered Frenzy Trueshot force-produces an owned Sentinel's Mark at the selected hostile primary. Actual Precise spenders roll the native mark chance; Aimed impact consumes a mark into Lunar Storm's native count and payload at the pinned150ms spacing. A mark grants Moon's Blessing's native Aimed recharge reduction.
- Actual Rapid Fire damage banks Sanctified Armaments with native residual values and carry. An Aimed critical impact applies Blood Fletching's native mark after1ms; later direct/periodic damage triggers its native bleed payload without recursive self-triggers or armor mitigation.
- `hunter_followup_tests` covers real recharge, repeating Auto Shots, forced mark consumption and both native damage producers.
- Each actual Explosive Shot primary tick feeds native212680 secondary-only cleave at the root's native excluded-share percentage and reduced-AoE cap. The equipped MID2 four-piece reduces running Aimed and Rapid recovery by its native effect2 milliseconds once per primary tick, never per cleave recipient.
- A target-carried Rapid Fire channel consumes its caster's earned Trick Shots on natural completion outside actual Volley; the channel's recipient is not its owner. `hunter_explosive_tests` covers actual cleave recipients, both real recharge clocks and this ownership boundary.
- Selected Salvo makes a real Volley pulse launch background Explosive Shot at its native-capped actual recipients, once per native Volley-duration internal cooldown. These applications retain native periodic damage and rolling carry without paying foreground Focus, starting its ordinary cooldown or granting Unstable Trigger. `hunter_salvo_tests` checks recipient cap, unchanged expiry across later pulses and actual periodic damage.

## Tests asserting this spec

- `scripts/tests/test_item_instance_data.py` — concrete signed rows, idempotence, unrelated-table retention and duplicate-ID rollback.
- `crates/server-simulation/src/dungeon_simulator/dps_parity/enhancement_operand_tests.rs` — native direct-damage/crit snapshots, racial metadata, selected-target ticking and owned summon lifetime/life filtering.
- `scripts/tests/test_dps_parity.py` — exact item identities/modifiers, reference-tier selection, missing-ID rejection, selected-talent metadata, direct/periodic action accounting and finite/missing/zero-reference stat comparisons.
- `crates/server/src/item_instance.rs::tests` — two passing item-level budget/weapon-damage and invalid-level regressions.
- `crates/server/src/item_instance/{bonuses,store}.rs::tests` — six native allocation/configuration/socket/crafted-stat and durable-reopen/owner-transfer/rollback regressions. Owned inventory hydration/projection is additionally proved at the persistence/Stats boundary.
- `crates/server/src/item_instance/load.rs::tests` — native SQLite configuration and enchant/gem join regressions; both passed within the 15-case library proof.
- `crates/server/src/spell_cast/linked_aura_tests.rs` — native zero/nonzero BP0, caster ownership, stacks/removal and trigger-depth controls. Nonzero parent10/8 overrides child-1/-1000 rather than rejecting the link.
- `crates/server/src/equipment_effects.rs::tests` — actual base/bonus/enchant equip-aura lifecycle and item-level replacement; enchant8039 records the resolved scaled parent/child amount, not just its zero DB2 base points.
- `crates/server/src/stats_instance_tests.rs` — actual StatsPlugin modifier-only change on native chest 271468/enchant 7987; genuine RED missed +50 Strength before consumer wiring; subsequent native GREEN and unchanged legacy Arcane control passed.

- `crates/server-simulation/src/dungeon_simulator/dps_parity/power_tests.rs` — opt-in native253/Duck/Cobra/grant owner/pet snapshots and grouped capped gains, plus native66 authoritative Mana serialization. Genuine receipt RED2 precedes implementation; current14e20fbca GREEN3/3 (110.93s), retained six source/parallel controls, all-target check/fmt pass. Snapshot rates/logged integer gains do not certify integrated realized regeneration or resource mechanics parity.

## Shared profile gear effects

- Reference followers retain NPC identity but use the declared native race for base stats and validated racial spell knowledge. Native player/pet aura effect202 must apply owner racial effects.
- Selected native Inspired Intellect458437 enhances an active Arcane Intellect1459 on that Mage by adding its native effect1 percentage points to the raid buff's amount, preserving other multipliers (`sc_mage.cpp6832–6841`). Do not apply a global intellect correction or an independent2% factor; actors without either aura are unchanged. The shared source gate covers actual Fire63, Frost64 and Affliction265 actor identities.

- Aura189 rating buffs must change derived native ratings through their real masks. Item spell scaling class−7 includes its native trinket combat-rating multiplier before rounding. Declared Magisters flask, Void-Touched augmentation, Royal Roast, Recklessness potion and Thalassian Phoenix Oil quality2 use native item/enchant/spell values. Oil coexists with permanent enchants. Potion selects the current highest positive/lowest negative secondary and is available once per encounter; unsupported tokens remain named gaps.
- Expansion tokens resolve native TraitNodeEntry identities, independent of class talents. Item scripts supply SimC relationships for Heart Morsels/stacking stats, asynchronous Venomfang/expiry bursts and perfected Guillotine poison, plus Omnium fire/lingering/secondary runes. Guillotine's RPPM starts without a preloaded attempt or bad-luck window and adds at most one capped interval per eligible attempt (`proc_rng.cpp95–114`). Native proc rates, scaling and recipients remain authoritative; neutral attackable targets must not be rejected as non-hostile factions.

[Implementation](../wiki/systems/character-stats.md#shared-profile-effects--2026-10-25).

## Known gaps (current cycle)

- [ ] Remaining profile bonus options, equip-spell aura/proc lifecycle and complete instance-movement qualification; basic Stats derivation and additive persistence have targeted GREEN proof.
- [ ] Real300s server dummy execution and equal-gear parity reports.
- [ ] Shipped105/257 healer BiS/HPS references are absent upstream.

## Elemental Shaman fixture (262)

- Both Player and follower measurements use the exact profile action rows through the shared literal SimC evaluator, native readiness and native spell execution. Player measurements must not also run AssistedCombat; the production follower kit is unchanged.
- Precombat rows complete native casts and GCDs before engagement. Ancestral Swiftness replaces Nature's Swiftness when its talent is active (SimC76f8118 `sc_shaman.cpp`8132–8137).
- Per-ability native output includes actual critical-hit count and damage, plus separate critical and noncritical mean damage. Zero critical hits produce a zero mean, not inferred criticals.
- Score 262 ST against `262.reference.single.json` and 3T against `262.reference-fixed.three.json`. The original `262.reference.three.json` has the known Patchwerk add-event error and is not a three-target reference.

Only successful ordinary Lava Burst51505 grants MOTE through native16166 effect0's trigger. Its broad generated HIT proc is suppressed so Lightning Bolt cannot immediately re-grant the consumed buff (`sc_shaman.cpp`7245–7247).

The Elemental MOTE policy uses the resolved native260734 effect0 amount, not an invented spell modifier or charge. It snapshots eligible cast damage, consumes the earned stack after a successful foreground launch (also triggered Flame Shock), and retains the multiplier through missile flight, periodic damage and ground fields. Eligible overloads snapshot MOTE at parent scheduling and retain it after consumption; later MOTE gains do not alter that snapshot. Background overloads do not consume it (SimC76f8118 `sc_shaman.cpp`3249–3252,3297–3300,3316–3351,3399–3408).

Elemental Blast117014 grants one native118522/173183/173184 buff after its cast; overload120588 grants it before execution. Choose uniformly among inactive buffs until all three are active, then refresh one. Native aura values and durations stay unchanged (`sc_shaman.cpp`7545–7577,7615–7620,7700–7704).

Flame Shock's literal action uses learned native470411, whose costed trigger casts188389. The DoT fact retains188389; do not grant knowledge of its unlearned payload or bypass readiness.

Elemental Overload callbacks are instant background actions that can execute during a foreground hardcast without replacing its cast state. Schedule the400ms callback before parent projectile travel, not400ms after impact (`sc_shaman.cpp`3316–3351). Selected Feedback Loop's third node independently rolls its native chance for one extra overload after each earned overload, including Earthquake fields; that extra cannot recurse (`sc_shaman.cpp`3393–3417). Native child damage, resource costs and aura/target checks remain authoritative; effect160 ordinary forced casts remain unchanged (SimC76f8118 `sc_shaman.cpp`4846–4847).

When Lightning Bolt Overload45284 executes while Stormkeeper is active, it snapshots Stormkeeper's native effect1 damage bonus despite the unmatched native class mask. The bonus survives projectile flight. Chain Lightning Overload retains native mask behavior (`sc_shaman.cpp`3254–3257,6315–6322,7320–7327).

Stormkeeper grants its native effect4 count of next-spell uses, bounded by the native stack cap. Retain the native single proc charge; do not represent the same uses again as extra charges. Each use carries one native modifier amount, not a stack-multiplied bonus. A successful foreground root spends one use; its background overload does not spend another (`sc_shaman.cpp`8759).

Primal Storm Elemental dismissal removes the owner's earned Wind Gust263806, including early dismissal; its native infinite aura must not outlive the actual summon (`sc_shaman.cpp4330–4334`).

Native445624 supplies actual221177 Ancestors and their native lifetime after selected Stormkeeper or Ancestral Swiftness. Successful foreground spells command each living owned Ancestor once with native447419/447425 children at the retained primary. Background overloads do not command them. Native owner spell power and crit are inherited; Ancestor Lava Burst is guaranteed critical and scales with inherited crit (`sc_shaman.cpp`4342–4441,6358,7856,8163–8166,8758). Natural expiry allows selected Final Calling465717; cancellation does not. Actual Ancestor447419/447425/465717 critical hits apply selected live Elemental Fury60188 effect0's critical bonus, preserving its native amount and other spells' modifier masks (`sc_shaman.cpp`11468–11469; SimC effect indexes are one-based). Removing that owner aura removes the bonus. Ancient Fellowship and remaining pet effect inheritance still require qualification.

Selected Routine Communication443445 draws once after an ordinary successful Lightning Bolt, Chain Lightning or Lava Burst execution. The native effect4/effect5 counts supply successes/total for a shuffled deck; reshuffle only after all cards are drawn. A successful draw summons the real native445624 Ancestor and447244 owner buff. Background builders, overloads and extra Chain recipients do not draw (`sc_shaman.cpp`6654–6658,7260–7263,7436–7438,13025–13033; `proc_rng.cpp`149–161).

While Ascendance1219480 is active, native Elemental Overload damage uses its effect7 bonus (`sc_shaman.cpp`4884–4891).

Elemental Lava Burst Overload285466 scales its damage by one plus the caster's current critical-strike chance (`sc_shaman.cpp`6835–6855).

Selected Mountains Will Fall creates the native298762 Earthquake overload field on a mastery success and adds one during Ascendance. The field uses native298765 damage, native mastery and talent effectiveness, and its source's persistent damage multiplier; Ascendance modifies its damage while active (`sc_shaman.cpp`3175–3180,3316–3351,3379–3414,7897–7915,7953–7968).

Selected Ancient Fellowship443423 draws its native shuffled deck once per natural Ancestor expiry. A successful draw summons a real replacement (`sc_shaman.cpp`4437–4439,13016–13023).

Selected Offering from Beyond443451 reduces the running Stormkeeper cooldown by its native effect0 duration for each actual Ancestor summon (`sc_shaman.cpp`11579–11596).

Selected Power of the Maelstrom191861 rolls its native chance on ordinary Lightning Bolt and Chain Lightning executions. Eligible foreground spells spend one native191877 use and, with Fusion of Elements462840, execute a free instant Elemental Blast at the native Fusion effectiveness. Its overload uses the Fusion effectiveness independently of other damage modifiers (`sc_shaman.cpp`2582,3303–3311,6660–6663,7439–7444,7599–7609,7653–7674).

With Primal Elementalist117013, every Ascendance execution with Call of Fire378255 summons the real native118291/61029 Primal Fire Elemental, and Stormkeeper with Fury of the Storms191717 summons native157319/77942 Primal Storm. When that elemental is already active, its lifetime and the owner buff are extended by the native summon duration and its action cooldowns reset; no second actor is created. Owned guardians receive template class mana. The actors follow the pinned APL order: Meteor, Immolate only when not ticking on the target, then Fire Blast; and Stormfury only with at least10s of Call Lightning, then Call Lightning and Wind Gust. Owner spell power is inherited at1.0 times one plus the native117013 effect0. Stormfury269005 compounds its native157375 effect1 damage bonus once per scheduled channel tick, with the same factor for all recipients (`sc_shaman.cpp`4026–4042,4070–4139,4160–4184,4248–4315,8638–8640,8752–8755,11555–11570).

Equipped MID2 four-piece1296626 grants native Flowing Elements1300219 uses when Stormkeeper or Ascendance ends, buffering the overlapping grant until Ascendance ends. Eligible foreground builders spend one use and earn native Overcharge1300222, which makes the next eligible spender free. Lightning Bolt Overload45284 and Lava Burst Overload285466 snapshot Flowing's native effect0 damage bonus when they execute while Flowing is active, despite their native class masks (`sc_shaman.cpp`3265–3267,6795,7323–7324,12600–12608,12636–12651,12914–12924).

## Out of scope

Rotation/decision implementation belongs to the lead/rotation agents. No system install, native-table re-export, canonical data mutation or protocol change without demonstrated need. Reference SimC is the official local CLI-only source build at76f8118cf3f7f41dad20708e8328e1f4ab2deebc (embedded70391); server inputs remain70448. Durable evidence lives in `game-engine/data/diagnostics/dps-parity-2026-10-21/`.

### Affliction265 literal fixture

- Spec265 uses the exact exported76f8118 Affliction action rows through shared Program/Driver/Prepared/Equipment. Native player identity, spell knowledge, costs, clocks, target selection and70448 values remain authoritative.
- Precombat summons the profile-selected pet and executes its first eligible harmful precast. Darkglare activity and remaining lifetime come from real owned summons; Malefic Grasp requires that actual pet. Channel chaining and interruptions use shared native channel decisions.
- The fixture does not grant shards, buffs or reference damage. Numeric ST/3T parity remains separately measurement-gated.

### Affliction native Shared Agony

- Selected1259825 makes a successful Agony root cast one real native980 background twin on a different living enemy inside its authored radius. Randomize ties, then prefer the shortest remaining owned Agony; foreign applications do not count. The twin pays no second cost and cannot recursively create twins (`sc_warlock_actions.cpp1419–1450`, `sc_warlock.cpp1274–1320`). Native application, stacks and periodic shard funding remain unchanged.

### Affliction native Agony funding

- Only landed Agony980 ticks from Affliction265 advance its owner's threshold-RNG shard progress, with uniform initial state and rollover. Pinned increment0.370 scales by active owned Agonies to the power−2/3 and native Creeping Death timing normalization, including the pinned rank2 bug. Losing the last owned Agony resets progress.
- Each threshold crossing pays native17941 effect0 through ordinary energize handling (one displayed Soul Shard); no fragment-per-tick Destruction policy applies to Affliction.

### Affliction Malefic Grasp sourced action

- `simc_apl:` selected Malefic Grasp1261149 resolves to native channel1261153 under the same sourced known-parent projection as Demoniac. Native70448 has no visible-action link. Unknown/unselected talents do not grant the channel; costs and actual channel damage stay native.

- Projection qualification retains the Demonology owner's gate: `demonology_ --ignored --skip demonology_implosion_`. The excluded Implosion test is the owner's existing master RED, not a Malefic Grasp regression (same exclusion as trainsdd4e681ed7/d577e0ad22). Native Affliction cast and Frost literal opener remain required.

### Affliction native Death's Embrace

- Selected234876 uses its native maximum amount and health threshold to scale source-qualified damage linearly with each actual recipient's live health below that threshold (`sc_warlock_actions.cpp529–535`). Preserve pinned exclusions: direct Corruption/Wither, Shadowbolt Volley and Malefic Grasp itself are not affected; Grasp's extra DoT payloads are affected. While any actual owned seed-applied UA stack remains in the native async scheduler, ordinary UA does not receive Embrace (pinned bugs=1).
- Selected234876 uses its native maximum amount and health threshold to scale source-qualified damage linearly with each actual recipient's live health below that threshold (`sc_warlock_actions.cpp529–535`). Preserve pinned exclusions: direct Corruption/Wither, Shadowbolt Volley and Malefic Grasp itself are not affected; Grasp's extra DoT payloads are affected. Seed-applied UA stack suppression belongs with MID2 Seed UA integration.
### Affliction native Nathreza recipients

- Actual1262710 retains native chain count/range/amplitude but only admits its actual Haunt primary or secondaries with this caster's ticking146739/445474. The undotted primary remains eligible; foreign DoTs and losing the actual owned DoT do not admit secondaries (`sc_warlock_actions.cpp2532–2546`). Haunt's one-owned-recipient replacement is already native-qualified and needs no separate implementation.

### Affliction native Darkglare Eye Beam

- Ordinary native205231 from an actual103673 Darkglare reads its real owner and actual recipient. Each distinct owned active Agony, Corruption, Wither, Seed or UA contributes the native205180 effect2 amount; multiple UA stacks count once and foreign DoTs do not count (`sc_warlock_pets.cpp2445–2455`, `sc_warlock.cpp248–264`). Keep native guardian rows and coefficients unchanged. Nether Plating's alternate AoE chain is not selected by this MID2 fixture and is separately unqualified.

### Affliction native MID2 Seed UA

- Selected1296569 makes successful Seed execution apply one real1259790 to the same smart primary selected by the actual Seed plan, before the projectile/explosion (`sc_warlock_actions.cpp1799–1813/1903–1910`). It pays no second resource cost and secondary Seed recipients do not earn UA. An instant Seed and its tier UA spend exactly one admitted native Instability use; a use earned during a hardcast remains available (`sc_warlock_actions.cpp1913–1924`).
- Seed applications retain visible native stacks but contribute the native set effect0 fraction to periodic UA damage; ordinary/Fatal Echoes applications remain full strength. Track provenance against the actual stored asynchronous stack deadlines, never a second expiry scheduler. Real stack loss ends its weighting and Embrace suppression; UA Grasp children deliberately retain the visible stack count (`sc_warlock.cpp140–172`, `sc_warlock_actions.cpp1652–1664/2212`).

### Affliction native Darkglare Presence

- Native1280663 direct/periodic masks and amounts apply only while an actual owned living103673 Darkglare remains unexpired. Actual death or expiry ends the increase; foreign summons and unmasked damage do not qualify (`sc_warlock_pets.cpp2487–2500`, `sc_warlock_actions.cpp200`). No independent buff timer or guardian-row change supplies this effect.

### Affliction native Unstable Empowerment

- Selected MID2 four-piece1296569 earns native1305774 from actual owned UA1259790 applications. Count UA stacks globally across active living targets, not DoT instances; ignore foreign casts and cap at the native buff maximum (`sc_warlock_actions.cpp1567–1568/5673–5699`).
- Each independent UA expiration and full removal updates the earned stack count; no remaining owned UA means no buff. Fatal Echoes updates it through its actual new application. Keep native player, pet and guardian damage effects and amounts; do not add haste or change damage coefficients (`actions5779–5780`, native1305774).
### Affliction Nightfall uses

- Native264571 stacks are independent next-spell uses, not a multiplier of its native effect amounts. One ordinary instant Shadow Bolt consumes one admitted use. A proc earned during a hardcast remains for the next action; queued cast/proc callbacks cannot consume it or spend the next stored use (`sc_warlock_actions.cpp877–878`).
- The admitted instant Shadow Bolt retains native264571 effect1's damage bonus through missile impact after consumption. Hardcasts cannot borrow a later proc's bonus. Preserve native coefficients and the pinned Hellcaller Grasp exception; this bonus applies only to Shadow Bolt (`actions902–908/2147–2152`).
- Drain Soul/Grasp consume one use on execution. Two uses retain the same native duration and tick-period reduction as one use; they cannot collapse the channel (`actions199/2151–2163/2182/2299–2311/2338`).

### Affliction earned Shard Instability

- Selected native1260264 rolls its native effect1 chance on landed Shadow Bolt and effect0 chance on actual Drain Soul/Malefic Grasp ticks, using independent accumulated-probability streams for Bolt and the shared channels. Native1260269 owns stacks, lifetime and spell modifiers.
- Actual Unstable Affliction consumes one earned Instability use only if its cast admitted that native cast-time modifier. An effect gained during a hardcast is retained.
### Live item proc facts

- Per-trinket `proc.any_dps.up` and `remains` observe only the actor-owned native stat-buff identities reachable from that item's effects. A multi-buff proxy returns the maximum live value, a different item cannot borrow the aura, and expiry returns zero.

- Native cooldown `max_charges` reads the effective capacity before and after charge consumption. The bare action operand reads that same capacity, not the remaining uses.

### Unholy literal fixture

- Bare `health.pct` and `is_boss` retain the actor context inside target scoring. Bare `duration` reads the proposed action's captured native periodic state (zero when absent), not its cooldown or hypothetical fresh duration.
- Spec252 runs the exact exported76f8118 Unholy action rows through shared Program/Driver/Prepared/Equipment with native player identity. Bind rune/Runic Power, disease and pet state to native70448 values; ordered precombat executes native Raise Dead. Numerical parity is separately measurement-gated.


### Affliction native Cunning Cruelty and Ravenous Afflictions

- Selected453172 shares one Cunning PRD stream across landed Shadow Bolt686 impacts and Grasp1261153/Drain Soul198590 ticks. Pinned SB-selected chance is0.50, DS-selected0.25 (`sc_warlock.hpp1066–1067`, init1232–1235). Success casts native453176 at the actual recipient; its native coefficient, missile and five-target limit remain authoritative. The talent's native value5 names that enemy count, not the proc chance.
- Selected459440 attempts its native RPPM only on actual critical Agony980, Corruption146739, Wither445474, UA1259790 ticks or critical Grasp extra children1261166/1261176/1261158/1279686. Success applies real264571 Nightfall. Generic/noncritical events cannot also advance this owned stream; talent removal stops acquisition (`actions459–467/2004–2025`, init1227).
- Preserve the pinned Grasp tick order: Instability, Cunning, extra DoT children. The existing primary-tick dispatch owns that order; no shared damage dispatcher changes are required. Grasp chooses only the selected Corruption/Wither child and reads actual Corruption146739, not direct172 (`actions2192–2214/2117–2131`).

### Ordinary native spell GCD haste

Category133 ordinary spells apply native spell haste before the independent aura417 haste-regeneration path. Preserve melee/ranged defense and ability/ranged-slot exclusions, native750–1500ms haste bounds, and exclude other base GCDs from this haste path. TC a352b1fa `Spell.cpp9209–9224` and SimC76f8118 `action.cpp648–666,1285–1315` define this behavior. The native clock gate covers representative foreground actions for all40 specs at0%,25% and150% haste; aura417 retains its separate native controls.
### Affliction native target talents

- Selected386976 Withering Bolt multiplies native Shadow Bolt686, Shadowbolt Volley453176, Grasp1261153 and Drain Soul198590 by its native percentage per distinct owned Agony980, Corruption146739, Wither445474, Seed27243 and UA1259790, up to its native effect1 cap. UA stacks count as one DoT; foreign/expired auras do not count (`sc_warlock.cpp248–264`, actions817–818/915–916/2223–2224/2368–2369).
- Selected1312998 Impetuous Wrath uses native effects0/1 for Shadow Bolt, Grasp and Drain Soul, or2/3 for Dark Harvest1257065, choosing the enhanced effect only for the caster's own active Haunt. It does not affect Volley. Removing the passive removes its factor (`actions918–919/2226–2227/2371–2372/2452–2453`). Native coefficients and other modifier stages remain unchanged.

### Affliction native Darkglare inheritance

- Native205180/103673 Darkglare inherits full live owner spell power through the existing guardian policy, as base `warlock_pet_t` specifies and Darkglare retains (`sc_warlock_pets.cpp19/2424–2430`). Native205231 supplies damage coefficients and cast time; other guardian policy entries are unchanged.
- Existing native guardian tests retain real cast timing, dynamic owner-SP changes, retargeting, expiry and persistent-pet preservation. A native265 cast must spawn103673 and land205231 with the full owner-SP term in its actual damage calculation.

### Affliction native Shadow of Nathreza

- With selected native1261984, actual Haunt48181 effect4 pulses launch native1262710 at that Haunt's recipient. Use its native one-second clock without an initial pulse; removing the selected talent stops further apex payloads without removing Haunt (`sc_warlock.cpp41–49`). Native coefficients, direct damage modifiers and critical rolls remain unchanged.
- Wrath's separate RPPM and guardian lifecycle are specified below; Shadow of Nathreza remains its own native damage payload.

### Affliction native Wrath of Nathreza

- Selected1261992 rolls its native hasted RPPM and actual aura timers through `try_native_policy_proc` on landed Haunt impacts and real Haunt pulses. Generic spell damage cannot also roll this owned stream (`sc_warlock.cpp41–49`, `sc_warlock_actions.cpp2408–2415/5651–5662`). A newly landed Haunt replaces that caster's earlier Haunt.
- Native1262094 imports and summons authentic Desperate Soul254505. Only its new guardian policy binds1278047 with full owner SP1.0; one actual Wrath executes around the owner's current Haunt recipient, then native expiry dismisses the soul. Other guardian policies and native damage coefficients are unchanged (`sc_warlock_pets.cpp19/2516–2563`).
- Guardian publication preserves authentic TDB models, is idempotent across importer runs and does not create world spawns.
- Qualification covers ST/3T, native target selectors, one-cast lifetime, actual RPPM producers and talent removal. More-than-eight-target reduction is outside this parity fixture's scope.

### Affliction native Malefic Grasp extra ticks

- Every landed native1261153 tick copies each ticking owned Agony, UA, Corruption or Wither base tick into its native child1261166/1261176/1261158/1279686 at the native channel effect1 fraction. Agony/UA retain actual stacks; Wither counters do not scale its copy. Children retain independent native direct modifiers and critical rolls, not the parent's resolved damage (SimC76f8118 `sc_warlock_actions.cpp136–158/2210–2214`).
- The pinned `bugs=1` Agony child compensation is1.95, as in `sc_warlock_actions.cpp2036–2044`; it is not a replacement coefficient. Its landed impact advances the owned Agony counter without funding shards (`2052–2060`). Foreign DoTs cannot produce children.

### Affliction native Cascading Calamity

- A successful overlapping native1259790 application earns selected1261124's native1261125 buff. The first owned UA on a recipient does not earn it. Observe the actual stored aura through the shared application stage, including refreshes, without replacing native aura callbacks (`sc_warlock_actions.cpp1547–1550`).
- Final-stack Fatal Echoes also earns the buff when the previous UA has ended (`1604–1606`); overlapping echoes use their normal application callback once. Native buff values and duration remain authoritative.

### Affliction native Fatal Echoes

- Actual native1259790 asynchronous application expirations roll selected1260229's native chance through one per-caster PRD stream. Each expired stack rolls once, including the final stack; a successful proc queues a free, fully effective native1259790 reapplication after final-aura cleanup (SimC76f8118 `sc_warlock_actions.cpp1587–1606/5757–5780`, `sc_warlock_init.cpp1248–1253`).
- The echo retains actual target identity, cannot consume Shard Instability, and earns selected native1261125 Cascading Calamity even when no previous UA remains. Native spell coefficients, stack deadlines and costs for ordinary casts are unchanged.

### Affliction native Nightfall

- Landed owned Corruption146739 or Wither445474 ticks with selected108558 advance an independent uniform-initial/rollover threshold stream. Pinned base0.130 is normalized by active owned dots, native Creeping Death and selected Sataiel's Volition. This does not modify Wither stacks or damage.
- Native264571 supplies stack cap, lifetime and cast-time modifiers. An admitted instant Shadow Bolt consumes one use; actual Drain Soul and Malefic Grasp executions consume one use. The additive foreground hook preserves native Shadow Bolt shard funding.

### Shadow Idol of N'Zoth

Selected Idol of N'Zoth tracks actual Madness and Mind Blast executions, foreground Pain and Vampiric Touch executions, and Mind Flay and Void Torrent ticks per owned target. Its native thresholds trigger Horrific Vision and Vision of N'Zoth, their damage, and their native periodic Insanity buffs. The target counter rolls over at the native upper threshold.
### Shadow Insidious Ire

Selected Insidious Ire earns its temporary channel bonus when one target has all three caster-owned Pain, Vampiric Touch and Madness effects. The window lasts until the earliest expiry and may be refreshed to a later earned deadline. Its amount comes from the selected native trait effect.
- Specialization Raise Dead46584 creates native26125 through the controlled-pet path, retaining the actual current-pet relationship and permanent lifetime. Native Dark Transformation must see the pet through UNIT_PET. DK inherits source-qualified owner melee AP/crit/haste, and ordinary owner casts order the real pet against the selected hostile primary. Hunter, Warlock and Mage pet policies remain unchanged.

### Unholy Putrefy summons

- Native Putrefy summons its lesser ghoul. The actual ghoul travels to the selected enemy before delivering native single-target and area damage; it is then dismissed. Selected Lord of the Dead uses its native chance to raise a Magus after that dismissal. A MID2 two-piece Magus casts its native Necrotic Bolt and Withering Grasp using the source-qualified share of owner melee attack power. A Lord formed from real Magi lasts for the selected Lord of the Dead talent's effect2 duration (SimC76f8118 `sc_death_knight.cpp14752–14758`). The selected MID2 four-piece increases Magus and Lord Necrotic Bolt direct damage and Withering Grasp periodic damage at or below its native health threshold (`sc_death_knight.cpp4456–4464,4513–4519`). Do not substitute caster-only damage or invent creature data.
- Native Putrefy summons its lesser ghoul. The actual ghoul travels to the selected enemy before delivering native single-target and area damage; it is then dismissed. Selected Lord of the Dead uses its native chance to raise a Magus after that dismissal. A MID2 two-piece Magus casts its native Necrotic Bolt and Withering Grasp using the source-qualified share of owner melee attack power. An existing Withering Grasp continues after its Magus is consumed into a Lord (SimC76f8118 `sc_death_knight.cpp4545–4574`). A Lord formed from real Magi lasts for the selected Lord of the Dead talent's effect2 duration (SimC76f8118 `sc_death_knight.cpp14752–14758`). Do not substitute caster-only damage or invent creature data.

### Unholy Festering ready states

- Successful native Festering Strike and its real Scythe replacement each earn one uniform native min/max count of1254252 ready-ghoul stacks. Selected Festering Scythe arms458123 after Strike; successful replacement consumes that owned state and opens1241077. Do not substitute damage, alter rune costs or grant resources beyond native effects.
### Unholy Dark Transformation state

- After Dark Transformation transforms the controlled ghoul, apply the native owner state 1235391 for the literal APL. Do not grant that state without the transformed owned pet.

### Unholy controlled ghoul

- Specialization Raise Dead46584 creates native26125 through the controlled-pet path, retaining the actual current-pet relationship and permanent lifetime. Native Dark Transformation must see the pet through UNIT_PET. DK inherits source-qualified owner melee AP/crit/haste, and ordinary owner casts order the real pet against the selected hostile primary. Hunter, Warlock and Mage pet policies remain unchanged.

- Spec269 executes the pinned Windwalker action rows on native player and follower fixtures. Player measurements retain player identity and use the literal driver rather than AssistedCombat diagnostics.
- In the 269 player-literal fixture only, Touch of Karma uses SimC's default 100-second interval and modeled absorbed-health fraction; the real player health and native spell values supply the redirected periodic damage (`sc_monk.cpp2735–2753,2757–2850`).
- Rushing Wind Kick and Rising Sun Kick use separate action recharge clocks in gameplay and literal simulation; both retain native charge recovery and haste rules (`action.cpp474`, `sc_monk.cpp1143–1204`).

- Windwalker Tigereye Brew starts combat with its native initial stacks, earns one stack per native Chi threshold from qualifying casts, and transfers up to the native cap into Zenith critical chance (`sc_monk.cpp393–406,4583–4595,7170–7183`). Out-of-combat regeneration remains unimplemented.
- A Windwalker Tiger Palm that consumes an earned Combat Wisdom proc triggers native Expel Harm healing and its damage child, including overhealing in the damage basis (`sc_monk.cpp879–882,4002–4070`). Selected Strength of Spirit modifies the actual heal using the caster's current health and its native effect value (`sc_monk.cpp59–70,4028–4033`).
- Improved Touch of Death against a stronger target applies active Combo Strikes mastery to its native maximum-health damage; the weaker-creature execute branch does not (`sc_monk.cpp2701–2718`).
- Whirling Dragon Punch emits its area damage at the initial pulse and the next two native periods; its final two native aura periods do not emit damage (`sc_monk.cpp1888–1906`).
### Hellbent Commander

- Hellbent Commander reconciles native1281559 stacks to the eligible live demon roster on the pinned randomized 5.25s heartbeat. Actual summon and demise events retain the pinned bug-model ordering, including ordinary Hand Imp and alternate Imp Implosion differences; Diabolist demons are excluded (`sc_warlock.cpp303–338`; `sc_warlock_pets.cpp203–226,992–1006,1310–1341`).

### Devastation Dragonrage

- Dragonrage launches one native Pyre explosion for each enemy selected by its native target limit; each explosion hits enemies in its area.
### Evoker Slipstream

- A successful Deep Breath or Breath of Eons restores all Hover charges when Slipstream is selected.

### Evoker Rising Fury

- At maximum Rising Fury stacks, direct and periodic damage use the native rank-two bonus once when the native effect suppresses stacking.
- Unbound Flame damage has 100% base critical-hit chance.

## MID2 support-spec inputs

- Generate level-90 profiles for 65, 104, 105, 256, 257, 264, 270 and 1468 using `scripts/generate_mid2_support.py` and `scripts/mid2_support_manifest.json`. Copy the specified MID2 donor equipment and modifiers, with the recorded Evoker substitutions; use native spec primary stats. Record the talent source and retrieval date. These inputs do not claim a SimC healing reference.
- Each generated profile and talent allocation must load through `read_reference` and spawn through `spawn_reference` against the canonical world database.
