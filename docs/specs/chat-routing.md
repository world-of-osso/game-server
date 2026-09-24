# Chat routing

The server delivers each chat message only to players Retail would deliver it to.

## What it must do

- [x] Say and `/e` emotes (including social emote text): players within 25 yd of the speaker.
- [x] Yell: players within 300 yd in the speaker's zone.
- [x] Whisper: the named online target (case-insensitive) plus an echo to the sender.
- [x] Whisper to an offline or unknown name: sender gets system message `No player named '<name>' is currently playing.` (ERR_CHAT_PLAYER_NOT_FOUND_S).
- [x] Whisper from a sender the target ignores: dropped silently for the target; the sender gets only their normal echo, no error.
- [x] Party and raid: online members of the sender's group ([groups](groups.md)). No group: sender gets system message `You aren't in a party.` (ERR_NOT_IN_GROUP).
- [x] Guild: members of the sender's guild.
- [x] Every channel: recipients who ignore the sender do not receive it.
- [x] Client-sent `System`/`ServerBroadcast` messages are not delivered.
- [x] Chat history returns only messages the requester received, minus senders they ignore now.

## Integration points

- Party routing reads each player's `ClientId` (character id) and asks `group::GroupRegistry::is_in_same_group` / `group_members`. The registry is the only record of group membership.
- Players without `Position` hear only their own Say/Yell; players without `Zone` share a zone with each other.

## Implementation inventory

- `crates/server/src/messaging.rs`: `route_chat`, `route_and_record`, `history_for_player`.
- `crates/server/src/networking.rs`: `ChatHistoryBuffer`, `ChatHistoryEntry` (message + recipient names).

## Tests asserting this spec

- `crates/server/src/messaging.rs` tests: Say 20 yd vs 40 yd, Yell range/zone, whisper target + echo, offline whisper error, ignored whisper, party without group, party reaching group members but not another group or ungrouped players (real `GroupRegistry` built through invite/accept), history scope, history ignore filter.
