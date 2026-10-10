# Class spell scripts

Spell-specific behaviours beyond generic Retail effects live in `crates/server/src/spell_cast/scripts.rs` and `crates/server/src/auras/scripts.rs`. Generic dispatch alone is not gameplay coverage. See [spell casting](../wiki/systems/spell-casting.md).

## What it must do

### First-batch defensive and hit bridges

- [x] Shield Block 2565 casts 132404 on the caster, granting the catalog block aura for six seconds.
- [x] Bloodthirst 23881 retains its damage and 80 raw Rage, heals through 117313 for 3% maximum health, and rolls effect 2's modified chance to apply Enrage 184362.
- [x] Rampage 184367 spends its catalog cost, retains the catalog attack triggers, and applies Enrage 184362.
- [x] Holy Shock 20473 casts 25912 at enemies or 25914 at allies (including self), using their catalog damage/heal formulas and retaining its one Holy Power gain.
- [x] Barkskin 22812's periodic Dummy casts Brambles damage 213709 every catalog tick when Brambles 203953 is present, hits enemies only, and stops on aura removal.
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
- [x] Butchery212436 with294029 reduces Wildfire Bomb259495's existing recharge by its authored3000ms per damaging hit, up to five hits; no rank or ready recharge means no change.
- [x] Wild Growth48438 applies its base periodic heal to at most five injured allies in the native target-centered area; healthy units do not consume slots.
- [x] Shadow Word: Death32379 applies its authored5% caster-max-health backlash if the target survives the damage, not on a kill.
- [x] Starfall191034 with327541 extends owned Moonfire164812/Sunfire164815 by three seconds, capped at28 seconds remaining, preserving existing tick clocks and foreign marks.

## How it works

- [Spell scripts and coverage boundaries](../wiki/systems/spell-casting.md#class-dummy-scripts)

## Implementation inventory

- `spell_cast/scripts.rs` — spell-ID hit, damage, crit, target-plan and cooldown hooks.
- `spell_cast/mod.rs`, `spell_cast/effects.rs` — validation, channel construction and normal effect execution.
- `auras/scripts.rs`, `auras/mod.rs` — periodic/removal callbacks and owned-duration extension.
- `stats.rs`, `auras/{handlers,stat_mods}.rs` — live stat-derived armor.
- `spell_cast/scripts/{batch3,batch4,batch5,batch6}.rs` — locally sourced bounded paths.
- `spell_cast/{dummy_tests,dummy2_tests,dummy3_tests,dummy4_tests,dummy5_tests,dummy6_tests}.rs` — real-ID casts and observable outcomes. Paths relative to `crates/server/src/`.

## Tests asserting this spec

- `crates/server/src/spell_cast/dummy_tests.rs` — `dummy_warrior_*`, `dummy_paladin_*`, `dummy_druid_*`, `dummy_priest_*`; Bloodthirst chance endpoints use explicitly labelled catalog clones, while the ordinary cast uses unchanged data.

- `crates/server/src/spell_cast/dummy2_tests.rs` — 24 native cast tests across nine class scripts, including tooltip level curves, conditional ranks, timer changes, cancellation/death and live armor changes.

- `crates/server/src/spell_cast/dummy3_tests.rs` — 16 per-spell cast tests, chance endpoints, radius/threshold/condition boundaries and Garrote silence immunity. [Census and surviving proof](../wiki/systems/spell-casting.md#third-batch-local-retail-rows).

- `crates/server/src/spell_cast/dummy4_tests.rs` —17 cast tests across ten paths, including passive/bleed/ownership gates, strict threshold, area cap/radius, ticking/expiry and existing-timer-only updates. [Census and proof](../wiki/systems/spell-casting.md#fourth-batch-local-retail-rows).

- `crates/server/src/spell_cast/dummy5_tests.rs` —14 per-spell authentic catalog cast functions with chance/ownership/rank/cap gates. Full spell_cast207/207 at`f6b842a`; [census and proof](../wiki/systems/spell-casting.md#fifth-batch-local-retail-rows).

- `crates/server/src/spell_cast/dummy6_tests.rs` — per-spell real catalog casts for the nine bounded paths; [scope and current proof](../wiki/systems/spell-casting.md#sixth-batch-local-retail-rows).

## Known gaps (current cycle)

- [ ] Unknown Dummy fields remain unsupported; no generic no-op handlers. Source diagnostics list remaining marker and talent contracts.

## Out of scope

- Full talent/hero variants: Fresh Meat's guarantee, Enrage talent gating and Rampage's optional AoE branch are not certified by these base-script tests.
- Penance Atonement, Power of the Dark Side and Dark Reprimand variants are not certified by the base-channel tests.
- Second-batch optional/uncertified paths: Teachings/normalization, Find Weakness, Volatility, Unholy Vigor, Chain Heal additional-jump LOS/talent range, Calcified Spikes/Retaliation, Fire Blast parallel casting/Hot Streak.
- Third-batch unimplemented variants: Improved Garrote windows/other concealment spells; Prayer selection injury priority/LOS beyond ordinary cast validation; Prayer-to-Serenity optional1246517; Shred stealth/Thrash variants; Thunder Clap reduced damage beyond five targets/root variant; Moonfire second target, damage reduction and Astral Power variants. Unknown root/descendant markers are not certified by base-path proof.
- Fourth-batch limits: unknown Dummy metadata remains unresolved. Steady Shot Marksmanship cooldown/level curve, Kill Shot enabling talents, Mangle marker, Renewing Mist overheal jumps/rank231606 procs, Vivify tooltip multiplier/talent variants, Chain Lightning Maelstrom Weapon/target-count marker, Howling Blast secondary damage/CAP/Frost Fever resource procs, Sanctify target cap/extra marker, Ebon Might stat distribution and extension caps are not certified.
- Fifth-batch limits: unidentified Demon's Bite/Disintegrate/Chaos Bolt markers; Chaos Strike's30 mirror is not separately consumed; Shuriken soft-cap/stealth/talent fields; Bone Shield consumption/haste/armor parity and Blood Plague leech; latent plague talent effects. Crash Lightning remains deferred because its native selector104 is unsupported. No alternate cone substituted. These base-path tests do not certify full class rotations.
- Sixth-batch limits: unknown root/child markers; Starfire/Keg/Butchery reduced-damage curves beyond their soft caps; Starfire429523 variant; Keg Shuffle/Brew reduction; Wild Growth declining tick curve/Tree of Life targets; Shadow Word: Death execute/damage-effect variants; Ice Lance Freezing/shatter; Starfall overlap/base tick child contracts. Selector104 remains unsupported: no Retail selector definition derived from the allowed local sources. No cone substitute or scope-wide rotation claim.
- Realm UDP 5000, deployment and merging.
