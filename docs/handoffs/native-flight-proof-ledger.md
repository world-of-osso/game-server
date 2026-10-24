# Native action flight — scoped proof

Runtime7826726dc0a7c4b1e07b9844b0173d416b352176; parent gateff1073 owns final integration. No DPS parity claim.

## Capability matrix

| Boundary | Behavior | Proof |
|---|---|---|
| Pending native missiles | Actor/spell-owned earliest real due; no cast-start timer | Pyro native launch/countdown/impact and foreign-actor exclusion |
| Hardcast | Not flight before launch | Ordinary native Pyro hardcast control |
| TC Meteor153561 | Live177345 expiry plus native153564 destination delay; then actual admitted missile | Native3sAT→1smissile→impact, no flight during ongoing burn |
| Failed expiry admission | Ends flight immediately when ordinary153564 is refused | Actual Fireball overlaps3s expiry; no invented4s timer |
| Other scripted chains | Not qualified by this slice | Explicitly no broad claim |

`missiles` shares the existing native speed/launch/min-duration/delay-seconds calculation between real launch and the Meteor continuation reader; no numeric travel policy changes, timer resource or executor. `NativeFacts` binds `action.<alias>.in_flight` and `.in_flight_remains`; remaining uses the nearest pending due, zero without flight.

## RED / GREEN

At3e3cacb6d, current-canonical source-view `--lib` tests: startup1PASS/4genuine unknown-property assertionRED9.65s; compile19m12, zero warnings. Both requested properties evaluated before failure. Earlier binary target was rejected by the maintained library-only runner, not RED.

At7826726dc:14/14GREEN23.04s, compile14m21, zero compiler warnings/exit0. Four native flight lifecycles plus startup inode equality and nine existing missile regressions: distance, instant, launch/landing combat, moving/dead/despawned targets, bouncy/chain travel and reflection. Runtime/test files hash-match compiled native root682b0eeeea6ae5f1bbea-45f25f5c0625.

Engine evidence `data/diagnostics/rotation-fidelity-2026-10-21/`: `native-flight-782-gate-packet.json`, `flight-{3e3-lead-lib-red,782-lead-lib-green}.log`, `world-flight-current-20261024T041837Z.receipt.json`. Fresh CURRENT canonical SQLite backup: canonicalSHA256215d75ec02c0311831cf5d98a4432c985500345811471fcce6ed23fee7709557 stable/noWAL; privateSHAcd7230ac1d62bf7d7ea8fef5fbb82faa9f36b61ec78108e591f1204c614a8ed6. Maintained native runner, innerTMPDIR/osso, authorizedlead_fire14779, one-j2job; no bypass.

[Contract](../specs/simc-apl.md#native-flight-observations); [shared native facts](../wiki/systems/simc-apl.md).
