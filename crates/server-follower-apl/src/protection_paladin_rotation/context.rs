//! Source-bound facts for the pinned MID2 Protection Paladin action list.
use crate::literal_rotation::equipment::Equipment;
use crate::simc_apl_native::{Bindings, Encounter, NativeFacts};
use bevy::prelude::{Entity, World};
use server::simc_apl::{Action, ActionFacts, Facts};
use shared::components::PowerType;

pub const ACTIONS: &[(&str, u32)] = &[
    ("rite_of_sanctification", 433568),
    ("rite_of_adjuration", 433583),
    ("devotion_aura", 465),
    ("consecration", 26573),
    ("holy_armaments", 432459),
    ("ardent_defender", 31850),
    ("fireblood", 265221),
    ("avenging_wrath", 31884),
    ("divine_toll", 375576),
    ("hammer_of_light", 427453),
    ("shield_of_the_righteous", 53600),
    // SimC76f8118 sc_paladin.cpp3410: Protection uses its own native rank.
    ("hammer_of_wrath", 1241413),
    ("judgment", 20271),
    ("avengers_shield", 31935),
    ("blessed_hammer", 204019),
    ("hammer_of_the_righteous", 53595),
    ("arcane_torrent", 155145),
    ("word_of_glory", 85673),
];
pub const BUFFS: &[(&str, u32)] = &[
    ("avenging_wrath", 31884),
    ("sentinel", 389539),
    ("hammer_of_light_ready", 427441),
    ("undisputed_ruling", 432629),
    ("divine_guidance", 433106),
    ("valor", 1269179),
    ("divine_purpose", 223819),
    ("vanguard", 1268810),
    ("shining_light_free", 327510),
    ("judgment", 197277),
];
pub const TALENTS: &[(&str, u32)] = &[
    ("eye_for_an_eye", 469309),
    ("instrument_of_the_divine", 1277162),
    ("glory_of_the_vanguard_3", 1267215),
];

pub fn bindings() -> Bindings {
    let names = |entries: &[(&str, u32)]| {
        entries
            .iter()
            .map(|&(name, id)| (name.to_owned(), id))
            .collect()
    };
    Bindings {
        actions: names(ACTIONS),
        buffs: names(BUFFS),
        talents: names(TALENTS),
        resources: [("holy_power".into(), PowerType::HolyPower)].into(),
        global_cooldown_spell: 20271,
        ..Default::default()
    }
}

pub struct Context<'a> {
    native: NativeFacts<'a>,
    equipment: &'a Equipment,
    consecration_up: bool,
    guidance_stacks: u8,
    next_armament: Option<u8>,
    potion: &'a str,
}

impl<'a> Context<'a> {
    pub fn new(
        world: &'a World,
        actor: Entity,
        targets: &'a [Entity],
        bindings: &'a Bindings,
        equipment: &'a Equipment,
        encounter: Encounter,
        consecration_up: bool,
        next_armament: Option<u8>,
        potion: &'a str,
    ) -> Self {
        Self {
            native: NativeFacts::new(world, actor, bindings, encounter, targets.len())
                .with_targets(targets.first().copied(), targets),
            equipment,
            consecration_up,
            guidance_stacks: world
                .get::<crate::auras::ActiveAuras>(actor)
                .and_then(|auras| {
                    auras.iter().find(|aura| {
                        aura.spell_id == 433106
                            && aura.caster == actor
                            && aura.source == crate::auras::AuraSource::Cast
                    })
                })
                .map_or(0, |aura| aura.stacks),
            next_armament,
            potion,
        }
    }
}

impl Facts for Context<'_> {
    fn value(&self, name: &str) -> Result<f64, String> {
        if name.starts_with("trinket.") || name.starts_with("equipped.") {
            return self.equipment.fact(name, &self.native);
        }
        match name {
            // SimC76f8118 player.cpp12317–12335: Protection's third apex row.
            "apex.3" => self.native.value("talent.glory_of_the_vanguard_3.enabled"),
            "consecration.up" => Ok(if self.consecration_up { 1.0 } else { 0.0 }),
            "buff.divine_guidance.stack" => Ok(f64::from(self.guidance_stacks)),
            // sc_paladin.hpp25–29: Holy Bulwark=0, Sacred Weapon=1.
            "holy_bulwark" => Ok(0.0),
            "next_armament" => self
                .next_armament
                .map(f64::from)
                .ok_or("NATIVE_ARMAMENT_STATE_UNAVAILABLE".into()),
            "potion.liquid_luster" => Ok(if self.potion == "liquid_luster_2" {
                1.0
            } else {
                0.0
            }),
            _ => self.native.value(name),
        }
    }
}

impl ActionFacts for Context<'_> {
    fn value_for(&self, action: &Action, name: &str) -> Result<f64, String> {
        if name.starts_with("trinket.")
            || name.starts_with("equipped.")
            || matches!(
                name,
                "apex.3"
                    | "consecration.up"
                    | "buff.divine_guidance.stack"
                    | "holy_bulwark"
                    | "next_armament"
                    | "potion.liquid_luster"
            )
        {
            self.value(name)
        } else {
            self.native.value_for(action, name)
        }
    }
    fn target_ids(&self, action: &Action) -> Result<Vec<u64>, String> {
        self.native.target_ids(action)
    }
    fn target_ready(&self, action: &Action) -> Result<bool, String> {
        // SimC76f8118 sc_paladin.cpp1536–1539,1799–1801: Judgment and
        // Hammer of Wrath are mutually exclusive while wings are active.
        if matches!(action.name.as_str(), "hammer_of_wrath" | "judgment") {
            let wings = self.native.value("buff.avenging_wrath.up")? != 0.0
                || self.native.value("buff.sentinel.up")? != 0.0;
            if (action.name == "hammer_of_wrath") != wings {
                return Ok(false);
            }
        }
        self.native.target_ready(action)
    }
    fn primary_resource(&self) -> Result<String, String> {
        self.native.value("holy_power")?;
        Ok("holy_power".into())
    }
}
