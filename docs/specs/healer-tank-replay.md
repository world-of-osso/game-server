# Healer and tank replay

Replay recorded incoming damage in the native server combat simulation to compare healer and tank performance on the same timeline.

## What it must do

- [ ] Load incoming hits with time, damage school, raw amount and target role. Reject invalid timelines rather than silently changing them. An optional `--party-trace` replays SimC raid-event source rolls onto the five native party slots alongside the tank trace.
- [ ] The native replay CLI accepts `HEALER_ENEMY_PROFILE` as a JSON file with a provenance string and half-open enemy intervals in milliseconds. Enemies enter and leave combat at their interval boundaries. The Altar of Fangs fixture is INFERRED_WCL from report `pPFDbkfQK7MACtmY`, fight 41 pull 1: first-to-last player damage to each NPC instance, not observed spawn or death times. `HEALER_WCL_INTAKE` replaces both SimC damage streams with that pull's recorded incoming damage; it requires an enemy profile and rejects a simultaneous synthetic party trace. `scripts/healer_wcl_intake.py` converts WCL damage events and observed victim max HP using school and defense metadata from the selected world snapshot. Per-role pressure is `(WCL amount + absorbed) / WCL max HP × native max HP`, delivered after mitigation but before native absorbs; armor and damage reductions are not applied twice. The replay uses the WCL pull's Brewmaster, Preservation, Arcane and two Elemental roles and does not add synthetic Bloodlust or raid debuffs. WCL Stagger 124255 and Time Dilation 361029 self-ticks are replayed as deferred enemy damage attributed to an engaged hostile; the pooled source is INFERRED_WCL because the log does not identify the original attacker. Other self-sourced rows are excluded. WCL records Stagger 115069 and Time Dilation 357170 deferral as absorbed on the initial hit; uniquely match those absorb receipts by target, attacker instance, ability and timestamp (same or +1 ms) and subtract them from initial pressure before replaying the later ticks. Native Stagger and damage-transfer routing are bypassed for these already-mitigated receipts so the same damage is not deferred twice. WCL environmental sources are assigned an engaged hostile and are inferred. The recorded miss, dodge, parry, deflect and immune outcomes generate corresponding native hostile proc events. WCL table receipts establish codes 0/7/8 from `pPFDbkfQK7MACtmY` fight 41 pull 1 and 9/10 from `Hx64j9RrfnytB8cA` fight 12. The report includes per-hit WCL and native HP, health-loss and absorbed fractions; normalizing post-mitigation pressure does not guarantee identical health curves.
- [x] Raid buffs follow the party composition, as production followers cast them ([follower utility](follower-utility.md)). Tank traces use `optimal_raid=1` with `override.<buff>=1` only for the raid buff the solo tank's class provides and `0` for the other five (`scripts/dps_parity.py` `composition_options`). The native tank casts that buff before the fight. The replay fails with `SIMC_TRACE_RAID_BUFFS_NOT_COMPOSITION` when the trace's raid-buff overrides differ from the tank's provided set. Healer replay applies no raid buffs: before the pull, the class party casts its own provided set. Composition traces and ten-measured references: `game-engine/data/diagnostics/party-buffs-composition-2026-11-02/tank/`. Earlier traces in `healer-tank-replay/` carry the old fixed four or six buffs and are rejected.
- [ ] Prefer SimulationCraft tank-dummy output as the timeline source; also accept recorded World of Osso runs. Preserve the distinction between swings, strikes and bleeds. SimC direct damage amounts are source rolls before hit outcomes and must not be reported as post-crit damage. Use each trace-declared attacker miss and crit chance while retaining native defender dodge, parry and block. Replay the physical spell_dot's zero-damage application as a landed direct spell hit, separate from its later bleed ticks.
- [ ] Use `/home/osso/Projects/world-of-osso/game-server/data/world.db` for measurements unless an explicit immutable snapshot is selected for tank replay with `DPS_PARITY_WORLD_DB`. Tank startup and reference catalogs must read the same selected database. Level-90 replay healers use the most common exact +17 WCL talent build per spec from `scripts/reference/healer-wcl-loadouts.json`, and the tracked MID2 top-character profile equipment; other party actors retain Journal gear. Level-70 Stockade is a Journal-gear smoke test. The replay party is single-faction; the healer adopts the tank's faction template before setup advances the world, while retaining its profile race and stats.
- [ ] Replay hits against a dummy party through native combat, with Holy Paladin (65) as the first healer and Protection Paladin (66) as the first tank. Extend the same harness to every healer and tank specialization and their spells. The initial Holy Paladin benchmark uses its native learned Holy Light and Flash of Light; this is not complete spell coverage.
- [ ] In SimC healer replay, begin the measured fight with the full update in which the first of tank or healer enters combat, rather than waiting for both. This includes instant actions on that transition and opening actions while the other completes precombat. The Restoration Druid 0.824 ratio and earlier Mistweaver results used the old boundary; remeasure them before comparing against the new boundary. Project the enemy training dummy's health from 100% to 0% over the requested fixed fight duration, using the DPS fixture's SimC fixed-time target-health model. This controls execute thresholds and does not change the incoming damage trace.
- [ ] For each healer report effective HPS, raw HPS, overheal percentage, per-spell HPS, mana over time, time to first party death, and idle-GCD percentage. Idle-GCD percentage is the fraction of fight time sampled at 50 ms when the living healer has no active cast or channel and no active GCD, after the policy has acted; dead time is not idle. Sample mana at one-second intervals. Use the recorded combat duration for HPS and report no death as null.
- [ ] For each tank report raw and mitigated damage taken and mitigation percentage, self-healing and consumed absorbs per second, time to death without a healer, active-mitigation uptime, and largest five-second damage spike. Raw incoming damage includes avoided attacks; mitigation includes avoidance. Damage taken and spike include self-sourced periodic damage from tank mechanics. Stop the tank measurement at death; report no death as null. For the initial Protection Paladin, active mitigation is Shield of the Righteous.
- [ ] In SimC dummy tank replay, reconsider the literal APL at the active global cooldown's ready time. During a GCD, check off-GCD actions immediately and retry at Gaussian 100±10 ms intervals; when no action is ready, retry the foreground APL at the same distribution. Draw poll delays from the seeded replay RNG (SimC `player.cpp:343–351,6790–6793,7130–7133`, `action.cpp:303–306`). Simulation and sampling boundaries remain at 50 ms.
- [ ] Generated class-follower SimC references must disable flasks, food, potions and augmentations, and must set `temporary_enchant=disabled`, because native class followers carry no consumables or weapon oils. Without that key, SimC applies its spec default weapon enchant (`player.cpp:1786–1791`). Profiles that name an explicit oil instead install the same oil natively through `profile_oils.rs`.
- [ ] In SimC tank replay, apply the trace-selected Touch of Elune day or night racial for a Night Elf; reject a trace and profile whose racial state disagrees. SimC defaults to night (`player.cpp:1003,16405–16407`).
- [ ] In SimC dummy comparison mode, apply and report the trace-declared Mark of the Wild, Arcane Intellect, Power Word: Fortitude, Battle Shout, Skyfury and the player's class-specific Blessing of the Bronze. Apply the target's Chaos Brand, Mystic Touch, Hunter's Mark and Mortal Wounds, and preserve its trace-declared bleeding state without generating a bleed spell or damage. Do not add them to raid-free DPS parity or no-healer replay.
- [ ] Mistweaver Mana Tea uses native-description-derived spending, stack cap, channel consumption and mana restoration. The pinned SimC source has no Mistweaver Mana Tea model.
- [ ] Print report tables with `scripts/healer_tank_replay.py`. Healer runs (`--mode healer` and `--all`) require `--level 70|90` and the measurement snapshot (`--world-db`, default `$FOLLOWER_DPS_WORLD_DB`), which only healer replays receive. SimC healer reports include actual mana paid by spell, frame-sampled minimum healer mana, minimum tank health including immediately after each incoming hit, and tank damage receipts per replay second.
- [ ] `--all` runs all seven healers on the Protection Paladin trace and six tanks on their respective 300-second seed-70448 SimC dummy traces, with both externally healed and no-healer tank runs. Run replays in parallel and print healer and tank summary tables; the healer table includes idle-GCD percentage. Write per-spell details in dated output files.
- [ ] See [Restoration Shaman (264)](healer-tank-replay/264.md) for its replay policy and data gaps.

