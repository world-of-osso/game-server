# Mount Journal

Approved2026-10-11. Retail Mainline behaviour/layout in both skins; Forever changes art only.

Collections→Mounts provides a searchable list; collected, mount-type and source filters; source/description text; rotating authentic creature-display preview; summon/dismount, account favourites and Summon Random Favorite Mount. Ownership and favourites survive logout/restart and are shared only by characters of the same account.

Mount identities, source spells, types/capabilities, displays and text come from Retail12.1.0.69933 Mount/MountXDisplay/MountCapability/MountTypeXCapability. Item acquisition comes from existing DB2 ItemEffect and read-only world.db vendor/quest data. Direct ITEM_SPELLTRIGGER_LEARN_SPELL teaches its authentic mount and consumes one item atomically; duplicate learning preserves the item. Normal quest/trainer LEARN_SPELL results grant account ownership. No fabricated starter mounts, IDs or assets.

Summoning uses the existing spell-cast, mount-aura and area/riding capability rules, not legacy display-ID mounting. Unowned/unusable mounts cannot be summoned or favourited. Random favourites select only owned usable favourites and use normal spell validation; no favourite yields an explicit refusal. Dismount cancels the real mount aura.

Reference: local Blizzard_Collections/Mainline/Blizzard_MountCollection.lua/.xml, TrinityCore a352b1fa mount-aura logic. Assets use local CASC only. Completion requires behavioural tests and privateUDP5901 fb_* proof in both skins, learning/summon/favourite/random/relogin, with inspected PNGs. Implementation/proof status belongs in the linked wiki, not this contract.
