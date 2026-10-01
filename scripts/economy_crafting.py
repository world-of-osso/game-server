"""Read-only modern crafting references; no inferred outputs or slot requiredness."""

REFERENCE_TABLES = (
    "crafting_data",
    "crafting_data_item_quality",
    "modified_crafting_spell_slot",
    "modified_crafting_reagent_slot",
    "modified_crafting_category",
    "modified_crafting_reagent_item",
    "mcr_slot_x_mcr_category",
)


def require_crafting_tables(db):
    tables = {
        row[0]
        for row in db.execute("SELECT name FROM sqlite_master WHERE type='table'")
    }
    missing = sorted(set(REFERENCE_TABLES) - tables)
    if missing:
        raise ValueError(
            "Import crafting references with scripts/crafting_data.py into the supplied database; "
            f"missing tables: {', '.join(missing)}"
        )


def crafting_item_ids(db):
    """Return actual modern direct output, fixed reagent and category alternative IDs.

    Pure read of the supplied SQLite connection, including unresolved-slot recipes
    for content projection only. This is not an eligibility or ingredient resolver.
    Reagent-definition IDs are not item IDs; item_sparse supplies the item relation.
    """
    require_crafting_tables(db)
    items = {
        row[0]
        for row in db.execute("""
            SELECT DISTINCT c.CraftedItemID FROM crafting_data c
            JOIN spell_effect e ON e.EffectMiscValue_0=c.ID
            WHERE e.Effect=288 AND e.DifficultyID=0
              AND c.Type=0 AND c.CraftedTreasureID=0 AND c.CraftedItemID>0
        """)
    }
    cursor = db.execute("""
        SELECT r.* FROM spell_reagents r WHERE EXISTS (
            SELECT 1 FROM spell_effect e WHERE e.SpellID=r.SpellID
              AND e.Effect=288 AND e.DifficultyID=0)
    """)
    indices = {column[0]: index for index, column in enumerate(cursor.description)}
    items.update(
        row[indices[f"Reagent_{index}"]]
        for row in cursor
        for index in range(8)
        if row[indices[f"Reagent_{index}"]] > 0
    )
    items.update(
        row[0]
        for row in db.execute("""
            SELECT DISTINCT i.ID FROM item_sparse i
            JOIN modified_crafting_reagent_item r ON r.ID=i.ModifiedCraftingReagentItemID
            JOIN modified_crafting_category c ON c.ID=r.ModifiedCraftingCategoryID
            JOIN mcr_slot_x_mcr_category x ON x.ModifiedCraftingCategoryID=c.ID
            JOIN modified_crafting_spell_slot s
              ON s.ModifiedCraftingReagentSlotID=x.ModifiedCraftingReagentSlotID
            JOIN spell_effect e ON e.SpellID=s.SpellID
            WHERE e.Effect=288 AND e.DifficultyID=0 AND i.ID>0
        """)
    )
    return items
