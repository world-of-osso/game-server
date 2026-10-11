//! Audited visual-only toys. DB2 dummy effects alone do NOT prove cosmetics:
//! Gadgetzan teleport and scouting maps have the same zero-valued shape.
//! See docs/wiki/systems/toys.md for descriptions and the excluded script census.
use crate::spell_info::SpellInfo;

/// These descriptions promise only a visual/attachment/sound, not a scripted
/// action. Both the audit AND current DB2 shape must agree; metadata drift blocks.
const VISUAL_ONLY_SPELLS: &[u32] = &[
    45416, 45418,  // Dancing Insoles, Fire Festival Batons
    128328, // Ken-Ken's Mask (friendly-target attachment)
    131317, 131318, // Alliance / Horde fireworks
    148538, // Blackflame Daggers
    171761, // Brewfest Banner
    176457, // Fruit Basket
    176594, // Touch of the Naaru
    185712, // Seafarer's Slidewhistle
    203657, // Noble's Eternal Elementium Signet
    217708, // Gravil Goldbraid's Famous Sausage Hat
    231129, 231142, 231143, 231144,  // Falcosaur accessories
    239560,  // Akazamzarak's Spare Hat (hat/rabbit visual models, not a summon)
    254513,  // Barrier Generator (channeled shield visual, not an absorb)
    261981,  // Spitzy
    286277,  // Worn Cloak
    302273,  // Heart of a Champion
    383065,  // Duck-Stuffed Duck Lovie (authored nap animation/attachment/sound)
    405634,  // A Drake's Big Basket of Eggs (conditional mount attachment)
    406870,  // Mallard Mortar (targeted squeak, not damage)
    409891,  // Blazing Shadowflame Cinder
    443689,  // Toxic Victory
    447446,  // Cherry Blossom Trail
    455023,  // Filmless Camera
    462145,  // Lampyridae Lure
    1269949, // Valdekar's Special
    1280563, // Lil' Scoots' Pillow
    1237584, 1271973, 1298433, // Etheric / Dominating / Corrosive Victory
];

pub(crate) fn is_visual_only(spell: &SpellInfo) -> bool {
    spell.has_spell_visual
        && VISUAL_ONLY_SPELLS.contains(&spell.id)
        && spell.effects.iter().any(|effect| effect.effect != 0)
        && spell.effects.iter().all(|effect| {
            if effect.effect == 0 {
                return true;
            }
            let visual_type = matches!(effect.effect, 3 | 77)
                || (effect.effect == 6 && effect.aura == crate::auras::handlers::DUMMY);
            visual_type
                && effect.trigger_spell == 0
                && effect.misc_values == [0, 0]
                && effect.base_points == 0.0
                && effect.item_type == 0
                && effect.mechanic == 0
        })
}
