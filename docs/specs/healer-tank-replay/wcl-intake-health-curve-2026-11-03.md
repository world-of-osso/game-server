# WCL intake health-curve check

The profile and post-mitigation intake use Altar of Fangs report `pPFDbkfQK7MACtmY` fight 41 pull 1. Damage pressure is `(amount + absorbed) / recorded victim max HP × native recipient max HP`. Native shields and hostile-hit procs apply, but armor and damage-taken multipliers are not applied again. The role mapping is WCL tank 6, healer 221 and DPS 1/225/231 to the native tank, healer and three DPS slots. The source damage is replayed on an engaged dummy, not the original NPC. Self-sourced WCL rows are excluded; the original tank is a Brewmaster while the native tank is a Protection Paladin. This is **INFERRED_WCL**, not a matched health curve.

A level-90 Preservation seed 1 over 75 seconds applied 160 positive hits; 160 later positive hits found their targets dead, and 154 recorded zero-damage outcomes were not replayed because their native miss types are not decoded. The tank died at 14.55 s. At the 42 pre-death tank hits with WCL HP observations, mean absolute post-hit HP difference was 45.35 percentage points. The last observed post-hit HP at selected times was:

| Pull time | WCL tank HP | Native tank HP |
|---:|---:|---:|
| 0.994 s | 96.80% | 100.00% |
| 2.644 s | 79.99% | 54.88% |
| 7.451 s | 97.29% | 16.24% |
| 9.979 s | 87.67% | 64.32% |
| 14.521 s | 76.43% | 0.00% |

By 14.55 s, 45 landed WCL hostile hits delivered 389.69% of original tank max HP before shields. WCL absorbed 317.76 percentage points and lost 71.93; native cumulative HP loss was 338.86 percentage points. Different shields and healing are material. These findings do not establish the class-specific cause of the difference, and spell-share comparison is not qualified. Full per-hit samples and run log: `/home/osso/.worktrees/healer-multi-enemy-evidence-v2/wcl-normalized-1468-1.json` and `.log`.
