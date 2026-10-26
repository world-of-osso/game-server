# Ret Divine Purpose and tier supply

Divine Purpose supplies free foreground spenders; MID2 Arbiter uses are a separate consequence of consuming it. Native data supplies amounts, chances and cooldowns.

## Producers and consumers

`ret_purpose.rs` rolls the native selected talent chance for Holy Power consumers. Empyrean Legacy's background Storm can produce fresh Purpose, but cannot consume or classify the existing buff.

`ret_aurora.rs` grants native408458 after selected Aurora439760's ordinary Wake execution and the Dawnlight budget grant. The grant uses the native ICD without another chance roll. Aurora owns that ICD; broad DUMMY hints cannot spend it on unrelated events.

`ret_tier.rs` reads the actual consumed modifier snapshot, applies Divine Power and arms an Arbiter kind only when no kind is pending. A different-kind foreground spender, or the sourced Legacy exception, consumes the pending use and dispatches native1306923.

## Qualification

Native REDb584a946f8 observed no Purpose after Wake. GREENc768cfaee3 proves ordinary Wake→free Verdict→armed use→real Storm-triggered Arbiter damage. Its three-seed supply probe produced36/33/34 successful payouts, with no expired/lost Arbiter uses. Native setup starts with zero Purpose/Arbiter uses, matching the pinned first-action state. Initial full gate `d841480cd6` passed 110 cases and exposed five fixture failures: four paid Dawnlight controls still selected Aurora, and the Player reset control treated queued requests as executions. Paid controls now leave Aurora legally unspent; the Player control waits for native admission before observing resets. Selected-Aurora positive coverage remains unchanged, with a separate unselected-Wake control. Final landing `455a8ad19` via `837a09a9bf` passed all 116 Ret/Dawnlight cases. Landed three-seed follower means are 159690.45 ST and 300626.02 three-target, or 89.683%/93.552% of the pinned references. That Aurora-only checkpoint remained below the 90% ST boundary. The later [HoW qualification](spell-casting.md#how-primary-impact-extension--2026-10-26) records completed Ret numerical acceptance.

The passive test observer accepts selected aura IDs; applications, charge gains and consumption are measured at native lifecycle boundaries. It does not grant buffs or change rolls. At the fixed 300s cutoff, consumed Arbiter uses must reconcile with actual terminal outcomes plus native hits observed in flight before target teardown. The fixed-time fixture sets its target health to zero before the final update, so reading the travel queue only after that update can miss impacts discarded for changed alive state. The observer retains the last live-target pending count and records actual native launch/due/discard times and recipients; mid-fight target death is an error. Outcome accounting reads every native Damage/Miss event, not only the presentation report's Miss/Dodge/Parry counters, and independently checks actual payload launches. The diagnostic does not drain projectiles beyond the measured window or assume that every consumed use has already landed.

## Sources

- [Contract](../../specs/dps-parity.md#ret-aurora-producer).
- [Aurora](../../../crates/server-spell-scripts/src/modules/ret_aurora.rs), [Purpose](../../../crates/server-spell-scripts/src/modules/ret_purpose.rs), [tier](../../../crates/server-spell-scripts/src/modules/ret_tier.rs).
- [Native supply probe](../../../crates/server-simulation/src/dungeon_simulator/dps_parity/ret_arbiter_supply_tests.rs).
- Pinned SimC76f8118 `sc_paladin_retribution.cpp878–881`, `sc_paladin.cpp73–74`, `sc_paladin.hpp1665–1727`.

## See Also

- [[ret-judgment]] — target damage amplification is separate from spender supply.
- [[spell-script-crate]] — downstream registration.
