# Generic aura342 attack timers

## Source and scope

Own `aura342-attack-timers` from master1da5f8bcaa2cd71dcdb83bb1db689aea7ce34884. Explicitly approved separate from Momentum because every342 user changes. No spell-ID exceptions, producer/lifecycle/haste-damage policy or numeric tuning. Momentum will merge this slice locally after its separate handoff.

Pinned TC a352b1fa `SpellAuraEffects.cpp414` maps342 to `HandleModMeleeRangedSpeedPct`;4517–4528 applies `ApplyAttackTimePercentMod` to BASE_ATTACK, OFF_ATTACK and RANGED_ATTACK only. It does not call cast-time/GCD/regen updates; the adjacent combat-speed handler does call `ApplyCastTimePercentMod`. Current `stat_mods.rs` incorrectly groups342 with unified `StatModifiers.haste_pct`, so spell cast time/resource regeneration change and speed adds instead of multiplying attack time.

Move342 into its own `AuraStatMods.melee_ranged_attack_time_mult`, composed with existing melee-only319 and slow factors for main/off-hand and creature clocks. Ranged autorepeat consumes342's factor only, never melee-only319. Do not broaden the slice to other native haste types192/138/140, stacking-exclusivity redesign or unrelated slow handlers.

## Strict native RED

- Test-only6de758b88 yields3genuineRED12.62s: real451298 cast changes production cast-time1106→802ms, derived haste58.2727→118.2727%, and actual melee reset2.274→1.649s rather than2.274/1.6=1.42125s. GCD1.5s remains unchanged as a control; this probe does not claim a haste-sensitive GCD RED.
- Complete propagation baselineaae8bb2510fbdc1e5f732e178a61440f8fc3808a preserves3genuineRED16.17s, with actual uncapped Energy regeneration17.41→24.01 after three ordinary frames. First single-frame regen read was stale because stat folding precedes a later resource frame; it was not promoted as regen proof.
- Native ordinary451298 applies its actual aura342/effect060/duration8s, without a fabricated effect or stat. Actual Tiger Palm expenditure keeps Energy uncapped. Ordinary attack intent and production timer resets supply the interval. This generic folding test does not certify Momentum's currently absent451298 producer.
- `dance_rppm_tests.rs` explicitly had the stale premise that Martial Agility1272685/native342/base30 is unified regen haste. Replace that premise with TC's attack-only expectation; the zero-gear level55 fixture must have zero spell/resource haste and nativePPM57/mod221 remains based on actual regen haste. Do not change that aura's native data or fabricate stats.

## GREEN and integration

Implementation is committed before GREEN. Same three native cases, the corrected Dance control, formatter/diff/readability and lead broad/plain-bin gate remain pending. No300s rerun or claimed DPS gain. Native private canonical4442 backups, input/binary hashes, full logs and commands live under engine `data/diagnostics/ww-parity-2026-10-23/`: `aura342-proof-ledger.json`, `aura342-red-aae8bb251.build*`, `native-aura342-red-aae8bb251/`. Baseline binary SHA2564554a71f5737a88ea2fb302b9d3ec690947e794d44e527b8af05adb1ab43dc41.

## Full current4442 impact census

All72 native342 rows/spells are listed below, including zero-base/scaled rows: once their342 effect is active, it belongs to attack timers, not unified haste. Zero effective amounts do not change magnitude. Other effects on the same spell remain unchanged; this is not proof that every listed spell can currently be produced or that unrelated effects are implemented. Raw source rows are retained in `aura342-source/all-native-4442-aura342.json` with current canonical4442/private7bad provenance.

