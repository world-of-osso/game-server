# Mythic+ catalog foundation

Offline local-CASC challenge catalog import exists; the server/client keystone loop is not operational. [Feature contract](../../specs/mythic-plus.md) owns acceptance. [Source qualification](../../mythic-plus-data.md) owns dungeon selection, numeric evidence, remaining policy gaps and the lead-owned copy-import command.

## Content

The importer validates destination independence and all six DB2 layouts before writing. It decodes readable records, normalizes matched DBD integer widths, compares duplicated inline relationship IDs and writes every table/source receipt in one explicit SQLite transaction. No runtime startup path calls it; publication is an explicit lead-owned administrative step.

Targeted five-test proof covers authentic catalog values, idempotency, corrupted-layout rejection, SQL rollback, destination refusal and read-only population/force eligibility. It does not prove inventory/Font interactions, affix selection, timers running, score calculation or either skin rendering.

## Sources

- [Importer](../../../scripts/import_mythic_plus.py) — copy-only CLI.
- [Tests](../../../scripts/tests/test_import_mythic_plus.py) — local behavioral proof.
- [Data qualification](../../mythic-plus-data.md) — exact local/pinned upstream sources.

## See Also

- [[content-import]] — existing instance population importer, not duplicated here.
- [[boss-scripts]] — Cobrahn framework; not an Azure Vault M+ definition.
