//! Audited visual-only toys. DB2 dummy effects alone do NOT prove cosmetics:
//! Gadgetzan teleport and scouting maps have the same zero-valued shape.
//! See docs/wiki/systems/toys.md for descriptions and the excluded script census.
use crate::spell_info::{SpellEffect, SpellInfo};

/// These descriptions promise only a visual/attachment/sound, not a scripted
/// action. Both the audit AND current DB2 shape must agree; metadata drift blocks.
const VISUAL_ONLY_SPELLS: &[u32] = &[
    45416, 45418,  // Dancing Insoles, Fire Festival Batons
    71909,  // The Heartbreaker's authored sob animations/heart attachment
    91743,  // Fountain of Knowledge: source10s ordinary ground model253570.
    128327, // Iron Belly Spirits (friendly-target mug/refreshment visual)
    128328, // Ken-Ken's Mask (friendly-target attachment)
    131317, 131318, // Alliance / Horde fireworks
    148538, // Blackflame Daggers
    171352, // Blazing Diamond Pendant: source120s ground magma, area group4031.
    171761, // Brewfest Banner
    176457, // Fruit Basket
    176594, // Touch of the Naaru
    // Gladiator back banners: existing native race/sex condition selection proven.
    176899, 176900, 185712, // Seafarer's Slidewhistle
    187146, // Tome of Secrets: source5min ordinary attachment1097945.
    193333, // Death's Door Charm's Helheim ghost-state attachment
    196783, // Darkshard Fragment (crystal growth attachments)
    203657, // Noble's Eternal Elementium Signet
    217708, // Gravil Goldbraid's Famous Sausage Hat
    219708,
    219871, // DH Skull of Corruption/Fel Focusing Crystal: permanent contemplation visuals.
    223143, // Prismatic Bauble: source20s ordinary rainbow attachment1417024.
    231129, 231142, 231143, 231144, // Falcosaur accessories
    239560, // Akazamzarak's Spare Hat (hat/rabbit visual models, not a summon)
    254513, // Barrier Generator (channeled shield visual, not an absorb)
    261981, // Spitzy
    272071, // Roar of Rezan: one-shot caster sound107006/animation74, no gameplay payload.
    286277, // Worn Cloak
    300104, 300144, // War Banner: source CurrentPvpFaction condition and combat interrupts
    302273, // Heart of a Champion
    318452, // Aqir Egg Cluster's missile-triggered swarm attachments
    302730, // Brewfest Chowdown Trophy's champion animation aura (visual88525)
    383065, // Duck-Stuffed Duck Lovie (authored nap animation/attachment/sound)
    // Audited build70448 complete model/positioner/animation/sound chains;
    // no scripted/gameplay payload or alias inferred from the dummy slots.
    202432, 295037, // Hot Buttered Popcorn, Banner of the Burning Blade.
    395705, 399041, // Gleaming Arcanocrystal, Lost Compass (not Wayfinder's Compass).
    396236, 1232024, // Yennu's Kite, Chowdar's Favorite Ribbon: ordinary owner attachments.
    425503,  // Fast-Growing Seed: source3s planting cast/30s ordinary growth visual.
    405634,  // A Drake's Big Basket of Eggs (conditional mount attachment)
    406870,  // Mallard Mortar (targeted squeak, not damage)
    409891,  // Blazing Shadowflame Cinder
    408067,  // B.B.F. Fist's selected-target animations/hand attachment
    430760,  // Abyss Caller Horn's authored water visual, not gameplay water
    443689,  // Toxic Victory
    447446,  // Cherry Blossom Trail
    455023,  // Filmless Camera
    462145,  // Lampyridae Lure
    1269949, // Valdekar's Special
    1280563, // Lil' Scoots' Pillow
    1237584, 1271973, 1298433, // Etheric / Dominating / Corrosive Victory
    // Build70448 stationary Loa victory/hand-glow/feathers: source30min cosmetics.
    1309827, 1309830, 1309831, 1309828,
    1309844, // Halazzi / Jan'alai source30min footstep cosmetics.
    // Complete ordinary-model caster kits; no procedural/positioner/conditional payload.
    // Exact source contracts and exclusions: docs/toy-effects14-coverage.md.
    160093, 160688, 170869, // Mysterious Flower, smoldering boots, Blazing Wings
    179869,
    298869, // Gaze of the Darkmoon enemy marker; Ocean Simulator sound/attachment kits
    188172, 212660, 242014, // Personal Spotlight, Kirin Tor mark, Demon Hunter Pepe
    225826, 225828, // Magi crystal cone and Powder Box friendly-area attachment kits
    280053, 335276, 340276, // Light snowfall, back vessel, quill/parchment
    403491, 467285, 1264653, // Hraxian's Will, Explosive Victory, wine toast
    // Cast-event ordinary-model kits; no aura, script binding or payload.
    103740, 290627, 187150, // Darkmoon Firework, Azerite Firework Launcher, Jewel of Hellfire
    // Spellcaster-object spells: self cosmetic auras with complete ordinary kits.
    424023, 1215373, // Chilling Celebration Banner, Kaja'Cola Enthusiast
];

/// Source-audited dummy aura slots, not a blanket mixed-spell exemption.
/// Paint carries amount1; the shell carries amount1 plus two native trigger
/// effects whose children each carry amount0. Their visuals and movement
/// interrupts are authored in DB2. See docs/toy-effects13-coverage.md.
/// Singed1220867 is the zero-payload five-second cosmetic child of1220729.
/// TC SpellAuraEffects.cpp:76,4836–5117 dispatches dummy4 to spell-specific
/// handlers, not a generic immediate effect; unaudited dummy spells stay refused.
/// Other slots still require their own ordinary runtime handler.
pub(crate) fn is_visual_aura(spell: &SpellInfo, effect: &SpellEffect) -> bool {
    let expected_amount = match (spell.id, effect.index) {
        (127803 | 128150, 0) => 1.0,
        // Exact70448 seasonal pennants: ordinary model/race-sex visual selection,
        // authored amount1 is not a generic zero-payload dummy exemption.
        (412530 | 412912 | 425994 | 430508 | 456798 | 460082 | 460083 | 1217166 | 1233142, 0) => {
            1.0
        }
        (128151 | 128152 | 1220867, 0) => 0.0,
        // TC taunt-flag script casts51657: native sixty-second corpse marker;
        // the sibling effect76 creates the actual authored flag object.
        (51657, 1) => 0.0,
        _ => return false,
    };
    let dummy_aura = effect.effect == 6 && effect.aura == crate::auras::handlers::DUMMY;
    let cosmetic_payload = effect.trigger_spell == 0
        && effect.misc_values == [0, 0]
        && effect.base_points == expected_amount;
    let no_gameplay_payload = effect.item_type == 0 && effect.mechanic == 0;
    spell.has_spell_visual && dummy_aura && cosmetic_payload && no_gameplay_payload
}

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
