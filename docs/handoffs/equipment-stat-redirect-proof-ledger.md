# Native equipment stat redirects

Verified2026-10-23: genuine native3RED11.02s at7aa72bf47; all ten non-redirected slot controls pass. Implementation GREEN/lead integration pending. [Contract](../specs/dps-parity.md#equipment).

## Boundary

Before the fix, profile `Equipment.options` retained `redirected_base_stats`, but `inputs::equip` discarded it when attaching native equipment/modifiers. Stats therefore used the original ItemSparse allocation. SimC76f8118 `item/item.cpp35–57` copies only donor stat types/allocations; original item identity, effects, quality/budget, armor/weapon metadata and socket multipliers remain original. Native70448 supplies donor values and the existing instance pipeline supplies the profile's level/bonuses. No reference stat-value override or catalog mutation is authorized.

Feral source artifact: engine `data/diagnostics/feral-parity-2026-10-23/per-slot-base-stat-diff.json`. Fifteen actual slots: five redirected tier slots differ; ten ordinary slots already match. Redirects: head271528→271875, shoulders271526→268246, chest271531→268235, hands271529→268234, legs271527→268225. Reported total difference native-minus-reference: Mastery−268, Versatility+168, Crit+157, Haste−57. The ordinary StatsPlugin regression independently reproduces these exact five differences; its other ten slots match.

## Regression scope

`equipment_input_tests` loads actual103 profile/unchanged native catalogs, equips each of fifteen base slots through shared `inputs::equip`, then advances the ordinary StatsPlugin. Enchant/gem contributions are explicitly removed in each in-memory test profile to isolate the base allocation measured by the source artifact; level, bonuses, item identities and option text remain exact. Concrete expected per-slot ratings come from the pinned reference artifact, not a production constant. Catalog and equipped-identity controls assert that redirection cannot replace native definitions or equipped IDs. Invalid native donor input must fail before mutation. A diagnostic regression requires this recognized option to leave ITEM_OPTION_UNMODELED without hiding other unknown options. Native3RED at7aa72bf47: recognized option still reported unsupported; malformed donor accepted; exact five slot differences retained. Build exit0/zero warnings27m56. Evidence engine `data/diagnostics/ww-parity-2026-10-23/equipment-redirect-red.{build.log,build-proof.json,output.log,proof.json}` records command, binary SHA and frozen sourceview. GREEN pending.

## Implementation boundary

Server-local `EquippedItemModifiers.base_stat_redirects` holds per-slot native donor IDs; `.items` and wire/persisted `ItemInstanceModifiers` remain unchanged. `inputs::equip` validates donors before inserting any equipment state; explicit0 is the pinned no-redirect sentinel. Stats copies the original template for calculation only, substitutes donor types/allocations before ordinary bonuses/item-level/enchant/gem processing, and applies the resulting stats with the original template/identity. Catalogs, socket multipliers, armor/weapon inputs, item effects and set membership remain original. Ordinary inventory callers retain empty redirect metadata. Only this recognized option leaves ITEM_OPTION_UNMODELED; other unknown fields remain explicit.

Runtime scope from the worktree:
```text
DPS_PARITY_OUTPUT=/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/feral-parity-2026-10-23
DPS_PARITY_WORLD_DB=/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/feral-parity-2026-10-23/world.db
DPS_PARITY_ACTOR=player DPS_PARITY_SPEC=103 TMPDIR=/var/tmp/osso
agent-run ww-monk-parity python3 scripts/dps_parity_native.py target/debug/deps/game_server-49562a93a94e52d1 dps_parity_equipment_redirected --include-ignored --nocapture --test-threads=1
```
No reference rating constants enter production code. Fifteen-slot base-stat coverage does not qualify every item effect, set producer or DPS channel.
