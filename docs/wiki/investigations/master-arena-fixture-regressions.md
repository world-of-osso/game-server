# Master arena/class fixture regressions

Verified: 2026-10-13. Three named tests passed on pre-arena master `f2b6d92b` and failed on integrated `74c2274a`. The recent Stealth-child, Prowl-level and live-ground commits were not the shared root cause. Only Arcane Power needed a production change; Moloch and mastery needed faithful test setup.

## Arcane Power: duplicate cost consumer

Git bisect identifies `057e7136`: generic school/power-mask aura72 begins applying native12042 effect1's −30%. That commit needs the compile-only accessibility/default-value corrections from `9430bc2c` to run its unchanged test; the bootstrap patch and both logs are retained. `dc5f0b40` passes; `9430bc2c` fails70 versus the old unranked100 expectation.

TC `SpellInfo.cpp:4185–4194` applies matching aura72 without checking343208. The earlier named rank consumer was both an invented extra gate and a second multiplier: ranked cost became49 rather than70. Remove that duplicate consumer; assert100/70/70 for absent12042, active12042 without rank, and active12042 with rank. The corrected native regression was genuinely RED at49 before deletion. Keep native expiry and Arcane-charge cost controls.

## Moloch: reciprocal invisibility is not a triggered exemption

Git bisect identifies `0852d806` against passing parent `9430bc2c`. Native55964 gives type4/9999 invisibility for2500ms. The source schedules its55966 attempt2000ms later, while the caster remains invisible. `SpellDefines.h:293–301` defines FULL_MASK as0x0007FFFF; IGNORE_TARGET_CHECK is0x00100000 and is **not** included. `Spell.cpp:5959` checks the explicit unit; `Object.cpp:1003–1005` requires that unit's reciprocal detection flags, unless the caster has aura335.

Do not invent a triggered exemption or remove invisibility early. The success fixture applies native23101 Eye of Divinity to the player: type4/1000 supplies reciprocal flags but cannot reveal9999-point Vanish. Every original threshold, emote, invisibility, stun, unattackability, cast-count and behind-position assertion remains. A second plain-player control asserts failed native admission, no scripted retry, then successful four-yard teleport after expiry. Observe that instant teleport before resumed pursuit moves the boss.

This is a source/data limitation for ordinary players, not a claim that the encounter now always teleports them.

## Mastery: capped area selection is not explicit self selection

The original expiry fixture co-located owner and foreign player and cast413984 with an explicit owner. Its native targets18/31 are an area selector with MaxTargets1, not a forced self target. Instrumentation on74c2274a selected foreign354v0 instead of owner351v0: foreign aura present, owner aura absent. The unwrap failed before any refresh/expiry assertion. TC `Spell.cpp:1445–1450` randomly caps ordinary area targets; it also gives no explicit-owner guarantee.

Apply both native recipient auras at the existing production aura boundary, as the foreign half already did. Preserve every duration, stack, decreasing-clock and dual-expiry assertion. Neither production target selection nor mastery refresh changes.

Historical bisection is qualified: the original mastery test was absent at the early arena revisions; historical revisions also had unrelated schema/build gaps. With lead approval, current-master source-slice probes falsified both ExcludeCaster and stealth-component reconciliation as individually sufficient causes. Native413984 Attributes1 is zero. The original test is RED at450acdd8 as well as74c2274a, GREEN atf2b6d92b. No unique first archetype-order-changing commit is claimed; the reproduced cap/tie dependency is the root cause, not an expiry defect.

## Sources

- Pinned raw single-file TC a352b1fa fetches: `SpellInfo.cpp`, `Spell.cpp`, `SpellDefines.h`, `Object.cpp`, `boss_randolph_moloch.cpp`; retained under `game-engine/data/diagnostics/master-red-fix-2026-10-12/reference/`.
- Native imported `spell_effect`, `spell_misc`, `spell_target_restrictions` rows in the read-only Skyborn2026-10-07 world copy.
- Bisect logs, compile-bootstrap patch, source-slice counterexamples, instrumented selection and RED/GREEN logs in the same diagnostics directory.
- [Spell contracts](../../specs/spell-dummy-scripts.md), [boss contract](../../specs/boss-scripts.md), [mastery proof limits](../../class-completeness2.md#summon-owner-damage-and-raw-mastery--batch18).

## See Also

- [[spell-casting]] — native cost and aura behavior.
- [[boss-scripts]] — source event timing and visibility limitation.
- [[auras]] — reciprocal invisibility detection.
