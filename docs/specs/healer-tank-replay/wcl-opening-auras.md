# WCL opening auras

Matched replay input combines each actor's CombatantInfo auras with party-targeted buffs whose first in-pull event is a removal, stack removal or refresh. Each entry records its WCL source, target, spell, stack count and evidence. Native replay applies these at its first-combat boundary. A removal does not prove expiration rather than consumption, so recorded events do not establish the remaining duration; native duration starts fresh. Unmapped sources and spells that cannot be applied must fail explicitly, except the unmodeled cosmetic Visage form 372014, which is logged and skipped.


## Geometry limitation

The matched enemy profile records only first/last observed damage intervals. It does not retain WCL actor/instance identities or positions. Native replay places active enemies at training-dummy positions inside the healer's AoE radius. The resulting stacked geometry can inflate SCK damage and damage-driven healing; intake-gate success does not qualify outgoing damage, healing or cast-mix parity.

WCL resource-bearing enemy hits can carry target x/y positions, and player casts can carry caster x/y positions. These are sparse observations, not continuous movement. Replaying their spread requires identity-preserving enemy intervals, map-aware coordinates and synchronized party/enemy position samples. Moving enemies alone into WCL coordinates while keeping the party at the fixture origin would break range checks. This geometry work remains a gap; no invented spread is applied.

## Replayed tank casts

Matched replay input also lists the tank's self-applied Vampiric Blood (55233) `applybuff` times inside the measured interval, with evidence `applybuff`. Native replay holds the spell on cooldown from its first-combat boundary, so the tank's APL cannot choose it. At each logged time the replay clears the hold, casts it through the normal follower execution path, and holds it again. A tank that died earlier than in the log skips its later logged casts, because deaths are outcomes; any other refused cast fails the replay. The healer replay then measures healer fidelity against the logged tank timing; tank cooldown fidelity is measured separately by the tank ranking. Reason: [Blood opener timing](250.md#matched-vampiric-blood-timing-gap).

## Preservation Merithra timing

**INFERRED_WCL:** In five current-build Altar pulls, 35 observed 1256579 applications followed by a same-source 1256581 consuming cast have a median delay of 11.602 s and nearest-rank p90 of 27.589 s. Reports and fight/pull: `vhKYp3J2BcVdQ7fa` 2/1, `b64JANPjvFap98Kf` 11/1, `caf6pzLP3nNr7CMv` 3/1, `1gKbWZAyzT7JqBn6` 4/1 and `6wC7gFdD9avRAWVL` 17/1. Opening unknown-age procs and incomplete intervals are excluded; refreshes do not restart the clock. Casts count only when the same-source removal occurs within 5 ms.

Before opening-aura replay, five native seeds of `vhKYp3J2BcVdQ7fa` 2/1 held the proc for 27.90, 15.20, 31.85, 27.65 and 28.05 s: median 27.90 s and nearest-rank p90 31.85 s. This is one pull with five random seeds, and party deaths affect later choices. The native policy is slower in this sample, but the comparison does not establish an unconditional proc-first rule: the logged healer cast Dream Breath at 16.193 s while holding the proc from 6.095 s until 24.207 s. The Preservation APL remains unchanged. Raw WCL and native traces: `/home/osso/.worktrees/healer-merithra-wcl/`.