Matched WCL replays require all five actors' recorded CombatantInfo talent entries and gear, including item levels, bonus IDs, enchants, gems and trinkets. The unmatched replay retains its reference profiles. Report race as unmatched when the combatant data does not identify it.

Gate normalized WCL intake for each party member in 15-second windows up to the first native party death, including the last partial window, but stop immediately before the fatal hit because remaining health clips its effective damage. Count the damage commit and all consumed absorbs, including death-prevention absorbs; do not subtract a rescue heal from the incoming damage. Report death and post-death undelivered hits as outcomes, not intake-gate failures. Include timestamped tank and healer casts, healer healing, party self-healing and self-absorbs, and a per-second party-health curve to investigate the 10 seconds before death. WCL intake conversion retains only the configured measurement interval; later pull events cannot invalidate that interval. WCL replay configurations record each party member's observed Bloodlust application time. Apply the 40-second native haste aura to each recorded recipient at that time. Treat WCL blocked hits as already-mitigated landed pressure. For changing WCL maximum health, use the latest available observation at each hit; if the first observation is later than the hit or pull start, record its timestamp as an inference rather than treating it as an observed starting value. Exclude WCL source-unknown Shadow Word: Death 32409 when its target is a party Priest: it is the Priest's self-inflicted backlash. Do not represent friendly damage-transfer events as hostile hits. WCL ability 3 is its Falling environmental sentinel, not the unrelated DB2 spell 3; replay it as physical non-spell pressure using the inferred engaged-hostile source, so its hostile proc semantics remain an approximation.

Opening auras with a recorded full removal are removed at that WCL timestamp, at the next replay tick, from the exact applied instance. A later native reapplication is not removed. **VERIFIED_WCL:** in `mHcDYRVyprndwjFC` fight 2 pull 2 (report start 2026-10-31), the Shaman's opening Ghost Wolf 2645 is removed at 2.535 seconds.

**VERIFIED_WCL** `mHcDYRVyprndwjFC` fight 2 pull 2 (2026-10-31): the matched Affliction actor selects Siphon Life 452999 and receives 453000 heals after Wither 445474 ticks. DB2 70448 effect 1 supplies 5% of eligible periodic damage; the effect 0 class mask identifies eligible spells.

**VERIFIED_WCL** `BCX8v3VPd6pnyzM4` fight 1 pull 1 (2026-11-01): the matched Holy Priest selects Light's Resurgence 193157; Prayer of Mending 33110 and Renew 139 land on the Priest at 16.224 seconds. DB2 70448 supplies its 12% chance per Mending heal.

**SOUL_LEECH_FEL_SYNERGY_GAP:** `VERIFIED_WCL` `mHcDYRVyprndwjFC` fight 2 pull 2 (2026-10-31) actor 2 selects Soul Leech 1311653, Demon Skin 219272 and Fel Synergy 389367; actor 3 is its pet. Soul Leech 108366 is consumed at 16.863 seconds; actor 2 deals multiple Agony ticks from 16.930 to 17.196 seconds, but 108366 and the 389372 heal reappear at 17.247 seconds. DB2 70448 describes single-target damage shields, periodic recharge and distinct owner/pet healing fractions. Native code has no corresponding shield generator or Fel Synergy heal. The observed cadence does not establish whether damage-grant batching is general or specific to Demon Skin. Recharge depends on the same accumulating, capped shield state, so no recharge-only approximation is installed. Evidence: `game-engine/data/diagnostics/healer-priest-shaman-2026-11-03/264-{combatants,buffs,damage-done,healing}.jsonl` and `264-actor-metadata.json`.
**VERIFIED_WCL** `mHcDYRVyprndwjFC` fight 2 pull 2: the matched Guardian selects Elune's Favored 370586 and receives 370602 healing at approximately one-second intervals. DB2 70448 supplies its 18% of Arcane damage in Bear Form and the one-second 370588 timer. The Guardian's WCL Frenzied Regeneration casts do not establish a general cast threshold; the native Guardian APL does not currently select it.

## Equal-health healer target ties

Healer target selection chooses the lowest health fraction. On exact ties it chooses the tank, then the lowest entity ID; it does not use a near-equal threshold. **INFERRED_WCL:** in seven current-build Altar Ritual Chieftain pulls, five first directed heals or HoTs targeted the tank. First three directed targets (T=tank, H=healer, D=DPS), excluding triggered duplicate casts and utility:

| Spec | WCL report / fight / pull | First three targets |
|---|---|---|
| 65 | `dtmp7Lg2abzyZVBG` / 3 / 1 | T, T, D |
| 105 | `2gBLy8vcfWAHQ7t3` / 25 / 1 | T, T, H |
| 1468 | `vhKYp3J2BcVdQ7fa` / 2 / 1 | T, T, D |
| 256 | `R19Zb4BqjxKvgzGm` / 13 / 1 | H, D, D |
| 257 | `BCX8v3VPd6pnyzM4` / 1 / 1 | T, H, D |
| 264 | `mHcDYRVyprndwjFC` / 2 / 2 | T, D, D |
| 270 | `BaN8nxT64QztvMkH` / 1 / 1 | D, T, H |

Mistweaver's first directed heal occurs at 29.64 seconds; attack-triggered automatic heals are excluded. These observations support a tie policy, not a rule for unequal health.

