SELECT CreatureDisplayID AS display_id FROM content_creature_template_model WHERE CreatureDisplayID > 0
UNION SELECT CreatureDisplayID FROM creature_template_model WHERE CreatureDisplayID > 0
UNION SELECT CAST(modelid AS INTEGER) FROM content_creature WHERE modelid != 0
UNION SELECT display_id FROM battle_pet_species WHERE display_id > 0
UNION SELECT CreatureDisplayInfoID FROM battle_pet_display_override WHERE CreatureDisplayInfoID > 0
UNION SELECT CreatureDisplayID FROM spell_shapeshift_form WHERE CreatureDisplayID > 0
