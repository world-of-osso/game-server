# Voluntary guild leadership transfer

The authenticated current Guild Master may transfer to another stored same-guild member, including an offline member. TC a352b1fa Guild.cpp1531–1576 supplies the normal transfer behavior; [guild lifecycle](../wiki/systems/guild-lifecycle.md) owns implementation details.

## What it must do

- [x] Recheck authenticated actor/current guild/current master and target membership; reject nonleader, foreign or absent targets without changes.
- [x] Atomically make the target the sole rank0 member and move the former master to the current lowest rank. Preserve other members, rank definitions, bank state and existing lifecycle history.
- [x] State survives redb reopen. New-master settings authority and former-master restrictions apply immediately; the former master may then leave normally, while the current master still cannot leave.
- [x] Existing real request/reply delivery refreshes both participants and does not grant authority to unauthenticated requests.
- [x] `GUILD_SELF_LEADER_TRANSFER_REFUSED_UNSOURCED`: refuse self-target with existing Permissions result and unchanged state. This is approved project policy, not Retail self-transfer parity.

## How it works

- [Guild lifecycle](../wiki/systems/guild-lifecycle.md).

## Implementation inventory

- `crates/server/src/guild_ranks.rs`: authenticated transfer transform and existing recipient refresh.
- `crates/server/src/persistence_banks.rs`: existing atomic member-rank persistence.
- `crates/server/src/guild_leadership_transfer_tests.rs`: real persistence, refusal, HostClient and concurrent-reader regressions, enabled after actual authority RED→GREEN.

## Tests asserting this spec

- `guild_leadership_transfer_moves_authority_and_reopens_before_former_master_leaves`.
- `guild_leadership_transfer_refuses_nonleader_foreign_missing_and_policy_self_without_writes`.
- `guild_leadership_transfer_host_replies_to_both_members_and_rechecks_auth_and_new_authority`.
- `guild_leadership_transfer_readers_never_observe_zero_or_two_masters_during_real_commits`.

## Known gaps (current cycle)

- [ ] Coordinated consumer gate and real two-member native authority readback remain pending. Actual authority RED4 precedes focused GREEN4/4 (six-rank/nonzero bank fixture, reopen/leave, auth/foreign/self, participant refresh and at least1000 real reader snapshots, continuing until the31-transfer writer finishes).

## Out of scope

Disband, inactivity dethroning, authenticator eligibility, rename and rank reordering. Retail Lua hides self in the context menu but forwards raw slash text to an unavailable native C API. TC's same-pointer normal path can overwrite the current master's rank after `_SetLeader`; it is source evidence, not a Retail runtime oracle and is not emulated.
