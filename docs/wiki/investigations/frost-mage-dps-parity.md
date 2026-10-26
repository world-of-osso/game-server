# Frost Mage DPS parity

Verified: 2026-10-26. Spec64 uses the literal pinned76f8118 APL and native70448 spell values. Numerical parity is still open.

## Native ownership and damage boundaries

- Frostbolt228597, Flurry228354, Spike228600 and Ray ticks earn the caster's own1221389 Freezing. Native1246769 supplies ordinary counts; selected Frostbite adds its native count on critical Frostbolt/Spike impacts.
- Lance228598 pays native1246949 Shatter once, scaled by consumed stacks. Shatter effect0 selects the primary; selected Frostbite enables effect1 on secondary enemies only. Do not let the native area selector hit the primary again.
- Fingers/Thermal require launch snapshots, not a live aura query after missile travel. Ordinary root30455 and native damage228598 are separate records. Native Orb proc supply and snapshot retention have their own native process test.
- Controlled Instincts copies the final resolved Splinter hit using native444483's specialization fraction. It excludes the hit's primary, uses the native radius and pinned `bugs=1` hard cap, and does not roll another critical hit or reapply caster modifiers. `ResolvedDamage` supplies the real source amount; no damage-dispatch signature change is needed.
- Infused Splinters uses its native Frost chance/count on actual443722 impacts to earn owned Freezing.
- Rapid Refreezing accrues five native200ms Icicle ticks during its1s window. Only Glacial Spike attempts the pinned accumulated proc curve; the broad native proc hint must not grant it on paid Lance/Shatter.
- Overflowing Energy eligibility belongs to actual Frostbolt228597 and Spike228600 positive direct hits. Schedule native394195 outcomes through the existing merged buff queue; do not grant or remove stacks on Lance. Native RED1b8be9a776 proves both missing gain and expiration; GREENd6ef48c03d qualifies the Frost-only callback and Lance exclusion. The initial landing was blocked by an unrelated Arcane fixture control; after its fixture-only fix,208adb8ca9 landed the same behavior with all controls retained.

## Proven landings

| Capability | Native proof | Landing |
|---|---|---|
| Owned Freezing and stack-paid Shatter, primary exclusion | RED72e1a35007/15da49910f → GREEN5d884bd519; nine Frost controls | de6efa82f |
| Controlled Instincts secondary copy and Infused Freezing | two RED9f79335000 → two GREEN2a2148ffbe; nine Frost controls retained | cd9e0e188 |
| Orb-earned procs, Lance flight retention, cast supply, Splintering Orbs/Augury, Hand/tier/Ray | RED7d25e5c2dc/ba38b7e5e0/7e193c9c90 → all18native cases GREENbdb2e07889 | 6ecb05c789 |
| Deep Shatter critical modifier and post-consumption Brain Freeze damage | GREENe5816f0565;20native Frost/CI cases and15costed controls | cfec0a955 |
| Paid-Lance Glaciate/Force of Will and once-per-Orb-pulse Fingers | GREEN896a8e1556;22native Frost/CI cases and15costed controls | 4aba2bff2 |
| Rapid Refreezing native tick clock and qualified Spike-only source | RED8455eb05e5/351688fab3 → GREENf19726f012; costed controls retained | ca856c659 |
| Ray execution-time10s Splinterstorm window; Frost Overflowing Energy feedback | RED1f25b0f870/1b8be9a776 → GREEN208adb8ca9;27Frost/2CI/15costed controls | ca48fe12e |

Latest confirmed measurement, from train208adb8ca9:153,092.12 ST and232,077.22 three-target DPS (78.0463%/77.7192% of reference). Against Rapid checkpoint152,167.13/232,622.16, the combined-source change is+924.99ST/−544.94three-target DPS; this does not isolate either fix. The ±10% goal remains open. Remeasure after the separately owned shared precombat equipment-proc gate and ordinary category133 spell-GCD haste fixes land.

## Investigation traps and remaining work

- `dps_parity_gaps.py` matches spell IDs. SimC reports Frostbolt116, Ice Lance30455 and Glacial Spike199786 damage; native damage uses228597,228598 and228600. Read both reports before classifying those root IDs as missing damage.
- After the third landing, actual remaining gaps rank ST Shatter23.6k, Splinter16.3k, Spike10.3k; three-target Shatter42.0k, Controlled Instincts20.0k, Orb18.7k and Splinter16.7k. The raw spell-ID report must be reconciled as above.
- Deep Shatter and post-consumption Brain Freeze are now qualified above. Their original2RED193e699b7e compared plain/selected critical17,865 and plain/earned Flurry59,618.
- The Glaciate test uses `full_recharge_time`, not admission `remains`, which is zero while another charge is usable. Native modified Shatter cap is6, not raw4: selected Heart of Ice/Polished Focus modifiers make the qualified reduction0.6s and Force of Will count3Splinters.
- Orb-tick Fingers and Overflowing Energy are now qualified above. Shared precombat equipment-proc admission and ordinary spell-GCD haste are being fixed by Survival and Arcane owners; do not duplicate them or replace missing relationships with coefficient tuning.
- Requested periodic-deadline probe5ecd5357cb on6f842e121 ran25GREEN controls but failed the two unimplemented Overflowing Energy assertions. The train skipped its measurement; there is no ST/three-target delta and no isolated proof of a periodic-rule regression.
- Shared flask and augment-rune bindings belong to the shared-consumables owner, not this investigation.

## Sources

- [Parity contract](../../specs/dps-parity.md) — native values, source relationships and literal fixture requirements.
- Pinned `sc_mage.cpp`3947/4055/4094–4095/4248/4304–4305/4324–4342/4445–4473/4995/5340–5410/7105–7181 — producer, consumer and copied-damage behavior.
- Train results/logs under `/home/osso/.worktrees/train/` — exact native proofs named above; private inputs remain outside tracked data.

## See Also

- [[simc-apl]] — literal fixture decisions and native facts.
- [[spell-script-crate]] — registered native spell-script boundaries.
