# Frost Mage DPS parity

Verified: 2026-10-25. Spec64 uses the literal pinned76f8118 APL and native70448 spell values. Numerical parity is still open.

## Native ownership and damage boundaries

- Frostbolt228597, Flurry228354, Spike228600 and Ray ticks earn the caster's own1221389 Freezing. Native1246769 supplies ordinary counts; selected Frostbite adds its native count on critical Frostbolt/Spike impacts.
- Lance228598 pays native1246949 Shatter once, scaled by consumed stacks. Shatter effect0 selects the primary; selected Frostbite enables effect1 on secondary enemies only. Do not let the native area selector hit the primary again.
- Fingers/Thermal require launch snapshots, not a live aura query after missile travel. Ordinary root30455 and native damage228598 are separate records. Native Orb proc supply and snapshot retention have their own native process test.
- Controlled Instincts copies the final resolved Splinter hit using native444483's specialization fraction. It excludes the hit's primary, uses the native radius and pinned `bugs=1` hard cap, and does not roll another critical hit or reapply caster modifiers. `ResolvedDamage` supplies the real source amount; no damage-dispatch signature change is needed.
- Infused Splinters uses its native Frost chance/count on actual443722 impacts to earn owned Freezing.

## Proven landings

| Capability | Native proof | Landing |
|---|---|---|
| Owned Freezing and stack-paid Shatter, primary exclusion | RED72e1a35007/15da49910f → GREEN5d884bd519; nine Frost controls | de6efa82f |
| Controlled Instincts secondary copy and Infused Freezing | two RED9f79335000 → two GREEN2a2148ffbe; nine Frost controls retained | cd9e0e188 |

The second landing measured77,364.44 ST and105,096.15 three-target DPS (39.44%/35.20% of reference). Later shared changes require a fresh measurement before treating those numbers as current.

## Investigation traps and remaining work

- `dps_parity_gaps.py` matches spell IDs. SimC reports Frostbolt116, Ice Lance30455 and Glacial Spike199786 damage; native damage uses228597,228598 and228600. Read both reports before classifying those root IDs as missing damage.
- Shatter remains the largest actual missing channel after the two landings. Orb-earned Fingers/Brain Freeze and native Lance flight retention failed the real-cast test7d25e5c2dc. That fix is measurement-gated separately.
- Further source work includes Frostbolt/Spike/Ray/Orb-tick proc supply, Brain Freeze follow-ups, Splintering Orbs, Augury, Hand of Frost, Shatter tier Icicles and hit-side cooldown reductions. Do not replace these missing relationships with coefficient tuning.
- Shared flask and augment-rune bindings belong to the shared-consumables owner, not this investigation.

## Sources

- [Parity contract](../../specs/dps-parity.md) — native values, source relationships and literal fixture requirements.
- Pinned `sc_mage.cpp`3947/4055/4094–4095/4248/4304–4305/4324–4342/4445–4473/4995/5340–5410/7105–7181 — producer, consumer and copied-damage behavior.
- Train results/logs under `/home/osso/.worktrees/train/` — exact native proofs named above; private inputs remain outside tracked data.

## See Also

- [[simc-apl]] — literal fixture decisions and native facts.
- [[spell-script-crate]] — registered native spell-script boundaries.
