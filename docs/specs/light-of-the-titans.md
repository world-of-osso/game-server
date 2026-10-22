# Light of the Titans

Selected native Protection66 talent378405, learned at41, produces the flagged
378412 HoT from actual Word of Glory85673 healing. Native70448 effect0/child
metadata supplies percentage and clock; SimC76f8118 `sc_paladin.cpp:1129–1177,
1251–1258` supplies the producer hook. Pinned TC a352b1fa paladin has no script.
See [aura execution](../wiki/systems/auras.md#native-rolling-periodic-refresh--2026-10-22).

## What it must do

- [ ] Successful ordinary Word of Glory healing with actual learned378405 produces owned378412 on the actual patient, including self healing.
- [ ] Native effect0 percentage of the resolved pre-overheal heal supplies the total, distributed over the child's actual duration/period; no copied coefficient or direct health mutation.
- [ ] Native effect1 bonus applies only to self Word while afflicted by actual harmful periodic health damage; do not borrow it for another patient.
- [ ] Actors without the learned talent and unrelated direct heals do not produce the HoT.
- [ ] Retain shared rolling carry, native resource costs/earning, tick phase and normal expiry; no duplicated spell-local pool.

## How it works

- [Shared carry](../wiki/systems/auras.md#native-rolling-periodic-refresh--2026-10-22).

## Implementation inventory

- `crates/server/src/auras/light_of_titans.rs`: native heal-event producer.
- `crates/server/src/auras/periodic.rs`: shared native fresh-HoT tick count.
- `crates/server/src/auras/refresh.rs`: shared flag-driven unpaid carry.

## Tests asserting this spec

- `crates/server/src/followers/kit_light_of_titans_tests.rs`: actual native Garrick, Judgment-earned Holy Power, ordinary WoG, level and hostile-DoT boundaries.

## Known gaps (current cycle)

- [ ] Qualify actual HoT amount/expiry, self-DoT enhancement and retained shared carry after the producer RED.

## Out of scope

Other flagged producers, AI policy, PvP amount scaling, canonical catalog edits and
simulator/live effectiveness. The SimC reference's legacy50%/NYI DoT bonus is not
native amount authority: local70448 effect1 is200%.
