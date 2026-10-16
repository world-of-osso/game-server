# Mail

Player mail at Mailbox game objects, following Retail with AzerothCore rules (`MailHandler.cpp`, `Mail.cpp`). Auction House results arrive as mail in the same mailbox. Messages use `MailChannel` (shared-protocol `protocol/mail_messages.rs`). The client frame is specified in game-engine `docs/specs/mail-frame.md`; for how it works see [mail](../wiki/systems/mail.md).

## What it must do

### Mailboxes
- [x] Mailboxes (`content_gameobject` rows of type 19 on map 0 with a Retail `GameObjectDisplayInfo` box, 56 of them) spawn beside the Guild Vaults.
- [x] `UseGameObject` on a mailbox opens `NpcRole::Mailbox` (alive, within 5 yd of the model box) and sends `MailboxContents`: delivered mail, newest first. Every later change to the character's mail sends it again while the mailbox is open.
- [x] Requests name the open mailbox; any other object answers `NotInteracting`.
- [x] A creature with source `NpcFlags::MAILBOX`, including Katy's GOSSIP|MAILBOX, opens the existing Mailbox role directly through ordinary live-NPC/map/range validation; no fabricated gossip choice. Same send/read/take requests and persistence apply. Death, range loss or despawn closes service without spending postage on rejected requests (TC `WorldSession::CanOpenMailBox`).

### Sending
- [x] Postage is 30c per attached item, 30c for a letter; the sender pays postage plus the money sent (`NotEnoughMoney`).
- [x] At most 12 attachments (`TooManyAttachments`), each a bag item of the sender, once (`InvalidAttachment`), not soulbound or a quest item (`BoundItem`).
- [x] Money and C.O.D. are exclusive (`Internal`); C.O.D. without items is dropped.
- [x] The recipient is found by name, case-insensitively (`RecipientNotFound`); not yourself (`ToSelf`), not the other faction (`WrongFaction`, AzerothCore `AllowTwoSide.Interaction.Mail` 0); a recipient already holding more than 100 mails is full (`RecipientMailboxFull`, TrinityCore `mailsCount > 100`).
- [x] Items to a character on another account arrive after 3600 s (AzerothCore `MailDeliveryDelay`) unless both are in the same guild (TrinityCore `HandleSendMail`); everything else arrives at once. `game-server-admin mail-deliver-now <character>` delivers mail in transit now.
- [x] Mail expires 30 days after delivery, C.O.D. mail after 3 days.
- [x] Subject and body are cut to 64 and 500 letters.
- [x] Removing the items, paying, and delivering happen in one database transaction.

### Inbox
- [x] Opening a mail marks it read.
- [x] Taking an item needs bag space (`InventoryFull`). The first item taken pays the C.O.D. (`CodInsufficientMoney`), which is mailed to the sender as a C.O.D. payment.
- [x] Taking money is refused past the gold cap (`TooMuchGold`).
- [x] Player mail that is not returned and not a C.O.D. payment is returnable: Return sends it back once with its items and money. Returnable mail still holding items or money cannot be deleted (`CannotDelete`); anything else can.
- [x] Returned items going to a character on another account wait the 3600 s delivery delay, like a send (TrinityCore `MailDraft::SendReturnToSender`, `Mail.cpp:168-192`); money and letters return at once.
- [x] Expired mail goes back to its sender when returnable and not empty, otherwise it is deleted with its items.
- [x] `PendingMail` lists the senders of the newest three unread delivered mails; it is sent on entering the world and whenever that list changes.

### Auction House
- [x] Auction sale, win, cancel, expiry and outbid mail is instant Auction House mail. Auction mail from before player mail is moved into the mailboxes at startup.

## Gaps
- No mail from NPCs with the `MAILBOX` npc flag, no stationery, no reply-to thread, no "Open All" server shortcut.
- Cross-faction mail follows the player default (TrinityCore grants `RBAC_PERM_TWO_SIDE_INTERACTION_MAIL` only to moderators); the TrinityCore exception for all-account-bound attachments is not modelled, nor are its conjured / expiring item refusals (item templates carry no such flags).