## Healer parity reference population

The [seven-healer hostile-hit recheck](healer-tank-replay/seven-healer-recheck-2026-11-02.md) records the same-binary ratios and the remaining Mistweaver gap.

The replay's talents are the modal exact builds among deduplicated +17 top-300 WCL rankings for Altar of Fangs and Den of Nalorakk. The WCL comparison is the mean of the two dungeons' spec-median-to-Holy-Priest-median HPS ratios. This compares populations rather than matching one named actor; named logs in specialization pages provide spell-mechanic evidence, not the ratio population. Rankings used for the medians may be capped and are not complete-population medians.

Level-90 healers wear their own tracked top-character profile gear, not identical Journal gear. Other party members wear Journal gear. Thus gear differences and the synthetic intake can affect the ratio; they do not necessarily cancel against Holy Priest.

## Specializations

- [Restoration Druid 105](healer-tank-replay/105.md)
- [Preservation Evoker 1468](healer-tank-replay/1468.md)

## Specializations

- [Brewmaster (268)](healer-tank-replay/268.md)

## Specialization notes

- [Guardian Druid (104)](healer-tank-replay/104.md)

## Composition tank status (2026-11-03)

All six replays use the 300-second composition-buffed dummy traces, external healing, native seeds `70448, 1–9`, and native tree `389ca621d0ed4ab7eef613a038bd67b8b79369b5`. All 60 runs survived and reported no fact errors. The reference uses seed 70448 and ten measured iterations. Native 95% Student-t intervals do not include independent reference uncertainty. Deltas are `(native / reference − 1) × 100`; the flag applies to the point estimate, not its confidence interval.

| Tank | Native taken/s ± 95% CI | SimC taken/s | Taken gap | Native DPS ± 95% CI | SimC DPS | DPS gap |
|---|---:|---:|---:|---:|---:|---:|
| Protection Paladin 66 | 12,852 ± 956 | 11,826 | +8.67% outside | 131,854 ± 3,349 | 142,925 | −7.75% outside |
| Protection Warrior 73 | 9,975 ± 269 | 10,622 | −6.09% outside | 136,235 ± 2,783 | 157,626 | −13.57% outside |
| Guardian 104 | 6,654 ± 562 | 7,007 | −5.04% outside | 101,070 ± 2,258 | 103,423 | −2.27% inside |
| Blood 250 | 227 ± 91 | 230 | −1.33% inside | 138,343 ± 3,554 | 127,534 | +8.47% outside |
| Brewmaster 268 | 11,939 ± 589 | 12,719 | −6.13% outside | 122,786 ± 2,492 | 140,478 | −12.59% outside |
| Vengeance 581 | 19,939 ± 296 | 19,302 | +3.30% inside | 126,575 ± 2,974 | 130,306 | −2.86% inside |

SimC `dtps` is damage per fight here, divided by 300. For Brewmaster it includes self-Stagger, so dummy DPS alone is not the reference. Blood's near-zero taken makes a percentage gate unstable. Guardian's reference corrects Ursoc's Fury, Matted Fur and Lunar Beam self-healing; its full modifier model remains inferred. The 73, 268 and 581 composition references use the counterfactual corrections described in their specialization pages; 66 uses solo `fake_solidarity=0`, and 250 uses the canonical reference.

### Scenario-specific rankings

These are not general tank rankings. The six traces have the same dungeon-dummy scenario but are distinct source rolls: native raw intake ranges from 152,004 to 152,335/s, not exactly equal. Taken is after absorbs and before self-healing; gear and talents differ. Adjacent intervals are classified by overlap only, not by a test of paired differences. Non-overlap is evidence of separation under this criterion; overlap is inconclusive.

| Rank | Lower taken/s first | Next neighbor's 95% CI | Higher outgoing DPS first | Next neighbor's 95% CI |
|---:|---|---|---|---|
| 1 | Blood 250 | no overlap | Blood 250 | overlap |
| 2 | Guardian 104 | no overlap | Protection Warrior 73 | overlap |
| 3 | Protection Warrior 73 | no overlap | Protection Paladin 66 | overlap |
| 4 | Brewmaster 268 | overlap | Vengeance 581 | overlap |
| 5 | Protection Paladin 66 | no overlap | Brewmaster 268 | no overlap |
| 6 | Vengeance 581 | — | Guardian 104 | — |

The existing matching-spell breakdown shows large outgoing differences in Shield Slam (73: native 36,496/s vs SimC 47,251/s), Death Strike (250: 56,834 vs 39,066), and Tiger Palm (268: 31,721 vs 35,248). These independent-sample differences do not establish causes. No single contributor to the other out-of-band gaps is established by this breakdown.

Native binary SHA-256: `2ec446070b15bcccdaa318db0dcd01ffa4b5670af7f726cf4fbfd6473598b697`. World DB SHA-256 before and after the batch: `7cb667e44a870ef3159024829ce706cb970d514fe106f6618befb9ca6321c97b`. Reports, `summary.json`, trace and profile hashes are in `game-engine/data/diagnostics/tank-remeasure-2026-11-03/`. Composition references and their command, profile and binary hashes are in `game-engine/data/diagnostics/party-buffs-composition-2026-11-02/tank/`. Corrected Guardian reference and proof are in `/home/osso/.worktrees/guardian-104-batch/post-callback/`, SimC binary SHA-256 `583f112321241cb997504978e7365b60540680e69eb8badc25a03acb12036e26`.

## GCD-ready timing comparison (2026-10-29)

Healed native replays of each tank's 300-second dummy trace used seeds 70448–70457 on `2341210d2` (50 ms APL checks) and `be3867541` (additional exact GCD-ready checks). The latter still checks non-GCD state on the 50 ms cadence; it is not a fully event-driven APL. The SimC column uses ten independent measured iterations; Guardian uses the corrected single-application Matted Fur and Ursoc's Fury reference. Damage is the mean per fight; all native runs survived.

| Spec | Before | GCD-ready checks | SimC |
|---:|---:|---:|---:|
| 66 | 3,304,981 | 3,311,661 | 3,341,843 |
| 73 | 2,893,245 | 2,893,206 | 2,783,862 |
| 104 | 1,766,524 | 1,735,614 | 1,850,456 |
| 250 | 47,621 | 56,541 | 88,144 |
| 268 | 3,429,579 | 3,199,642 | 3,318,109 |
| 581 | 5,837,808 | 5,783,904 | 5,949,784 |

The paired change in damage has standard error 217,665 / 38,987 / 86,436 / 14,251 / 56,211 / 55,522 respectively. Native reports: `game-engine/data/diagnostics/healer-tank-replay/all-tankgaps2-ready-baseline-ten-seeds/` and `all-tankgaps2-gcd-events-ten-seeds/`. These samples do not establish full parity.

