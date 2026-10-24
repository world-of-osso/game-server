//! FactionTemplate reaction rules as pure functions over DB2 rows (no Bevy
//! or storage types), so one copy can move into shared-protocol.
//!
//! `reaction` is AzerothCore's template-only `Unit::GetFactionReactionTo`
//! check; player reputation (forced ranks, at-war) is not consulted.

/// AzerothCore `FACTION_TEMPLATE_FLAG_HATES_ALL_EXCEPT_FRIENDS`.
const HOSTILE_BY_DEFAULT: u32 = 0x2000;

/// One `faction_template` row (DB2 FactionTemplate).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FactionTemplateEntry {
    pub id: u32,
    pub faction: u32,
    pub flags: u32,
    pub faction_group: u32,
    pub friend_group: u32,
    pub enemy_group: u32,
    pub enemies: [u32; 8],
    pub friends: [u32; 8],
}

impl FactionTemplateEntry {
    /// AzerothCore `FactionTemplateEntry::IsFriendlyTo`.
    fn is_friendly_to(&self, other: &Self) -> bool {
        if self.faction == other.faction {
            return true;
        }
        if other.faction != 0 {
            if self.enemies.contains(&other.faction) {
                return false;
            }
            if self.friends.contains(&other.faction) {
                return true;
            }
        }
        self.friend_group & other.faction_group != 0 || self.faction_group & other.friend_group != 0
    }

    /// AzerothCore `FactionTemplateEntry::IsHostileTo`.
    fn is_hostile_to(&self, other: &Self) -> bool {
        if other.faction != 0 {
            if self.enemies.contains(&other.faction) {
                return true;
            }
            if self.friends.contains(&other.faction) {
                return false;
            }
        }
        self.enemy_group & other.faction_group != 0
    }
}

/// How one unit regards another. Unfriendly (a reputation rank) needs the
/// reputation model and is not produced yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Reaction {
    Hostile,
    Neutral,
    Friendly,
}

/// One side of an attack/assist check. `template: None` = no or unknown
/// template (neutral to everyone).
#[derive(Debug, Clone, Copy)]
pub struct Unit<'a> {
    pub template: Option<&'a FactionTemplateEntry>,
    pub is_player: bool,
}

/// Reaction of template `own` towards template `other` (template checks of
/// `GetFactionReactionTo`). A missing template on either side is neutral.
pub fn reaction(
    own: Option<&FactionTemplateEntry>,
    other: Option<&FactionTemplateEntry>,
) -> Reaction {
    let (Some(own), Some(other)) = (own, other) else {
        return Reaction::Neutral;
    };
    if own.id == other.id {
        return Reaction::Friendly;
    }
    if own.is_hostile_to(other) {
        Reaction::Hostile
    } else if own.is_friendly_to(other) || other.is_friendly_to(own) {
        Reaction::Friendly
    } else if own.flags & HOSTILE_BY_DEFAULT != 0 {
        Reaction::Hostile
    } else {
        Reaction::Neutral
    }
}

/// AzerothCore `_IsValidAttackTarget`, faction part: never a friendly
/// unit; creature vs creature only when one is hostile to the other.
/// There is no PvP flag or duel model, so player vs player is rejected.
pub fn can_attack(a: Unit, b: Unit) -> bool {
    let forward = reaction(a.template, b.template);
    let backward = reaction(b.template, a.template);
    if forward == Reaction::Friendly || backward == Reaction::Friendly {
        return false;
    }
    match (a.is_player, b.is_player) {
        (true, true) => false,
        (false, false) => forward == Reaction::Hostile || backward == Reaction::Hostile,
        _ => true,
    }
}

/// Assist (friendly spell) target: `a` regards `b` as friendly. Stands in
/// for `_IsValidAssistTarget`'s reaction check plus its PvC rule (creature
/// `type_flags` CAN_ASSIST is not imported).
pub fn can_assist(a: Unit, b: Unit) -> bool {
    reaction(a.template, b.template) == Reaction::Friendly
}
