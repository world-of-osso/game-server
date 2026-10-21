# Source-backed conversations

Generic conversation actors, lines and lifetimes from pinned TDB1210 and native70448 DB2 data, with Orator's authored ingress scripts as the first consumer. Source: TC a352b1fa `Conversation.cpp`, `ConversationDataStore.cpp` and `boss_orator_krix_vizk.cpp`. Server runtime/payload is gated first; protocol and Godot TalkingHead presentation follow, not dropped.

## What it must do

### Source ingress

- [x] Import source conversation templates, actors and line templates plus70448 ConversationLine/BroadcastTextDuration into an explicitly owned copy, without changing other tables or canonical data. Preserve receipt authentication/unreadable-record facts; never promote available rows into tablewide authentication claims.
- [ ] Preserve actual24642→67085,24643→67091,24644→67092 and authored actor51642/display116692 identities; no invented dialogue, actor aliases or placements. Fail explicitly for unavailable required definitions.

### Server runtime

- [ ] Create source actor/line payloads in the creator's exact copy, with private-owner isolation and no extra recipient/copy delivery.
- [ ] Use authored line order, duration contributions and TC's10-second post-line object lifetime; do not guess missing voice durations or phase clocks.
- [ ] Actual static ingress spawns266–268 on2669 run the source Player/non-GM checks, use that instance's Orator216619, create the private conversation at the entrant's position and remove the source trigger as authored.

### Protocol and presentation

- [ ] Add the approved conversation message on a separate shared-protocol branch from03cebdb; retain source actor/line identities and private ownership.
- [ ] Godot renders the authored70448 Retail TalkingHead template for both skins. No substitute chat window, invented layout or extra controls.
- [ ] Provide native1920×1080 PNGs for both skins from the actual conversation payload consumer. Protocol/server/engine are jointly gated by the lead.

## How it works

- [Conversations](../wiki/systems/conversations.md).
- [World-static ingress](../wiki/systems/world-area-triggers.md).

## Implementation inventory

- `scripts/conversation_data.py`: pinned local source reads, owned-copy import, receipt-preserving report and replayable SQL publication artifact.

## Tests asserting this spec

- `scripts/tests/test_conversation_data.py`: actual native actor/line records and scoped SQLite writes.

## Known gaps (current cycle)

- [ ] Server runtime, owner payload, source lifecycle and Orator ingress.
- [ ] Approved protocol and detached-engine TalkingHead presentation slice after the server gate.

## Out of scope

Fabricated source records, source-independent timing and canonical publication by a child agent. Missing source data remains a named boundary, not a replacement value.