## Gaussian decision-poll comparison (2026-10-29)

The replay now schedules foreground readiness and Gaussian 100±10 ms off-GCD and idle polls instead of polling every 50 ms. It still does not reproduce SimC's optimization that suppresses off-GCD checks when no off-GCD action can become ready before the foreground event; extra checks can consume RNG draws. Healed replays use the same traces and ten native seeds as above. SimC has ten independent measured iterations; 104 uses the corrected single-application reference.

| Spec | 50 ms baseline | Gaussian polls | SimC |
|---:|---:|---:|---:|
| 66 | 3,304,981 | 2,852,249 | 3,341,843 |
| 73 | 2,893,245 | 2,900,928 | 2,783,862 |
| 104 | 1,766,524 | 1,855,710 | 1,850,456 |
| 250 | 47,621 | 49,054 | 88,144 |
| 268 | 3,429,579 | 3,376,957 | 3,318,109 |
| 581 | 5,837,808 | 5,759,202 | 5,949,784 |

All native runs survived. The paired mean damage changes have standard errors 247,554 / 42,753 / 83,599 / 18,516 / 87,889 / 71,242 respectively. The Paladin shift of −452,732 is about 1.8 paired standard errors and remains unresolved at this sample size. Both native samples averaged 92.8 Shield of the Righteous, zero Guardian of Ancient Kings and 5 Ardent Defender casts; SimC averaged 91.6, no Guardian entry and 5.0 respectively. Mean Holy Bulwark consumption rose from 3,156,938 to 3,617,946, accounting for most of the damage difference, but its cause and persistence are unproven. Reports: `game-engine/data/diagnostics/healer-tank-replay/all-tankgaps2-ready-baseline-ten-seeds/` and `all-tankgaps2-simc-polls-ten-seeds/`.

## SimC attack-table inputs

SimC 76f8118 logs raw miss, dodge and parry chances before constructing the attack table. A nonpositive chance creates no corresponding avoidance outcome (`engine/action/attack.cpp:189–241`); the replay records that outcome as zero. Nonfinite values and invalid crit chances remain errors.

## Scaled tank pressure

`--pressure-scale X` multiplies the raw amounts of all SimC tank-dummy swings, strikes and bleed ticks before native combat; timestamps, attack tables and other trace fields remain unchanged. The report records `pressure_scale` and `incoming_outcome_sequence` (outcome, parry, dodge and block chance per attack). The scale must be positive and finite.

Recipient rolls (avoidance, block, critical block) for each replayed attack draw from their own stream, seeded from the run seed and the attack's trace index. The tank's procs and decisions use the separate actor stream, so a scale change cannot reshuffle later attack outcomes. For the same seed, equal avoidance and block chances roll equal outcomes at every scale. Blood and Brewmaster still change outcomes across scales, because their own procs change the chances themselves: Dancing Rune Weapon raises Blood parry from about 34% to 65%, and Elusive Brawler raises Brewmaster dodge.

`--pressure-sweep` runs each tank without external healing at every scale from 1.0 to 5.0 in 0.25 steps (17 scales) and ten consecutive native seeds, starting at 70448 by default. The survival scale is the last grid scale before the first scale with fewer than five survivors; a later recovery does not count. The sweep also reports mean net damage per second at 1.0×. Damage taken is already after absorbs, so net damage subtracts effective self-healing only. Per-seed reports and `summary.json` are written to the output directory. This synthetic trace is not a general ranking of tank performance in encounters.

### Qualified tank comparison (snapshot v3)

These are **scenario-specific estimates, not a general tank ranking**. The DTPS tables rank lower expected-avoidance damage taken first, after absorbs but before self-healing. They use ten 120-second paired follower replays per row with the recorded SimC dummy traces, infinite health and external healing. Means have Student-t 95% intervals. The point-estimate ordering is not proof of differences between tanks: intervals overlap, profiles and reference adjustments differ, and there is no paired cross-spec significance test. The 70 and 90 profiles have different gear and talents; do not compare values across levels.

| Rank | Level 70 tank | Expected native DTPS ± 95% CI | Qualification |
|---:|---|---:|---|
| 1 | [Protection Paladin 66](healer-tank-replay/66.md) | 72,862 ± 3,889 | Native/reference difference +4.11 ± 2.59 pp; interval crosses ±5%. |
| 2 | [Protection Warrior 73](healer-tank-replay/73.md) | 73,737 ± 4,367 | Each side's own per-swing avoidance; adjusted reference uses logged-keystone APL and omits Focused Vigor's DB2 Strength effect. |
| 3 | [Brewmaster 268](healer-tank-replay/268.md) | 74,253 ± 4,561 | Pre-Stagger expectation; keeps native Treemouth self-damage. |
| 4 | [Vengeance 581](healer-tank-replay/581.md) | 75,897 ± 1,057 | Reference needs retail-supported armor correction; corrected delta +0.91 ± 2.19 pp. |
| 5 | [Guardian 104](healer-tank-replay/104.md) | 88,526 ± 3,531 | WCL/DB2-adjusted reference; delta +3.03 ± 2.61 pp crosses ±5%. |
| 6 | [Blood 250](healer-tank-replay/250.md) | 122,539 ± 6,896 | DTPS excludes Death Strike healing; compare the separate survival measure. |

| Rank | Level 90 tank | Expected native DTPS ± 95% CI | Qualification |
|---:|---|---:|---|
| 1 | [Brewmaster 268](healer-tank-replay/268.md) | 53,426 ± 1,768 | Pre-Stagger expectation; its normalization differs from other rows. |
| 2 | [Protection Warrior 73](healer-tank-replay/73.md) | 68,904 ± 1,959 | Each side's own per-swing avoidance; adjusted-reference delta −4.83 ± 5.52 pp. |
| 3 | [Guardian 104](healer-tank-replay/104.md) | 69,749 ± 2,462 | WCL/DB2-adjusted reference; expected delta +0.36 ± 2.07 pp. |
| 4 | [Protection Paladin 66](healer-tank-replay/66.md) | 73,976 ± 1,080 | Solo reference; delta −3.91 ± 2.34 pp crosses ±5%. |
| 5 | [Vengeance 581](healer-tank-replay/581.md) | 75,058 ± 2,570 | Retail-armor-adjusted delta +4.29 ± 5.59 pp crosses ±5%. |
| 6 | [Blood 250](healer-tank-replay/250.md) | 93,244 ± 8,606 | First ten seeds of the documented 30-seed run; excludes Death Strike healing. |

The comparison-only reference adjustments are **INFERRED**: Guardian uses the WCL/DB2 absorb evidence in [104](healer-tank-replay/104.md); Warrior uses the DB2 and WCL charge/armor evidence in [73](healer-tank-replay/73.md); Brewmaster applies DB2 Calming Presence as described in [268](healer-tank-replay/268.md); Vengeance re-mitigates with DB2 and WCL armor evidence in [581](healer-tank-replay/581.md). Retire each adjustment when the pinned SimC reference reproduces the corresponding retail behavior, or revise it if stronger retail evidence contradicts it. None is a native data override.

