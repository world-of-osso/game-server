CREATE TABLE crafting_data (
    ID INTEGER PRIMARY KEY,
    Type INTEGER NOT NULL,
    CraftingDifficultyID INTEGER NOT NULL,
    CraftedItemID INTEGER NOT NULL,
    ItemBonusTreeID INTEGER NOT NULL,
    CraftingDifficulty INTEGER NOT NULL,
    Field_10_0_0_44649_005 REAL NOT NULL,
    CraftSkillBonusPercent REAL NOT NULL,
    ReCraftSkillBonusPercent REAL NOT NULL,
    InspirationSkillBonusPercent REAL NOT NULL,
    Field_10_0_0_44649_009 REAL NOT NULL,
    Field_10_0_0_45141_011 REAL NOT NULL,
    FirstCraftFlagQuestID INTEGER NOT NULL,
    FirstCraftTreasureID INTEGER NOT NULL,
    FirstCraftPlayerDataFlagCharacterID INTEGER NOT NULL,
    CraftedTreasureID INTEGER NOT NULL
);
CREATE TABLE crafting_data_item_quality (
    ID INTEGER PRIMARY KEY,
    ItemID INTEGER NOT NULL,
    CraftingDataID INTEGER NOT NULL
);
CREATE TABLE modified_crafting_spell_slot (
    ID INTEGER PRIMARY KEY,
    SpellID INTEGER NOT NULL,
    Slot INTEGER NOT NULL,
    ModifiedCraftingReagentSlotID INTEGER NOT NULL,
    Field_9_0_1_35679_003 INTEGER NOT NULL,
    ReagentCount INTEGER NOT NULL,
    ReagentReCraftCount INTEGER NOT NULL
);
CREATE TABLE modified_crafting_reagent_slot (
    Name_lang TEXT NOT NULL,
    ID INTEGER PRIMARY KEY,
    Flags INTEGER NOT NULL,
    PlayerConditionID INTEGER NOT NULL,
    ReagentType INTEGER NOT NULL,
    ReagentSource INTEGER NOT NULL,
    Field_11_2_0_61476_006 INTEGER NOT NULL,
    Field_12_0_0_63534_007 REAL NOT NULL
);
CREATE TABLE modified_crafting_category (
    ID INTEGER PRIMARY KEY,
    DisplayName_lang TEXT NOT NULL,
    Description_lang TEXT NOT NULL,
    Field_9_0_1_33978_001 INTEGER NOT NULL,
    MatQualityWeight REAL NOT NULL,
    Field_10_0_0_44649_004 INTEGER NOT NULL
);
CREATE TABLE modified_crafting_reagent_item (
    ID INTEGER PRIMARY KEY,
    Description_lang TEXT NOT NULL,
    ModifiedCraftingCategoryID INTEGER NOT NULL,
    ItemBonusTreeID INTEGER NOT NULL,
    Flags INTEGER NOT NULL,
    Field_9_1_0_38511_004 INTEGER NOT NULL,
    ItemContextOffset INTEGER NOT NULL
);
CREATE TABLE mcr_slot_x_mcr_category (
    ID INTEGER PRIMARY KEY,
    ModifiedCraftingCategoryID INTEGER NOT NULL,
    _Order INTEGER NOT NULL,
    ModifiedCraftingReagentSlotID INTEGER NOT NULL
);
