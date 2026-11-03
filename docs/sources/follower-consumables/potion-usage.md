# INFERRED_WCL combat potion usage

Player-key sample and source caveats: see `derivation.md`. Events are actual cast events for the bounded combat-potion ID list. A potion is attributed to a dungeon pull when its timestamp falls inside that pull; an event up to 10 seconds before a pull is attributed to that next pull. No hit-count, fight-pressure or decision trigger is inferred. First-pull counts include a potion at most 10 s before pull 1.

| Spec | Players N | Potion users | First observed potion on pull 1 / users | Boss casts | Trash casts | Other casts | Median repeat gap (seconds; interval N) | Example citations |
|---|---:|---:|---:|---:|---:|---:|---|---|
| death-knight-blood | 56 | 32 | 27/32 | 33 | 70 | 0 | 358.6; 71 | `NWqhPZbdJDGzkKpx/47:11`, `VWhXtxrPjBnYfpKc/2:7` |
| death-knight-frost | 17 | 12 | 11/12 | 12 | 24 | 0 | 369.9; 24 | `kn6rXN2ZcWJDmKf9/9:17`, `jzQf4TrFVxPYpcBK/1:5` |
| death-knight-unholy | 10 | 9 | 8/9 | 7 | 21 | 0 | 370.8; 19 | `dyL9BFJzXwkAVxNj/1:4`, `FXapfQnWwC1MBL7z/21:247` |
| demon-hunter-devourer | 18 | 10 | 8/10 | 11 | 18 | 0 | 335.2; 19 | `FB3fmbP4AjW1y7RC/10:1`, `N41fXWMAaLTczd3R/21:1240` |
| demon-hunter-havoc | 30 | 21 | 19/21 | 33 | 45 | 0 | 323.3; 57 | `VWhXtxrPjBnYfpKc/2:6`, `jzQf4TrFVxPYpcBK/1:1` |
| demon-hunter-vengeance | 11 | 8 | 5/8 | 6 | 16 | 0 | 346.6; 14 | `dyL9BFJzXwkAVxNj/1:1`, `cTAk9n2zgyV1JwQF/2:4` |
| druid-balance | 19 | 12 | 11/12 | 8 | 29 | 0 | 389.2; 25 | `ywdN6H2M4FKZnCJX/1:1`, `2f94DFGnbCwvzjNV/6:232` |
| druid-feral | 10 | 7 | 6/7 | 12 | 16 | 0 | 321.1; 21 | `Mq7DTBL2vb8Wh4H9/51:257`, `7DYPh21VGpB6W8zK/22:291` |
| druid-guardian | 61 | 43 | 39/43 | 44 | 104 | 0 | 353.7; 105 | `jzQf4TrFVxPYpcBK/1:2`, `6cWwTXnvQagz8tNb/12:368` |
| druid-restoration | 18 | 1 | 1/1 | 1 | 3 | 0 | 327.9; 3 | `GjapMz1RQKrZCDLx/4:5` |
| evoker-augmentation | 8 | 6 | 5/6 | 4 | 13 | 0 | 361.3; 11 | `kzvd1XpNrZYAQftP/5:1`, `1Fr6vhkTMaYB9NX8/7:88` |
| evoker-devastation | 11 | 7 | 5/7 | 11 | 13 | 0 | 340.5; 17 | `j9m1aJRLZvAkzQ78/7:2`, `Y9DFKHNbrz6gvmBL/2:46` |
| evoker-preservation | 14 | 3 | 2/3 | 4 | 7 | 0 | 415.9; 8 | `KRHCbx2ykW8LPTAm/2:2`, `FqzTptJrnP3ba8f6/8:1` |
| hunter-beast-mastery | 32 | 18 | 14/18 | 18 | 31 | 0 | 359.8; 31 | `vGDBNHkg8F9WTphX/5:1`, `HQ8gR6fqpXKn471a/1:1` |
| hunter-marksmanship | 10 | 5 | 5/5 | 7 | 11 | 0 | 350.6; 13 | `BCVnH7ZbWrv1QPXL/32:970`, `BpWD1VvHxzX8G3Tw/59:14` |
| hunter-survival | 8 | 6 | 6/6 | 11 | 13 | 0 | 348.1; 18 | `KRHCbx2ykW8LPTAm/2:4`, `vPaRfhCpxQB8XZVm/3:162` |
| mage-arcane | 60 | 36 | 32/36 | 47 | 82 | 0 | 344.9; 93 | `VWhXtxrPjBnYfpKc/2:43`, `dmfg1yLqp4PQB6WK/21:902` |
| mage-fire | 6 | 4 | 4/4 | 4 | 10 | 0 | 340.9; 10 | `FB3fmbP4AjW1y7RC/10:132`, `Byz9kPWN7D4RFQX6/4:2` |
| mage-frost | 8 | 5 | 5/5 | 6 | 8 | 0 | 331.4; 9 | `C3j4F6HLhAfDnNg7/6:152`, `K3wDQ1n7dFfpHTkN/70:1315` |
| monk-brewmaster | 15 | 8 | 8/8 | 13 | 19 | 0 | 345.4; 24 | `KGnTvgtcDX2BQMk7/22:6`, `Qr1b9kK7mA8qMLNT/1:6` |
| monk-mistweaver | 18 | 9 | 6/9 | 9 | 19 | 0 | 328.0; 19 | `9C6PwNFG81Bh7DyK/37:250`, `GVT6rBFfJ7bDQc2M/9:8` |
| monk-windwalker | 19 | 11 | 9/11 | 20 | 24 | 0 | 349.5; 33 | `R8wmvkfqAy3rYjxz/18:804`, `ZjrNtmAFHCbx1Rh6/4:424` |
| paladin-holy | 51 | 25 | 20/25 | 22 | 52 | 0 | 334.6; 49 | `NWqhPZbdJDGzkKpx/47:1214`, `dmfg1yLqp4PQB6WK/21:903` |
| paladin-protection | 36 | 15 | 13/15 | 21 | 28 | 0 | 328.7; 34 | `kn6rXN2ZcWJDmKf9/9:14`, `vGDBNHkg8F9WTphX/5:529` |
| paladin-retribution | 45 | 21 | 19/21 | 30 | 46 | 0 | 345.6; 55 | `1tk7vFWNbMcYVpGa/1:1`, `ZGHAPM7aXqT2bgfN/17:244` |
| priest-discipline | 11 | 6 | 4/6 | 7 | 8 | 1 | 437.1; 10 | `tDzYZ9RxLvVyCGB4/6:321`, `GajRZCmD2AhtFrvk/2:3` |
| priest-holy | 32 | 11 | 7/11 | 6 | 18 | 0 | 377.2; 13 | `kn6rXN2ZcWJDmKf9/9:13`, `1tk7vFWNbMcYVpGa/1:3` |
| priest-shadow | 19 | 11 | 9/11 | 11 | 21 | 0 | 390.8; 21 | `R8wmvkfqAy3rYjxz/18:535`, `tYApvmfCkH9rVLJ1/1:3` |
| rogue-assassination | 16 | 11 | 10/11 | 12 | 24 | 0 | 390.3; 25 | `drh37pC6cYK4Hy2v/1:2`, `8MphRJ1jaV9YN4wP/12:1044` |
| rogue-outlaw | 10 | 7 | 7/7 | 10 | 18 | 0 | 309.4; 21 | `KGnTvgtcDX2BQMk7/22:2`, `Hzna6hPd8x2ZCXFq/2:2` |
| rogue-subtlety | 29 | 25 | 23/25 | 36 | 52 | 0 | 327.3; 63 | `NWqhPZbdJDGzkKpx/47:1218`, `9C6PwNFG81Bh7DyK/37:640` |
| shaman-elemental | 55 | 33 | 31/33 | 48 | 75 | 0 | 386.3; 90 | `VWhXtxrPjBnYfpKc/2:8`, `dmfg1yLqp4PQB6WK/21:4` |
| shaman-enhancement | 11 | 8 | 7/8 | 10 | 12 | 0 | 389.6; 14 | `DYVqG6Hw3bf4TRz2/3:2`, `y2RpPAKND9hjVFxL/5:4` |
| shaman-restoration | 47 | 18 | 13/18 | 23 | 28 | 0 | 327.9; 33 | `4pYDX8BxzCgVMFJ9/1:2`, `ZGHAPM7aXqT2bgfN/17:242` |
| warlock-affliction | 10 | 6 | 6/6 | 6 | 13 | 0 | 365.4; 13 | `FxWQAhNGqjyVv2ZX/4:208`, `wR2jgBAtKXb8hpGD/1:2` |
| warlock-demonology | 26 | 14 | 13/14 | 16 | 29 | 0 | 357.4; 31 | `4pYDX8BxzCgVMFJ9/1:3`, `kzvd1XpNrZYAQftP/5:4` |
| warlock-destruction | 10 | 4 | 3/4 | 7 | 6 | 0 | 347.7; 9 | `Gmfpzak48AHv1VxR/4:5`, `H1rGgR3xymTqDza7/1:8` |
| warrior-arms | 66 | 52 | 48/52 | 89 | 121 | 0 | 331.7; 158 | `dmfg1yLqp4PQB6WK/21:900`, `6cWwTXnvQagz8tNb/12:367` |
| warrior-fury | 9 | 5 | 4/5 | 4 | 13 | 0 | 331.1; 12 | `y2RpPAKND9hjVFxL/5:708`, `dTG619Q2Ffp8nByV/2:114` |
| warrior-protection | 13 | 9 | 8/9 | 10 | 19 | 0 | 334.7; 20 | `YkLPaZ1bAX7NvVjH/6:391`, `X6ZVNpFYCkwWy4nQ/2:6` |

Overall: 1869 casts from 554/955 player-keys; first potion on first pull 481/554; inside/before trash 1179, inside/before boss 689, outside both 1.

The 10-second pre-pull association is an analysis window, not a recommendation. Potion events do not reveal whether users selected the next opportunity based on boss identity, active damage windows, pull size, cooldown availability or another input.