For 104, 250 and 73, expected DTPS and intervals above scale each side's landed swing damage by expected/observed landed swings, preserving other damage. The first ten level-90 Blood seeds give +1.51 ± 1.81 pp rather than the documented 30-seed result. Warrior uses each side's own measured per-swing probabilities, which are not identical; its post-absorb rescaling remains approximate because block, shield depletion and procs can correlate with avoidance. The 268 pre-Stagger calculation changes incoming swings before mitigation and shields. Receipts: `game-engine/data/diagnostics/tank-fightremains-v3-2026-11-01/`, `tank-notemp-v3-2026-11-01/`, `prot-warrior-parity-2026-11-01/`, `tank-parity-brm-prot-2026-11-01/` and `vdh-parity-2026-10-31/`. Analyses: `/home/osso/.worktrees/logs/tank-104-581-estimates.py` and `/home/osso/.worktrees/tank-73-parry-evidence/analyze.py`.

**Survival is a separate measurement.** The following is a mixed-tree, ten-seed, level-90 no-healer 300-second pressure grid, not the DTPS test. The 0.25-step scale is the last step before fewer than five survivors, as defined above. The interval shown is a 95% Wilson interval for survival probability at that single selected step, **not a confidence interval for the survival scale or rank**; selection and sparse seeds make the scale uncertain.

| Rank | Tank | Scale; survivors/10 | Survival probability 95% CI | Qualification |
|---:|---|---:|---:|---|
| 1 | Vengeance 581 | 4.50×; 6/10 | 31.3–83.2% | Post-attr6 tree; SimC omits two healing talents. |
| 2 | Blood 250 | 3.75×; 5/10 | 23.7–76.3% | Integrated `a14a55cc97` grid; exactly at the threshold. |
| 3 | Brewmaster 268 | 3.25×; 8/10 | 49.0–94.3% | Integrated grid, before the newer level-70 fixes. |
| 4 | Protection Warrior 73 | 3.00×; 10/10 | 72.2–100% | Post-Pain and Gain correction; SimC omits that talent. |
| 5 | Guardian 104 | 2.00×; 10/10 | 72.2–100% | Post-Wildfire/proc-mask grid; 3/10 at 2.25×. |
| 6 | Protection Paladin 66 | 1.25×; 8/10 | 49.0–94.3% | Integrated grid; not a current same-tree comparison. |

The corrected per-spec survival grids for 73, 104 and 581 supersede their older integrated-grid rows. The other three retain the integrated grid below. These measurements use different native trees and have not been rerun as one integrated grid; the ordering is provisional and must not be read as retail class balance.

### Integrated pressure grid at `a14a55cc97`

All six tanks were rerun on one integrated tree, with 300-second traces, no external healer, ten seeds 70448–70457 and the full 1.00–5.00× grid at 0.25 increments. The preceding comparison is the mixed-tree grid in `pressure-sweep-731ff1b3dc-66-104/merged-summary.json`: 66 and 104 used `731ff1b3dc`, while the other four used `fc2dbbabb6`. Results and all 1,020 reports are in `game-engine/data/diagnostics/healer-tank-replay/pressure-sweep-a14a55cc97/`.

| Tank | Survival scale, previous → current | Net damage/s at 1×, previous → current | Survivors at current scale |
|---|---:|---:|---:|
| Vengeance (581) | 4.50 → 4.75 | −58 → −11 | 6/10 |
| Protection Warrior (73) | 4.75 → 4.50 | +5 → +3 | 10/10 |
| Blood (250) | 3.50 → 3.75 | 0 → 0 | 5/10 |
| Brewmaster (268) | 3.25 → 3.25 | +72 → +42 | 8/10 |
| Guardian (104) | 2.00 → 2.00 | −433 → −262 | 8/10 |
| Protection Paladin (66) | 1.25 → 1.25 | +483 → +404 | 8/10 |

The current sample reverses Vengeance and Warrior; the 0.25-step separation is not reliable with ten seeds. Blood's 3.75 result is exactly 5/10. All tanks survived 10/10 at 1×. Effective self-healing is limited by missing health, so net damage is not a ranking of surviving tanks.

Against finite-health, no-healer, no-raid-buff SimC single-fight runs using the same scales and seeds, the survival scales are 66 **1.25×**, 73 **3.00×**, 104 **1.00×**, 250 **3.50×**, 268 **3.50×**, and 581 **3.75×**. This uses dungeon armor coefficient 4,531.03 and level 92, disables Paladin fake Solidarity, selects night racial for 104 and 581, and corrects SimC's extra Guardian absorb multipliers. Guardian SimC still sends Lunar Beam healing to the dummy rather than self. At the first failing step, 66 has 1/10 survivors at 1.50×; 73 has 3/10 at 3.25×. Individual runs are in `game-engine/data/diagnostics/healer-tank-replay/simc-solo-unbuffed-finite-dungeon-single-grid/`. These independently sampled results are not general class rankings.

The trees differ in multiple ways. Vengeance includes trace-selected Touch of Elune and shared replay scheduling changes; Blood includes Rune Weapon/Hemostasis, Boiling Point and Icy Talons changes; Guardian includes Galactic Guardian, Keeper haste, Touch of Elune and Thrash modifiers; Paladin includes Sacred Weapon target and callback changes. Warrior and Brewmaster include shared scheduling and other integrated changes. These changes are potential contributors, not established causes of a particular grid shift: no isolated counterfactual was run for this ranking.

### Earlier pressure grid results

Protection Paladin (66) and Guardian Druid (104) were rerun on master `731ff1b3dc` after the Matted Fur total-AP fix. Warrior (73), Blood (250), Brewmaster (268) and Vengeance (581) retain master `fc2dbbabb6` results. Each uses its 300-second SimC trace, no healer, seeds 70448–70457, and 17 pressure scales. Cells show survivors/10 and mean time, counting survivors as 300 s. New reports and `merged-summary.json` are in `game-engine/data/diagnostics/healer-tank-replay/pressure-sweep-731ff1b3dc-66-104/`; the four older rows are in `pressure-sweep-fc2dbbabb6/`. The two master trees differ, so the combined ranking is not a same-tree comparison. The newer grid predates the subsequent Guardian two-piece and Sacred Weapon RPPM-clock fixes.

