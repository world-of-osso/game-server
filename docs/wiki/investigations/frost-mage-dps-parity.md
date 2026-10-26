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
| Orb-earned procs, Lance flight retention, cast supply, Splintering Orbs/Augury, Hand/tier/Ray | RED7d25e5c2dc/ba38b7e5e0/7e193c9c90 → all18native cases GREENbdb2e07889 | 6ecb05c789 |

The third landing measured117,989.23 ST and167,793.59 three-target DPS (60.15%/56.19% of reference). Later shared changes require a fresh measurement before treating those numbers as current.

## Investigation traps and remaining work

- `dps_parity_gaps.py` matches spell IDs. SimC reports Frostbolt116, Ice Lance30455 and Glacial Spike199786 damage; native damage uses228597,228598 and228600. Read both reports before classifying those root IDs as missing damage.
- After the third landing, actual remaining gaps rank ST Shatter23.6k, Splinter16.3k, Spike10.3k; three-target Shatter42.0k, Controlled Instincts20.0k, Orb18.7k and Splinter16.7k. The raw spell-ID report must be reconciled as above.
- Native2RED193e699b7e proves both missing Deep Shatter critical amplification (plain/selected17,865) and missing post-consumption Brain Freeze Flurry amplification (plain/earned59,618). Their implementations remain separately GREEN/measurement-gated.
- Native RED0a917790e7 proves missing paid-Lance Glaciate reduction: native4stacks expected0.4s, observed0s. Its running-charge test uses `full_recharge_time`, not admission `remains`, which is zero while another charge is usable.
- Remaining unqualified relationships include Orb-tick Fingers and source/data modifier completeness. Do not replace missing relationships with coefficient tuning.
- Shared flask and augment-rune bindings belong to the shared-consumables owner, not this investigation.

## Sources

- [Parity contract](../../specs/dps-parity.md) — native values, source relationships and literal fixture requirements.
- Pinned `sc_mage.cpp`3947/4055/4094–4095/4248/4304–4305/4324–4342/4445–4473/4995/5340–5410/7105–7181 — producer, consumer and copied-damage behavior.
- Train results/logs under `/home/osso/.worktrees/train/` — exact native proofs named above; private inputs remain outside tracked data.

## See Also

- [[simc-apl]] — literal fixture decisions and native facts.
- [[spell-script-crate]] — registered native spell-script boundaries.
