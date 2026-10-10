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
## How it works

- [Spell scripts and coverage boundaries](../wiki/systems/spell-casting.md#class-dummy-scripts)

## Implementation inventory

- `spell_cast/scripts.rs` — spell-ID hit, damage, crit, target-plan and cooldown hooks.
- `spell_cast/mod.rs`, `spell_cast/effects.rs` — validation, channel construction and normal effect execution.
- `auras/scripts.rs`, `auras/mod.rs` — periodic/removal callbacks and owned-duration extension.
- `stats.rs`, `auras/{handlers,stat_mods}.rs` — live stat-derived armor.
- `spell_cast/{dummy_tests,dummy2_tests}.rs` — real-ID casts and observable outcomes. Paths relative to `crates/server/src/`.

## Tests asserting this spec

- `crates/server/src/spell_cast/dummy_tests.rs` — `dummy_warrior_*`, `dummy_paladin_*`, `dummy_druid_*`, `dummy_priest_*`; Bloodthirst chance endpoints use explicitly labelled catalog clones, while the ordinary cast uses unchanged data.

- `crates/server/src/spell_cast/dummy2_tests.rs` — 24 native cast tests across nine class scripts, including tooltip level curves, conditional ranks, timer changes, cancellation/death and live armor changes.

## Known gaps (current cycle)

- [ ] Unknown Dummy fields remain unsupported; no generic no-op handlers. Source diagnostics list remaining marker and talent contracts.

## Out of scope

- Full talent/hero variants: Fresh Meat's guarantee, Enrage talent gating and Rampage's optional AoE branch are not certified by these base-script tests.
- Penance Atonement, Power of the Dark Side and Dark Reprimand variants are not certified by the base-channel tests.
- Second-batch optional/uncertified paths: Teachings/normalization, Find Weakness, Volatility, Unholy Vigor, Chain Heal additional-jump LOS/talent range, Calcified Spikes/Retaliation, Fire Blast parallel casting/Hot Streak.
- Realm UDP 5000, deployment and merging.