| Scale | 66 | 73 | 104 | 250 | 268 | 581 |
|---:|---:|---:|---:|---:|---:|---:|
| 1.00 | 10/10, 300 s | 10/10, 300 s | 10/10, 300 s | 10/10, 300 s | 10/10, 300 s | 10/10, 300 s |
| 1.25 | 8/10, 287 s | 10/10, 300 s | 10/10, 300 s | 10/10, 300 s | 10/10, 300 s | 10/10, 300 s |
| 1.50 | 1/10, 171 s | 10/10, 300 s | 10/10, 300 s | 10/10, 300 s | 10/10, 300 s | 10/10, 300 s |
| 1.75 | 0/10, 128 s | 10/10, 300 s | 10/10, 300 s | 10/10, 300 s | 10/10, 300 s | 10/10, 300 s |
| 2.00 | 0/10, 95 s | 10/10, 300 s | 8/10, 288 s | 10/10, 300 s | 10/10, 300 s | 10/10, 300 s |
| 2.25 | 0/10, 73 s | 10/10, 300 s | 0/10, 176 s | 10/10, 300 s | 10/10, 300 s | 10/10, 300 s |
| 2.50 | 0/10, 59 s | 10/10, 300 s | 0/10, 145 s | 10/10, 300 s | 10/10, 300 s | 10/10, 300 s |
| 2.75 | 0/10, 50 s | 10/10, 300 s | 0/10, 108 s | 10/10, 300 s | 9/10, 281 s | 10/10, 300 s |
| 3.00 | 0/10, 44 s | 10/10, 300 s | 0/10, 98 s | 10/10, 300 s | 8/10, 272 s | 10/10, 300 s |
| 3.25 | 0/10, 39 s | 10/10, 300 s | 0/10, 89 s | 9/10, 296 s | 5/10, 246 s | 10/10, 300 s |
| 3.50 | 0/10, 39 s | 10/10, 300 s | 0/10, 80 s | 8/10, 283 s | 2/10, 221 s | 10/10, 300 s |
| 3.75 | 0/10, 38 s | 10/10, 300 s | 0/10, 75 s | 2/10, 218 s | 5/10, 246 s | 9/10, 296 s |
| 4.00 | 0/10, 38 s | 10/10, 300 s | 0/10, 72 s | 0/10, 167 s | 1/10, 171 s | 8/10, 287 s |
| 4.25 | 0/10, 37 s | 10/10, 300 s | 0/10, 69 s | 3/10, 164 s | 0/10, 130 s | 7/10, 280 s |
| 4.50 | 0/10, 35 s | 9/10, 297 s | 0/10, 65 s | 0/10, 134 s | 0/10, 137 s | 6/10, 266 s |
| 4.75 | 0/10, 34 s | 6/10, 261 s | 0/10, 63 s | 0/10, 90 s | 0/10, 120 s | 4/10, 225 s |
| 5.00 | 0/10, 33 s | 2/10, 218 s | 0/10, 61 s | 0/10, 56 s | 0/10, 87 s | 3/10, 214 s |

Paladin, Warrior, Guardian and Vengeance survivor counts fall monotonically with scale. Blood (0/10 at 4.00, 3/10 at 4.25) and Brewmaster (2/10 at 3.50, 5/10 at 3.75) recover; their own procs change avoidance chances across scales. Against the earlier `fc2dbbabb6` grid, Paladin's survival scale rises from 1.00 to 1.25 and Guardian remains at 2.00. The trees contain multiple changes, so this comparison does not isolate individual fixes.

Ranking: the highest scale before the first scale with fewer than five survivors (0.25 resolution).

| Rank | Tank | Survival scale | Net damage/s at 1.0× |
|---:|---|---:|---:|
| 1 | Protection Warrior (73) | 4.75 | 5 |
| 2 | Vengeance Demon Hunter (581) | 4.50 | −58 |
| 3 | Blood Death Knight (250) | 3.50 | 0 |
| 4 | Brewmaster Monk (268) | 3.25 | 72 |
| 5 | Guardian Druid (104) | 2.00 | −433 |
| 6 | Protection Paladin (66) | 1.25 | 483 |

Ten seeds give a survivor-count standard deviation of about 1.6 near 50%, so adjacent ranks one grid step apart (Warrior and Vengeance; Blood and Brewmaster) are not reliably separated. Brewmaster's 3.25 depends on 5/10 at that scale exactly. Effective healing cannot exceed missing health, so a tank that stays at full health shows zero net damage; that column does not rank surviving tanks.

## Ten-seed healed replay versus corrected SimC

Measured on master `93a42f1f4a`: each tank's SimC trace, external dummy healing, native seeds 70448–70457. SimC is the mean of `collected_data.dtps` over ten measured iterations, after absorbs and including self-Stagger. `timeline_dmg_taken` sums run about 9% higher (consistent with 11 iterations divided by 10); do not use them. The corrected references remove SimC double applications and the Demon Spikes armor omission recorded in `healer-tank-replay/73.md`, `268.md` and `581.md`, and SimC's extra 100% Ursoc's Fury shield recorded in `104.md`. Reports and `SUMMARY.md` are in `game-engine/data/diagnostics/healer-tank-replay/all-10seeds-93a42f1f4a/`. Rows marked with a later commit were rerun after that tank's fixes; see its page.

