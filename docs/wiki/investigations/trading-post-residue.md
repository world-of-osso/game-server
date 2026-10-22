# Trading Post private-account residue

Verified 2026-10-22: vendor items 1615 and 1608 are already admitted to October 2026 (PerksMonth 46). “Residue” describes expired purchases on the disposable historical `fb_tpui_mod` account, not missing catalog content.

## Content readback

| Vendor | Item | Name | Tender | Modified appearance → appearance → display |
|---|---|---|---:|---|
| 1615 | 233053 | Crown of the Violet Rose | 250 | 230544 → 83428 → 673911 |
| 1608 | 269755 | Large Artisan Katana | 130 | 304804 → 129574 → 733288 |

Both literal item links already join those vendor IDs in `content/trading-post-monthly-catalog.json`, October entry (array index 44). Local DB2 build `12.1.5.70448` and read-only canonical `game-server/data/world.db` agree on vendor/item/price and the transmog chains above. The cached Wowhead October article also carries both item links, prices and display IDs. Its embedded article markup gives prices 250/130; the rendered-text currency amounts are truncated and must not replace those values.

The historical account owns both purchases, costing 380 Tender, beyond their refund window. The earlier handoff explicitly left that state untouched. No catalog admission, producer fix, SQL delta, refund-policy change or account repair is needed or authorized here. No behavioral failure was reproduced; no RED→GREEN claim is made.

## Sources

- [Monthly catalog](../../../content/trading-post-monthly-catalog.json) — existing PerksMonth 46 membership and item joins; [source policy](../../specs/trading-post.md#monthly-inventory-source-policy-lead-decision-2026-10-19).
- `/home/osso/Projects/world-of-osso/game-engine/docs/handoffs/trading-post-forever-slow-purchase.md`, “Separate prior-state residue — not cleaned” — private-account ownership residue and expired refund window.
- Evidence root: `/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/trading-post-calendar-2026-10-19/`; `news-sample/2026-10.html` contains the article, `news-backfill/draft-catalog.json` its exact joins, and `news-backfill/report.md` the extraction boundary.
- DB2: `/home/osso/Projects/world-of-osso/game-engine/data/db2/12.1.5.70448/{PerksVendorItem,ItemSparse,ItemModifiedAppearance,ItemAppearance}.csv`; canonical SQLite readback used read-only mode. Direct Wowhead item requests through `/home/osso/.worktrees/logs/wowhead-fetch.py` returned HTTP 202; conclusions use the cached article, not unavailable item-page responses.

## See Also

- [Trading Post authority](../../specs/trading-post.md) — approved news-based monthly policy, not exhaustive Retail membership proof.
