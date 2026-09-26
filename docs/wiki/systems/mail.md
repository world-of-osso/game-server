# Mail

How the server implements mail. The contract is in [mail spec](../../specs/mail.md).

## Storage (redb, bitcode)
`persistence_mail.rs` (`persistence::mail_store`):

| Table | Key → value |
|---|---|
| `character_mail` | character_id → `Vec<MailData>` (delivered and in transit) |

- `MailData { mail_id, sender: Player{character_id, name} | AuctionHouse, subject, body, items: Vec<MailItemData{slot, item}>, money, cod, delivers_at, expires_at, read, returned, cod_payment }`.
- Attachments keep their fixed `slot`, so taking one leaves the others' indices alone.
- Items keep their guid in the mail, so `item_instances` follows them.
- The legacy `mailboxes` table (auction-only `u32` mail) is moved into `character_mail` as Auction House mail at startup and dropped.

## Operations (`mail.rs`)
- `send_mail`, `act_on_mail` (MarkRead / TakeAttachment / TakeMoney / Return / Delete), `expire_mail` and `deliver_pending_mail` are plain functions over the database; tests call them directly or through the bank test harness.
- Each writes the character's `InventoryRecord` and the affected mailboxes in one write transaction (`write_inventory_record`, `write_mailbox`, `push_mail`). Mail ids and item guids are minted before it, because redb allows one writer.
- Results come back as `MailChanges { gold, mailboxes }`: gold goes to `GoldUpdateQueue` (`Gold` + `InventoryDelta`), mailboxes to `ChangedMailboxes`.

## Systems (Update, chained)
1. `send_opened_mailboxes`: `Changed<ActiveInteraction>` with `NpcRole::Mailbox` sends `MailboxContents`.
2. `process_send_requests`, `process_mail_requests`: refused with `NotInteracting` unless the request names the open mailbox.
3. `tick_mail` (once a second): expiry sweep, then per connection the `PendingMail` senders (sent when they change) and the delivered count (a change resends an open mailbox, so delayed mail shows up).
4. `send_changed_mailboxes`: resends the inbox to open mailboxes of changed characters.

## Auction House
`auction_house::operations::deliver_mail` builds an instant `MailSender::AuctionHouse` mail. The old `QueryAuctionMailbox` / `ClaimAuctionMail` path is gone; auction results are taken at a mailbox.
