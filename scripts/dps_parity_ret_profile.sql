-- Private-copy-only Ret parity surrogate, approved by lead 2026-10-23.
-- Retain sourced Captain Garrick template/model/armor and class2; change the
-- controller identity to native ChrSpecialization70. This is not an official
-- Ret follower identity and must never be applied to the canonical world.db.
-- Attack20271 and cooldown31884 are native Paladin abilities and appear in the
-- pinned SimC76f8118 Ret APL. Rotation decisions do not use the legacy slots.
UPDATE follower_profiles
SET spec_id = 70,
    role = 2,
    attack_spell = 20271,
    cooldown_spell = 31884,
    heal_spell = 0,
    taunt_spell = 0,
    form_spell = 0,
    melee = 1,
    label = 'Retribution Paladin (private parity surrogate)'
WHERE entry = 209057 AND class_id = 2 AND spec_id = 66;
