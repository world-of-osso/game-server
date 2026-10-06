# Group roster portrait transport

## Contract

`GroupMemberSnapshot.portrait: GroupPortraitAppearance` contains:

- `race: u8`.
- `appearance: CharacterAppearance`: `sex`, `skin_color`, `face`, `eye_color`, `hair_style`, `hair_color`, `facial_style`, `customization_choices` (`option_id`, `choice_id`), and `visage`.
- `head: Option<EquippedAppearanceEntry>`: the Head entry's `slot`, `item_id`, `display_info_id`, `inventory_type`, and `hidden`. `None` means no head item; a hidden entry retains its explicit visibility flag.

Canonical appearance is retained rather than inventing a second list of customization selectors: arbitrary race-specific option IDs can affect head geosets/textures, and the server has no client customization database to classify them. No torso, shoulder, cloak, or weapon equipment is transported.

The server reads login-loaded `Player` and authoritative `EquipmentAppearance`. Inventory's existing `send_inventory_deltas` updates that component on equipment changes. The group registry retains the last observed portrait in its character-ID profile, including while the entity/connection is absent. Every membership/presence roster includes it. Appearance changes invalidate the last reported online state and send the existing reliable GroupChannel roster once; unchanged state and non-head equipment do not invalidate it. No appearance is added to periodic `GroupMemberStates`.

The online view is not filtered by Position, Zone, InMap, or replication interest. All map copies currently share one ECS World, so members across zones use the same registry. Groups remain in-memory and disband when everyone is offline; this feature does not add restart persistence.

## Renderer evidence and helm scope

Read-only game-engine `godot/rust/src/unit_portraits.rs::sync_party_portraits` currently discovers appearance only through replicated Player entities and `world.unit_appearance`. `world_models.rs::UnitAppearance::Player(Player, EquipmentAppearance)` passes canonical customization and equipment to the existing detached player visual loader. `unit_portraits.rs::Portrait::sync` rebuilds on changed appearance. That renderer does not remove head equipment before loading. This transport therefore preserves equipped helm appearance, including hidden/transmog display state, without shipping unrelated equipment.

Retail source evidence already recorded in game-engine `docs/wiki/systems/portrait-party-frames.md` and `data/diagnostics/party3-2026-10-05/proof.md`: UnitFrame.lua:175–189,199–200 uses `SetPortraitTexture`/`UNIT_PORTRAIT_UPDATE`; PartyMemberFrame.lua:570–585 desaturates disconnected portraits. Lua delegates actual head/helm composition to the native API; it does not document native helm filtering. Independent Retail helmet pixel parity is not established by these sources or the server tests.

## Exact deferred client hookup

1. Retain `GroupMemberSnapshot.portrait` in `godot/ui-model/src/group_state.rs::GroupState.members` when consuming `GroupRosterSnapshot`; do not drop it on offline live-state cleanup.
2. In `godot/rust/src/unit_portraits.rs::GameClient::sync_party_portraits` (currently line 568), replace the `self.replica.units()` appearance-map construction with `self.account.group.members.iter()`. For each roster member build:

```rust
UnitAppearance::Player(
    Player {
        name: member.name.clone(),
        race: member.portrait.race,
        class: member.class,
        appearance: member.portrait.appearance.clone(),
    },
    EquipmentAppearance {
        entries: member.portrait.head.iter().cloned().collect(),
    },
)
```

Key by `member.name` and pass that map to the existing `PartyPortraits::sync` appearance closure. No replicated entity, previous replication, or `member.online` condition is needed. The existing equality/rebuild path refreshes changed heads, and existing party tint/desaturation remains independent of appearance. Update roster fixture literals for the new required field. `unit_portraits/party.rs::PartyPortrait` can stop describing roster appearance as unavailable.

Client source is unchanged in this task. Spec: shared-protocol `docs/specs/group-roster-portraits.md`. Behavioral proof: server `group::tests::roster_portrait_*` and shared `protocol_group_tests::roster_portrait_round_trips_on_the_wire`.

The server fixtures cover three-member delivery across maps at `(50000, 20000, 100)`, customization changes after a map transfer without rejoining, display/hidden/removal changes, and retention of the latest changed helm for an offline member and a new recipient. `roster_portrait_persisted_helm_equip_reaches_far_member` loads concrete redb character data and sends an actual `SwapItem` request for head item 32329: inventory refreshes `EquipmentAppearance` in PostUpdate, the next Update sends exactly one roster to the distant member, and the physical equipment is persisted. The protocol round trip includes non-empty Dracthyr visage selectors and both hidden-head and absent-head payloads.