|SpellID|Native name|Effect index|Base percent|Scaling coefficient|
|---|---|---:|---:|---:|
|702|Curse of Weakness|0|-20.0|0.0|
|5171|Slice and Dice|0|50.0|0.0|
|5760|Numbing Poison|1|-15.0|0.0|
|6136|Chilled|1|-20.0|0.0|
|13750|Adrenaline Rush|1|20.0|0.0|
|23060|Battle Squawk|0|5.0|0.0|
|27648|Thunderfury|0|-20.0|0.0|
|114720|Melee and Ranged Haste Percent|0|10.0|0.0|
|117544|Spiritual Innervation|2|0.0|0.0|
|117545|Spiritual Innervation|2|0.0|0.0|
|117546|Spiritual Innervation|2|0.0|0.0|
|117547|Spiritual Innervation|2|0.0|0.0|
|117688|Spiritual Innervation|2|0.0|0.0|
|133541|Swiftblade's Nod|0|10.0|0.0|
|138130|Storm, Earth, and Fire|4|0.0|0.0|
|162981|Hiss|0|10.0|0.0|
|166588|Rampage|0|6.0|0.0|
|172429|Void Touched: Disorienting Blast|0|-30.0|0.0|
|172435|Test Magic Debuff|0|-30.0|0.0|
|172447|Test Magic Debuff - Enemy|0|-30.0|0.0|
|173861|Choking Vines|2|-35.0|0.0|
|174436|Debilitating Vial|1|-20.0|0.0|
|174842|Reactive Pollen|1|-25.0|0.0|
|174896|Rabid Bite|2|-25.0|0.0|
|182773|Heavy Weaponry|0|-100.0|0.0|
|185313|Shadow Dance|7|25.0|0.0|
|186286|Adrenaline Rush|1|20.0|0.0|
|194879|Icy Talons|0|0.0|0.0|
|195605|Swift as the Wind|1|70.0|0.0|
|198293|Wind Strikes|0|3.0|0.0|
|198473|Frost Armor|1|-20.0|0.0|
|200953|Berserking|0|3.0|0.0|
|202033|Adrenaline Rush|1|20.0|0.0|
|211510|Solitude|0|10.0|0.0|
|212283|Symbols of Death|4|0.0|0.0|
|212704|The Beast Within|4|10.0|0.0|
|246558|Lesser Adrenaline Rush|1|10.0|0.0|
|260249|Bloodseeker|0|10.0|0.0|
|267612|Gale-Force Striking|0|15.0|0.0|
|269571|Zeal|0|30.0|0.0|
|272790|Frenzy|0|20.0|0.0|
|284267|Frenzy|0|30.0|0.0|
|293031|Suppressing Pulse|1|-25.0|0.0|
|315496|Slice and Dice|0|50.0|0.0|
|317009|Sinful Brand|2|-30.0|0.0|
|328506|Blessing of Winter|2|-2.0|0.0|
|353100|Numbing Strike|1|-15.0|0.0|
|357140|The Beast Within|2|10.0|0.0|
|359078|Numbing Poison|1|-25.0|0.0|
|381965|Static Electricity|0|2.0|0.0|
|382889|Flurry|0|15.0|0.0|
|383918|Cadence of Fujieda|0|5.0|0.0|
|388012|Blessing of Winter|2|-2.0|0.0|
|404542|Crusading Strikes|1|15.0|0.0|
|439606|Perilous Fate|1|-50.0|0.0|
|439897|Silken Chain Weaver|0|0.0|0.0|
|442804|Curse of the Satyr|0|-20.0|0.0|
|446077|Tunneling|0|2.0|0.0|
|449587|Aura of Enfeeblement|0|-20.0|0.0|
|451298|Momentum Boost|0|60.0|0.0|
|473153|Slice and Dice|0|50.0|0.0|
|1217377|Frenzy Strikes|0|25.0|0.0|
|1250646|Takedown|3|0.0|0.0|
|1250725|Stormspirit Strikes|4|0.0|0.0|
|1252415|Crash Lightning|1|0.0|0.0|
|1257338|Slice and Dice|0|100.0|0.0|
|1270491|Unholy Devotion|0|25.0|0.0|
|1271231|Malfunctioning Energy Generator|0|30.0|0.0|
|1271744|Whirlwind Cleave|2|-10.0|0.0|
|1271798|Blight of Weakness|0|-100.0|0.0|
|1272685|Martial Agility|0|30.0|0.0|
|1297365|Freezing Tempest|0|1.0|0.0|

## Sources

- [DPS contract](../specs/dps-parity.md), [combat clocks](../wiki/systems/combat.md).
- Pinned TC local source `game-engine/data/diagnostics/follower-power-scaling-2026-10-22/tc-a352b1fa-SpellAuraEffects.cpp` (handler4517–4528), SHA25617d13becb8d0e9c8e417b8d36e461968b9678eeb16f284425d08977d07922776.
