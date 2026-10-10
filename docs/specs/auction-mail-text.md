# Retail auction mail text

Retail Mainline owns subjects and invoice wording. The native client formats mail; the server never invents English auction letters.

## Contract

- Auction House subject strings carry `GLOBALSTRINGS_TAG:item name`. Supported tags: `AUCTION_SOLD_MAIL_SUBJECT`, `AUCTION_WON_MAIL_SUBJECT`, `AUCTION_EXPIRED_MAIL_SUBJECT`, `AUCTION_OUTBID_MAIL_SUBJECT`, `AUCTION_REMOVED_MAIL_SUBJECT`.
- Sold/won body strings carry `participant:bid:buyout:deposit:consignment:count` in copper. Names replace Retail's GUID/name lookup in this custom transport. Empty participant denotes multiple buyers (economy purchases); the client uses the Retail multiple-participant label.
- Other auction bodies are empty. Player letters remain literal. No wire or persisted bitcode layout changes; no compatibility translation of historical invented letters.
- Seller invoice retains sale price, buyout, deposit, cut, participant and count independently of remaining attachments. Amount received is bid + deposit - consignment; buyer amount paid is bid.
- Existing direct outbid balance refunds, unsold deposit policy and instant delivery remain unchanged. This change owns text/invoices only.

## Sources

- Retail 12.1.0.69933 local CASC FDID 1394440 (`GlobalStrings`), WoWDBDefs `GlobalStrings.dbd` layout D40F6D96: sold `Auction successful: %s`, won `Auction won: %s`, expired `Auction expired: %s`, outbid `Outbid on %s`, removed `Auction canceled: %s`.
- Retail `AddOns/Blizzard_MailFrame/MailFrame.lua:535-545` renders the formatted subject and `GetInboxText`; `554-630` consumes `GetInboxInvoiceInfo`, formats stack count and renders seller/buyer invoice rows. Lua does not expand auction subjects itself; the native mail API does.
- TrinityCore a352b1fa `src/server/game/AuctionHouse/AuctionHouseMgr.cpp`, `BuildAuctionMailSubject`, `BuildAuctionWonMailBody`, `BuildAuctionSoldMailBody`: structured subject and numeric body, not English. Our tag/name transport is equivalent at the formatting responsibility boundary, not a reproduction of Blizzard packet syntax.

## Tests

`auction_house_tests/clock.rs`: real posting/bid, injected expiry, repeated sweep and reopened delivered subject/body/invoice fields. `lifecycle.rs` and `regressions.rs`: cancellation/outbid subjects and empty bodies; exact refund unchanged. Client rendered text/labels/amounts: engine `godot/ui-model/tests/native_mailbox.rs`.
