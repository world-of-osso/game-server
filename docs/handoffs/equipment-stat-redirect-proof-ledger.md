# Native equipment stat redirects

Source verified2026-10-23; native regression runtime RED pending. [Contract](../specs/dps-parity.md#equipment).

## Boundary

Profile `Equipment.options` retains `redirected_base_stats`, but `inputs::equip` currently discards it when attaching native equipment/modifiers. Stats therefore uses the original ItemSparse allocation. SimC76f8118 `item/item.cpp35–57` copies only donor stat types/allocations; original item identity, effects, quality/budget, armor/weapon metadata and socket multipliers remain original. Native70448 supplies donor values and the existing instance pipeline supplies the profile's level/bonuses. No reference stat-value override or catalog mutation is authorized.

Feral source artifact: engine `data/diagnostics/feral-parity-2026-10-23/per-slot-base-stat-diff.json`. Fifteen actual slots: five redirected tier slots differ; ten ordinary slots already match. Redirects: head271528→271875, shoulders271526→268246, chest271531→268235, hands271529→268234, legs271527→268225. Reported total difference native-minus-reference: Mastery−268, Versatility+168, Crit+157, Haste−57. This source census is not a substitute for the new runtime regression.

## Regression scope

`equipment_input_tests` loads actual103 profile/unchanged native catalogs, equips each of fifteen base slots through shared `inputs::equip`, then advances the ordinary StatsPlugin. Enchant/gem contributions are explicitly removed in each in-memory test profile to isolate the base allocation measured by the source artifact; level, bonuses, item identities and option text remain exact. Concrete expected per-slot ratings come from the pinned reference artifact, not a production constant. Catalog and equipped-identity controls assert that redirection cannot replace native definitions or equipped IDs. Invalid native donor input must fail before mutation. A diagnostic regression requires this recognized option to leave ITEM_OPTION_UNMODELED without hiding other unknown options. Runtime RED/GREEN pending; no production redirect implementation yet.

Shared wire/persisted `ItemInstanceModifiers` ABI remains outside scope. The intended runtime input belongs to equipped stat context, not a replacement item identity. Existing unmodified inventory/equipment callers retain empty redirect metadata.