| Tank | Native taken | SimC | Δ | Healed / unhealed deaths |
|---|---:|---:|---:|---:|
| 66 | 3,316,668 (`745943a0a`) | 3,249,867 (solo, `fake_solidarity=0`) | +2.1% | 0 / 0 |
| 73 | 3,010,673 | 2,951,770 (corrected) | +2.0% | 0 / 0 |
| 104 | 1,741,972 (`04110b594`) | 1,901,807 (corrected: Ursoc's Fury percentage) | −8.4% | 0 / 0 |
| 250 | 101,591 | 88,144 (canonical) | +15.3% | 0 / 0 |
| 268 | 3,465,194 | 3,471,015 (corrected) | −0.2% | 0 / 0 |
| 581 | 5,922,231 | 5,812,546 (corrected) | +1.9% | 0 / 0 |

Blood absorbs about 8.69M of 8.79M pre-absorb intake, so its 13k difference is 0.03% of raw intake and within its 54k–154k seed range; a percentage gate is not meaningful there. The Protection Paladin native replay has no Solidarity, so its reference is the solo SimC run; the default reference adds fictional allied effects. Its absorb drop from 3.13M to 2.85M between runs is random Bulwark-application variation, and the remaining gap was missing Lightsmith armament rules; see [Protection Paladin](healer-tank-replay/66.md#mitigation-decomposition-and-lightsmith-fix-2026-10-29). The Guardian reference corrects SimC's Ursoc's Fury shield size; see [Guardian](healer-tank-replay/104.md#ursocs-fury-counterfactual-reference).

### Party-buff composition remeasure (2026-11-02)

Old = SHA 1 `c7bd39f512` with the pinned traces and fixed-buff references; both carry all six raid buffs. New = branch `party-buffs-parity` with composition traces, where each tank holds only its own class buff; references are in `game-engine/data/diagnostics/party-buffs-composition-2026-11-02/tank/`. Healed replays, seeds 70448–70457. SimC is the mean `dtps` over ten measured iterations, with the corrected binary where the table above uses it. The old SimC column reruns the pinned options on that binary and reproduces the table above.

| Tank | Old native / SimC | Old Δ | New native / SimC | New Δ |
|---|---:|---:|---:|---:|
| 66 | 3,282,031 / 3,249,867 | +1.0% | 3,573,562 / 3,547,720 | +0.7% |
| 73 | 2,869,731 / 2,951,770 | −2.8% | 2,967,994 / 3,186,502 | −6.9% |
| 250 | 59,300 / 88,144 | −32.7% | 71,333 / 69,035 | +3.3% |
| 268 | 3,518,570 / 3,471,015 | +1.4% | 3,512,976 / 3,815,658 | −7.9% |
| 581 | 5,805,206 / 5,812,546 | −0.1% | 6,001,571 / 5,790,554 | +3.6% |

104 is blocked on both binaries: its top-character trace fails `parse_trace` with `invalid parry in SimC attack table … parry=-0.03`. After this change, 73 and 268 are outside ±5%. SimC takes 8.0% and 9.9% more damage without the lost buffs; native takes 3.4% and −0.2% more. Both are open for investigation and were not tuned. Blood's ±5% gate is not meaningful (see above).

Seven-healer party pulse, same inputs as the [recheck](healer-tank-replay/seven-healer-recheck-2026-11-02.md). Old = SHA 1, which applies the hardcoded Arcane Intellect and Mark of the Wild to the party. New = the party 66/healer/262/63/253 casts its own set before the pull: Arcane Intellect and Skyfury, plus Fortitude with a Priest healer. Difference is native/Holy against WCL/Holy.

| Healer | Old /Holy | Old diff | New /Holy | New diff |
|---|---:|---:|---:|---:|
| 256 | 1.0428 | +14.04% | 0.9896 | +8.22% |
| 264 | 0.9872 | +10.32% | 0.9650 | +7.84% |
| 65 | 1.0240 | +9.41% | 0.9857 | +5.31% |
| 1468 | 0.9852 | +13.74% | 0.9915 | +14.46% |
| 270 | 1.0500 | +18.34% | 1.0388 | +17.09% |
| 105 | 1.0639 | +17.13% | 0.9974 | +9.81% |

Holy Priest means are 34,088 (old) and 32,715 (new). Old SHA 1 values differ from the recheck table measured on `7cd700375a` (for example Discipline +14.04% vs +2.54%). The cause is not attributed. Reports: `game-engine/data/diagnostics/party-buffs-composition-2026-11-02/native/{old,new}/`.

### Midnight Season 2 boss-pressure calibration (2026-10-29; provisional)

Warcraft Logs extracts are in `game-engine/data/diagnostics/mplus-calibration-2026-10-29/`. They include report metadata and affixes, boss-pull lists, resource-bearing damage events, and selected Guardian buff streams. The current-season Altar of Fangs sample has one run each at +2, +4, +6, +8, +10, +12, +15, +22, +23 and +24; three bosses provide health observations in each run. Den of Nalorakk and Ruby Life Pools each have +22 and +24 runs. These are convenience samples, not population-weighted estimates.

For Rav'i, observed maximum HP at +2/+4/+6/+8/+10/+12/+15/+22/+23/+24 was 21,793,375 / 25,052,199 / 28,514,697 / 40,989,878 / 46,845,573 / 56,520,201 / 75,360,268 / 146,901,603 / 161,668,150 / 177,707,656. The +8 sample has Tyrannical; +10 and higher have both Fortified and Tyrannical. After dividing Tyrannical boss HP by 1.25, all three Altar bosses match the rounded factors 1.07, 1.23, 1.40, 1.61, 1.84, 2.22 and 2.96 through +15. These observations support the rounded factors and a 25% Tyrannical boss-health increase, not a direct measurement of Mythic 0. Affix IDs 9 and 10 appear together in every sampled +10 and higher run; there is no Fortified-only week at those levels.

The following uses normal auto-attacks against Guardian tanks on +2, with no Tyrannical affix. The median logged `unmitigatedAmount` is divided by 1.07 and by the SimC dungeon dummy's 250,000 melee. The ranges are individual logged hits, not confidence intervals. Buff streams are saved alongside the extracts; the sample includes defensive auras and does not isolate cooldown-free hits.

| Boss | Report / fight / pull | Hits | Logged hit range | Median | M0-normalized median / dummy |
|---|---|---:|---:|---:|---:|
| Writhing Coil | `zyRhmrY2NxnGgTPZ` / 1 / 8 | 9 | 260,419–280,615 | 274,277 | 1.025 |
| Zul'jan | `zyRhmrY2NxnGgTPZ` / 1 / 14 | 22 | 131,593–294,574 | 200,384.5 | 0.749 |
| Hoardmonger | `YcgyAmbfv1JLhq39` / 1 / 4 | 16 | 140,022–327,637 | 161,350.5 | 0.603 |
| Sentinel of Winter | `YcgyAmbfv1JLhq39` / 1 / 10 | 16 | 277,030–326,297 | 282,633.5 | 1.057 |
| Nalorakk | `YcgyAmbfv1JLhq39` / 1 / 14 | 8 | 316,949–365,884 | 326,249 | 1.220 |

The resulting 0.603–1.220 band is a spread of five boss medians, not an uncertainty interval for a universal boss. WCL's field may still include persistent tank reductions; if it does, the raw damage before those reductions is higher. The extracts do not establish which reductions WCL has already reversed, so do not divide the field by Thick Hide or versatility without additional proof. Tyrannical's damage factor is assumed to be 1.15, not independently isolated by this sample.

Applying each end of this proxy band to the survival scales gives the following **illustrative** highest keys: low boss pressure gives the upper end; high boss pressure gives the lower end. Fortified and Tyrannical differ only at +7–+9; at +10 and above both apply. A dash means the modeled +2 exceeds the survival scale, not that Mythic 0 is proven survivable.

| Rank | Tank | Fortified | Tyrannical |
|---:|---|---:|---:|
| 1= | Protection Warrior, Vengeance Demon Hunter | +13–+21 | +13–+21 |
| 3= | Blood Death Knight, Brewmaster Monk | +12–+19 | +12–+19 |
| 5 | Guardian Druid | +8–+14 | +6–+14 |
| 6 | Protection Paladin | below +2–+8 | below +2–+6 |

These are not validated real-key ceilings: the field is not proven attacker-side raw, the sample includes cooldowns and other modifiers, and the pressure grid scales dummy strikes and bleed ticks as well as melee. The 1.15 damage assumption and survival grid also carry their own uncertainty.

## SimC party-pressure trace

The second canonical healer input is `game-engine/data/diagnostics/healer-tank-replay/66.simc-party-pulse-seed70448.debug.log`. It adds a 20,000 Shadow pulse every 12 seconds from 8 seconds through 296 seconds. This is a reproducible synthetic dungeon-pressure scenario, not a measured dungeon encounter. SimC `76f8118` generates the five recipient source rolls with the MID2 Protection profile and four named copies; the native replay maps them to its tank, healer and three DPS followers. The original tank-only trace still supplies the tank's melee and bleed rolls. Each party pulse is a landed hostile magic hit for native damage-taken procs. Earlier party-pulse healer measurements lacked this hit classification and must not be compared as equivalent to corrected results.

Generate it with the pinned `engine/simc`, the MID2 Protection profile, `copy=party_healer copy=party_damage_1 copy=party_damage_2 copy=party_damage_3 tank_dummy=Tank_Dummy_Dungeon iterations=1 threads=1 max_time=300 fixed_time=1 vary_combat_length=0 seed=70448 debug=1 'raid_events+=/damage,first=8,cooldown=12,amount=20000,type=shadow'` and the canonical output path. The copied SimC actors supply event slots, not reference healer or DPS behavior. The healer replay CLI accepts `HEALER_TANK_TARGETS` for a fixed number of engaged dummies; the default is one. The first dummy supplies the recorded tank pressure, and the same raid debuffs apply to every dummy. Reports include damage per dummy and Discipline Pain/Atonement target-time receipts for enemy-count controls.

## How it works

- [Replay system](../wiki/systems/healer-tank-replay.md)

## Tank per-spell reporting

The SimC tank report includes raw and effective self-healing and outgoing periodic damage tick counts per spell. Its fixed-time target health uses the same encounter clock as the DPS parity fixture, so execute-phase abilities use the projected target health.

## Implementation inventory

- `crates/server-simulation/src/healer_tank_timeline.rs`: recorded incoming-hit timeline loader.
- `crates/server-simulation/src/simc_tank_timeline.rs`: SimC tank-dummy debug trace reader.
- `crates/server-combat/src/followers/simulator/healer_profiles.rs`: benchmark-only healer configuration and initial Holy Paladin decisions.
- `crates/server-simulation/src/dungeon_simulator/healer_metrics.rs` and `healer_replay.rs`: native healing receipts and recorded-timeline metrics.
- `crates/server-simulation/src/dungeon_simulator/holy_paladin_simc.rs`: Holy Paladin triage policy for the two-player SimC dummy replay; this is not a complete Holy spell policy.
- `crates/server-simulation/src/dungeon_simulator/tank_replay.rs`: isolated native tank replay and metrics.
- `scripts/healer_tank_replay.py`: SimC-dummy and native recorded-intake report tables.

## Tests asserting this spec

- `crates/server-simulation/src/healer_tank_timeline.rs`: concrete timeline loader test.
- `crates/server-simulation/src/simc_tank_timeline.rs`: SimC dummy raw damage trace test.
- `crates/server-simulation/src/dungeon_simulator/holy_paladin_replay_tests.rs`: native Holy Paladin replay of recorded incoming damage.
- `crates/server-simulation/src/dungeon_simulator/holy_paladin_simc_tests.rs`: MID2 Holy Paladin healing a native MID2 Protection tank on the SimC timeline.
- `crates/server-simulation/src/dungeon_simulator/tank_replay_tests.rs`: native Protection Paladin replay without a healer.
- `crates/server-simulation/src/dungeon_simulator/dps_parity/brewmaster_stagger_tests.rs`: Brewmaster incoming damage, Stagger ticks and Purifying Brew.
- `crates/server-simulation/src/dungeon_simulator/replay_cli_tests.rs`: CLI output from a native tank replay.

## Specializations

- [Protection Paladin (66)](healer-tank-replay/66.md)
- [Protection Warrior (73)](healer-tank-replay/73.md)
- [Discipline Priest (256)](healer-tank-replay/256.md)
- [Holy Priest (257)](healer-tank-replay/257.md)

## Known gaps (current cycle)

- [Blood Death Knight 250](healer-tank-replay/250.md): spec-specific replay notes and known reference differences.

- [x] Guardian Ursoc's Fury grants the native cumulative 372505 absorb from actual Thrash and Maul-family damage, including Thrash ticks, capped at 30% max health. Source: `sc_druid.cpp5148–5188,5600–5604,5912–5920`.
- [ ] Open gap: Bloodlust and the raid debuffs (Chaos Brand, Mystic Touch, Hunter's Mark, Mortal Wounds, bleeding) still come from `optimal_raid=1` in tank traces, and healer replay still applies lust, Chaos Brand and Mystic Touch to every party. They do not follow the composition. This is a separate decision.
- [ ] Connect SimC tank-dummy trace input to the healer replay.
- [ ] Register native tank APL adapters. Guardian 104 uses the pinned default Guardian APL and reports Ironfur, Frenzied Regeneration, Barkskin and Rage of the Sleeper as active mitigation. SimC tank replay requires a registered literal parity driver and explicit active-mitigation auras; unregistered tanks fail rather than use the smoke-test policy.
- [ ] Support replay durations other than 300 seconds with matching literal-APL encounter timing.
- [x] Publish native victim-side melee miss, dodge and parry proc events with their hit outcomes; explicit native or sourced script masks determine which auras can react.
- [x] Report adapter-registered named resource counters and target debuff uptime. Vengeance 581 records successful fragment generation, actual fragment consumption, and the owned Fiery Brand and Sigil of Flame debuffs.
- [x] Report attempted and effective tank Energize combat-log amounts by spell. Events do not identify the resource type; a spell that grants multiple resources combines them, and changes without an Energize event are not included.
- [x] SimC tank replay JSON reports precombat casts separately from encounter casts. It reports effective outgoing damage against the dummy by spell (spell 0 is an unlabelled attack), including owned summons, and paid ordinary cast resources by spell and resource type. Precombat damage, gains and costs are excluded, as are costs outside the ordinary cast payment boundary.
- [x] `--seeds N` runs consecutive native tank seeds starting at `--seed` (default 70448) against the same SimC trace and reports mean and min/max casts, aura uptimes and mitigation, plus death counts. Missing spell entries count as zero; unavailable mitigation is N/A. `--all` saves each tank seed's report separately and runs each healer once with seed 70448.
- [x] SimC tank JSON reports split damage taken into dummy swings, strikes, bleeds and self-periodic damage; dummy attacks also report outcome counts, damage before absorbs by kind and outcome, and mean defense stats sampled on attacks.
- [x] Keep no-healer tank replay as the default. An optional SimC dummy comparison applies the dummy's 75,000 external heal every five seconds and the trace's native Bloodlust at its recorded start time; it reports the buff, per-spell casts, effective self-healing, consumed absorbs and aura uptime separately.
- [ ] Extend coverage beyond the initial two specializations.

## Out of scope

- Deployment; this is a local simulation harness.
