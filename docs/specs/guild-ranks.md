# Player-managed guild ranks

## Contract

- Guild Master alone creates, removes, renames, reorders and configures ranks. There
  are 2–10 ranks, ordered highest first. Rank 0 is immutable. Names are nonempty,
  at most 15 characters. Occupied ranks cannot be removed.
- Reordering preserves each member's rank identity. Removing empty ranks renumbers
  lower positions. Adding ranks does not move members with an implicit lowest rank.
  Rank list and member positions commit in one redb transaction.
- Promote/demote requires its right and a lower-ranked target in the same guild.
  Promotion cannot reach the actor's rank; rank 0 cannot be promoted/demoted.
- Rank flags govern guild/officer chat listening and speaking, invitations, MOTD,
  guild information and officer-note editing/viewing. Remove-member rights are
  represented for the future removal feature; no player removal request exists yet.
- Bank view/deposit and stacks/day are per rank/tab. Zero means no withdrawals.
  Only rank 0 has unlimited access. Gold withdrawal and repair share copper/day
  allowance. Existing bank action validation reloads the current rank on every
  request; logs and contents omit hidden tabs. Rank edits refresh open vaults.
- Only authenticated in-world connections can request controls. The actor is taken
  from the connection, not client data. Success broadcasts recipient-specific ranks;
  errors are typed and rendered with Retail GlobalStrings.
- redb layout 2 adds social rights. Layouts 0 and 1 migrate once without losing
  existing bank balances, tabs, limits, logs or repair permissions.

## Retail references

Local root: `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/`.

- `Blizzard_GuildControlUI/Blizzard_GuildControlUI.lua:4,536–620`: max 10, minimum 2,
  deletion disabled for occupied ranks and shifts excluding Guild Master.
- `Blizzard_GuildControlUI/Blizzard_GuildControlUI.xml:7`: rank names, 15 letters.
- `Blizzard_GuildControlUI/Blizzard_GuildControlUI.lua:137–238,420–459`: tab
  view/deposit/stack allowances, rank flags and gold/day enabled for repair/withdraw.
- `Blizzard_Communities/GuildRoster.lua:137–141`: strict promote/demote hierarchy.
- `game-engine/data/GlobalStrings.csv`: `ERR_GUILD_PERMISSIONS`,
  `ERR_GUILD_RANK_IN_USE`, `ERR_GUILD_RANK_TOO_HIGH_S`, `ERR_GUILD_RANK_TOO_LOW_S`.

## Behavioral proof

`guild_ranks_*` tests cover GM-only changes, rank count bounds, occupied removal,
immutable rank zero, hierarchy/flags, atomic member remapping, restart round-trip,
layout-1 migration, actual player rank packets followed by bank withdrawals and hidden
logs, daily item/gold refusal, and guild/officer chat delivery/history permissions.
Existing guild bank and merchant repair tests cover shared repair/gold spending.
