# Groups (party and raid)

Players form parties and raids with Retail rules. Messages are in shared-protocol
`protocol/group_messages.rs`; the roster is `GroupRosterSnapshot`.

## What it must do

- [x] Invite by name (`GroupInviteIntent`). The online target gets a `GroupInvitePrompt` (the `PARTY_INVITE` popup data) with a 60 s timeout. A timeout counts as a decline.
- [x] Accept or decline (`RespondGroupInvite`). A decline tells the inviter `ERR_DECLINE_GROUP_S`.
- [x] An ungrouped inviter has one invite out until the party forms (`ERR_INVITE_PARTY_BUSY`).
- [x] Only the leader invites, kicks (`GroupUninviteIntent`), promotes (`PromoteGroupLeader`), converts to raid (`ConvertGroupToRaid`), moves subgroups (`SetRaidSubgroup`), sets the loot method (`SetGroupLootMethod`) and starts ready checks.
- [x] Parties hold 5 members (`ERR_GROUP_FULL`). Raids hold 40 in 8 subgroups of 5. Moving into a full subgroup is ignored.
- [x] Leaving (`LeaveGroup`) passes leadership to the next member. A group left with one member disbands.
- [x] Members set their own role; the leader sets anyone's (`SetGroupRole`).
- [x] The loot method is stored and shown in the roster. Nothing uses it yet.
- [x] Every change sends the roster to all online members: name, class, level, online, subgroup (1–8), role, leader flag, entity id while in world. Former members get an empty roster.
- [x] Ready check (`StartReadyCheck`, `RespondReadyCheck`): 30 s, the initiator counts as ready, `ReadyCheckUpdate` goes to participants on start, on each answer and on finish. The roster carries ready/total counts.
- [x] Results and errors are `GroupCommandResponse` with a `GroupMessageCode` naming the Retail `GlobalStrings` key and its English text.
- [x] A group survives a member's relog. Members are character ids, and a relog resends the roster with the new entity id. The group disbands once no member is online. Groups are not saved to redb, so a server restart drops them.
- [x] Chat routing hooks: `GroupRegistry::is_in_same_group(a, b)` and `group_members(id)`, keyed by character id (`ClientId`).

## Implementation inventory

- `crates/server/src/group/registry.rs`: `GroupRegistry`, the pure rules.
- `crates/server/src/group/mod.rs`: `GroupPlugin` and the message system.

## Tests asserting this spec

- `crates/server/src/group/tests.rs`: loopback-connection tests for invite/accept, decline, ready check, leader leave and relog; registry tests for party cap, raid conversion and subgroups, leader-only commands, invite timeout, disband, roles and loot.

## Known gaps

- [ ] Raid assistants and main tank/assist flags.
- [ ] Converting a raid back to a party.
- [ ] Ignore list, faction and combat checks on invite.
- [ ] A leader leaving hands off to the next member even when that member is offline.
- [ ] The client does not yet send the new messages or show the invite popup.
