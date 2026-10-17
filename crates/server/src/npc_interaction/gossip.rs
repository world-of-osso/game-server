//! Gossip menus from world.db (`content_gossip_menu`, `content_gossip_menu_option`, `content_npc_text`,
//! `content_broadcast_text`, `content_creature_template.gossip_menu_id`) and the choice of what an
//! NPC opens. The `conditions` table is not imported, so every option whose
//! `OptionNpcFlag` matches the NPC is shown and a menu with several texts uses the
//! lowest `TextID`.

use std::collections::HashMap;

use bevy::prelude::*;
use rusqlite::{Connection, Row, params};
use shared::protocol::{GossipMenu, GossipMenuOption, InteractionKind, NpcFlags, NpcRole};

/// `npc_text` 68, "Greetings, $n." — the client's default gossip text
/// (AzerothCore `DEFAULT_GOSSIP_MESSAGE`).
const DEFAULT_NPC_TEXT_ID: u32 = 68;
const NPC_TEXT_SLOTS: usize = 8;

/// What selecting a gossip option does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GossipAction {
    Submenu(u32),
    Role(NpcRole),
    Close,
}

#[derive(Debug, Clone, PartialEq)]
struct GossipOptionData {
    option_id: u32,
    icon: u8,
    text: String,
    option_type: u8,
    npc_flag: NpcFlags,
    action_menu_id: u32,
}

#[derive(Debug, Clone, Default, PartialEq)]
struct GossipMenuData {
    text: String,
    options: Vec<GossipOptionData>,
}

#[derive(Resource, Debug, Default, Clone)]
pub struct GossipCatalog {
    menus: HashMap<u32, GossipMenuData>,
    template_menus: HashMap<u32, u32>,
    default_text: String,
}

/// A frame to open plus the routing of its gossip options.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Opening {
    pub kind: InteractionKind,
    pub actions: Vec<(u32, GossipAction)>,
}

impl GossipCatalog {
    /// What interacting with an NPC opens (Retail): a gossip menu when it has the
    /// GOSSIP flag or several roles, otherwise its single role; `None` when it has
    /// nothing to interact with. Quests are listed by the quest system, not as a
    /// gossip option.
    pub(crate) fn open(&self, template_id: u32, flags: NpcFlags) -> Option<Opening> {
        // TC CanOpenMailBox accepts a creature's MAILBOX flag directly, including
        // Katy's source GOSSIP|MAILBOX flags; no authored gossip choice is required.
        if flags.contains(NpcFlags::MAILBOX) {
            return Some(Opening {
                kind: InteractionKind::Role(NpcRole::Mailbox),
                actions: Vec::new(),
            });
        }
        let roles = roles_from_flags(flags);
        if !flags.contains(NpcFlags::GOSSIP) && roles.len() == 1 {
            return Some(Opening {
                kind: InteractionKind::Role(roles[0]),
                actions: Vec::new(),
            });
        }
        if !flags.contains(NpcFlags::GOSSIP) && roles.is_empty() {
            return None;
        }
        // Menu 0 is AzerothCore's default menu: one option per NPC role.
        let menu_id = self.template_menus.get(&template_id).copied().unwrap_or(0);
        Some(self.submenu(menu_id, flags))
    }

    /// Menu `menu_id` with the options `flags` allow.
    pub(crate) fn submenu(&self, menu_id: u32, flags: NpcFlags) -> Opening {
        match self.menus.get(&menu_id) {
            Some(menu) => self.data_menu(menu_id, menu, flags),
            None => gossip_opening(menu_id, self.default_text.clone(), Vec::new()),
        }
    }

    fn data_menu(&self, menu_id: u32, menu: &GossipMenuData, flags: NpcFlags) -> Opening {
        let options = menu
            .options
            .iter()
            .filter(|option| flags.0 & option.npc_flag.0 != 0)
            .filter_map(|option| {
                let action = option_action(option.option_type, option.action_menu_id)?;
                Some((option.option_id, option.icon, option.text.clone(), action))
            })
            .collect();
        let text = if menu.text.is_empty() {
            self.default_text.clone()
        } else {
            menu.text.clone()
        };
        gossip_opening(menu_id, text, options)
    }
}

fn gossip_opening(
    menu_id: u32,
    text: String,
    options: Vec<(u32, u8, String, GossipAction)>,
) -> Opening {
    let actions = options
        .iter()
        .map(|(id, _, _, action)| (*id, *action))
        .collect();
    let options = options
        .into_iter()
        .map(|(option_id, icon, text, _)| GossipMenuOption {
            option_id,
            icon,
            text,
        })
        .collect();
    Opening {
        kind: InteractionKind::Gossip(GossipMenu {
            menu_id,
            text,
            options,
        }),
        actions,
    }
}

#[cfg(test)]
mod transmog_tests {
    use super::*;

    #[test]
    fn transmog_authored_gossip_option_opens_the_service() {
        let conn = Connection::open_with_flags(
            crate::test_data::world_db_path(),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let catalog = load_gossip_catalog(&conn).unwrap();
        let opening = catalog.open(54442, NpcFlags(268435585)).unwrap();
        assert!(
            opening
                .actions
                .contains(&(0, GossipAction::Role(NpcRole::Transmogrifier)))
        );
    }

    #[test]
    fn transmog_flag_opens_the_transmogrifier_service() {
        let opening = GossipCatalog::default().open(62821, NpcFlags(NpcFlags::TRANSMOGRIFIER));
        assert_eq!(
            opening.map(|opening| opening.kind),
            Some(InteractionKind::Role(NpcRole::Transmogrifier))
        );
    }
}

/// Roles an NPC's flags grant, in Retail gossip order.
pub(crate) fn roles_from_flags(flags: NpcFlags) -> Vec<NpcRole> {
    const VENDOR_ANY: u64 = NpcFlags::VENDOR
        | NpcFlags::VENDOR_AMMO
        | NpcFlags::VENDOR_FOOD
        | NpcFlags::VENDOR_POISON
        | NpcFlags::VENDOR_REAGENT
        | NpcFlags::REPAIR;
    const TRAINER_ANY: u64 =
        NpcFlags::TRAINER | NpcFlags::TRAINER_CLASS | NpcFlags::TRAINER_PROFESSION;
    [
        (NpcFlags::QUESTGIVER, NpcRole::QuestGiver),
        (VENDOR_ANY, NpcRole::Vendor),
        (TRAINER_ANY, NpcRole::Trainer),
        (NpcFlags::FLIGHTMASTER, NpcRole::FlightMaster),
        (NpcFlags::INNKEEPER, NpcRole::Innkeeper),
        (NpcFlags::BANKER, NpcRole::Banker),
        (NpcFlags::PETITIONER, NpcRole::Petitioner),
        (NpcFlags::TABARD_DESIGNER, NpcRole::TabardDesigner),
        (NpcFlags::BATTLEMASTER, NpcRole::Battlemaster),
        (NpcFlags::AUCTIONEER, NpcRole::AuctionHouse),
        (NpcFlags::STABLEMASTER, NpcRole::StableMaster),
        (NpcFlags::TRANSMOGRIFIER, NpcRole::Transmogrifier),
        (NpcFlags::CRAFTING_ORDERS, NpcRole::CraftingOrders),
        (NpcFlags::TRADING_POST, NpcRole::TradingPost),
    ]
    .into_iter()
    .filter(|(mask, _)| flags.contains(*mask))
    .map(|(_, role)| role)
    .collect()
}

/// AzerothCore `GossipOptionType` → action. Hidden (`None`): NONE (script-only),
/// QUESTGIVER (quests are listed separately), spirit healer/guide (dead players
/// only), ARMORER (AzerothCore never shows it), and the pre-Retail talent
/// unlearn / dual spec / outdoor PvP types.
fn option_action(option_type: u8, action_menu_id: u32) -> Option<GossipAction> {
    Some(match option_type {
        1 if action_menu_id != 0 => GossipAction::Submenu(action_menu_id),
        1 => GossipAction::Close,
        3 => GossipAction::Role(NpcRole::Vendor),
        4 => GossipAction::Role(NpcRole::FlightMaster),
        5 => GossipAction::Role(NpcRole::Trainer),
        8 => GossipAction::Role(NpcRole::Innkeeper),
        9 => GossipAction::Role(NpcRole::Banker),
        10 => GossipAction::Role(NpcRole::Petitioner),
        11 => GossipAction::Role(NpcRole::TabardDesigner),
        12 => GossipAction::Role(NpcRole::Battlemaster),
        13 => GossipAction::Role(NpcRole::AuctionHouse),
        14 => GossipAction::Role(NpcRole::StableMaster),
        34 => GossipAction::Role(NpcRole::Transmogrifier),
        // Mainline GossipNpcOption::PerksProgramVendor; TDB Seabraid menus30312/30315.
        47 => GossipAction::Role(NpcRole::TradingPost),
        // TDB1210 menu30243 OptionNpc50: create a crafting order.
        50 => GossipAction::Role(NpcRole::CraftingOrders),
        _ => return None,
    })
}

pub fn load_gossip_catalog(conn: &Connection) -> rusqlite::Result<GossipCatalog> {
    let broadcast = load_broadcast_texts(conn)?;
    let npc_texts = load_npc_texts(conn, &broadcast)?;
    let mut menus = load_menu_texts(conn, &npc_texts)?;
    load_menu_options(conn, &broadcast, &mut menus)?;
    let mut template_menus = conn
        .prepare(
            "SELECT entry, gossip_menu_id FROM content_creature_template WHERE gossip_menu_id <> 0",
        )?
        .query_map(params![], |row| {
            Ok((row.get::<_, i64>(0)? as u32, row.get::<_, i64>(1)? as u32))
        })?
        .collect::<rusqlite::Result<_>>()?;
    load_crafting_broker_options(conn, &mut menus, &mut template_menus)?;
    Ok(GossipCatalog {
        menus,
        template_menus,
        default_text: npc_texts
            .get(&DEFAULT_NPC_TEXT_ID)
            .cloned()
            .unwrap_or_default(),
    })
}

/// Source Retail brokers retain their TDB customer-service choice, not a synthetic menu.
fn load_crafting_broker_options(
    conn: &Connection,
    menus: &mut HashMap<u32, GossipMenuData>,
    templates: &mut HashMap<u32, u32>,
) -> rusqlite::Result<()> {
    let mut statement = conn.prepare(
        "SELECT c.entry,g.MenuID,o.OptionID,o.OptionText
        FROM tdb_creature_template c
        JOIN tdb_creature_template_gossip g ON g.CreatureID=c.entry
        JOIN tdb_gossip_menu_option o ON o.MenuID=g.MenuID
        WHERE (c.npcflag & ?1)!=0 AND o.OptionNpc=50 ORDER BY c.entry,o.OptionID",
    )?;
    for row in statement.query_map([NpcFlags::CRAFTING_ORDERS as i64], |row| {
        Ok((
            row.get::<_, u32>(0)?,
            row.get::<_, u32>(1)?,
            row.get::<_, u32>(2)?,
            row.get::<_, String>(3)?,
        ))
    })? {
        let (entry, menu_id, option_id, text) = row?;
        templates.insert(entry, menu_id);
        let menu = menus.entry(menu_id).or_default();
        if menu
            .options
            .iter()
            .any(|option| option.option_id == option_id)
        {
            continue;
        }
        menu.options.push(GossipOptionData {
            option_id,
            icon: 0,
            text,
            option_type: 50,
            npc_flag: NpcFlags(NpcFlags::CRAFTING_ORDERS),
            action_menu_id: 0,
        });
    }
    Ok(())
}

/// broadcast_text id → MaleText, or FemaleText when MaleText is empty.
fn load_broadcast_texts(conn: &Connection) -> rusqlite::Result<HashMap<u32, String>> {
    conn.prepare("SELECT ID, MaleText, FemaleText FROM content_broadcast_text")?
        .query_map(params![], |row| {
            let male: Option<String> = row.get(1)?;
            let female: Option<String> = row.get(2)?;
            Ok((
                row.get::<_, i64>(0)? as u32,
                first_non_empty([male, female]),
            ))
        })?
        .collect()
}

fn first_non_empty<const N: usize>(texts: [Option<String>; N]) -> String {
    texts
        .into_iter()
        .flatten()
        .find(|text| !text.is_empty())
        .unwrap_or_default()
}

/// npc_text id → text of its most probable slot (lowest slot on ties).
fn load_npc_texts(
    conn: &Connection,
    broadcast: &HashMap<u32, String>,
) -> rusqlite::Result<HashMap<u32, String>> {
    let columns: Vec<String> = (0..NPC_TEXT_SLOTS)
        .map(|slot| format!("text{slot}_0, text{slot}_1, BroadcastTextID{slot}, Probability{slot}"))
        .collect();
    let sql = format!("SELECT ID, {} FROM content_npc_text", columns.join(", "));
    conn.prepare(&sql)?
        .query_map(params![], |row| {
            Ok((
                row.get::<_, i64>(0)? as u32,
                npc_text_from_row(row, broadcast)?,
            ))
        })?
        .collect()
}

fn npc_text_from_row(row: &Row, broadcast: &HashMap<u32, String>) -> rusqlite::Result<String> {
    let mut best: Option<(f64, String)> = None;
    for slot in 0..NPC_TEXT_SLOTS {
        let base = 1 + slot * 4;
        let broadcast_id = row.get::<_, i64>(base + 2)? as u32;
        let text = broadcast
            .get(&broadcast_id)
            .filter(|text| !text.is_empty())
            .cloned()
            .unwrap_or_else(|| first_non_empty([row.get(base).ok(), row.get(base + 1).ok()]));
        let probability: f64 = row.get(base + 3)?;
        if text.is_empty() || best.as_ref().is_some_and(|(p, _)| *p >= probability) {
            continue;
        }
        best = Some((probability, text));
    }
    Ok(best.map(|(_, text)| text).unwrap_or_default())
}

fn load_menu_texts(
    conn: &Connection,
    npc_texts: &HashMap<u32, String>,
) -> rusqlite::Result<HashMap<u32, GossipMenuData>> {
    conn.prepare("SELECT MenuID, MIN(TextID) FROM content_gossip_menu GROUP BY MenuID")?
        .query_map(params![], |row| {
            let text_id = row.get::<_, i64>(1)? as u32;
            Ok((
                row.get::<_, i64>(0)? as u32,
                GossipMenuData {
                    text: npc_texts.get(&text_id).cloned().unwrap_or_default(),
                    options: Vec::new(),
                },
            ))
        })?
        .collect()
}

fn load_menu_options(
    conn: &Connection,
    broadcast: &HashMap<u32, String>,
    menus: &mut HashMap<u32, GossipMenuData>,
) -> rusqlite::Result<()> {
    let mut stmt = conn.prepare(
        // TDB GossipOptionNpc::Transmogrifier (34) is not an AzerothCore
        // OptionType. Preserve that authored service instead of its lossy view row.
        "SELECT m.MenuID, m.OptionID, m.OptionIcon, m.OptionText, m.OptionBroadcastTextID,
                CASE WHEN r.OptionNpc = 34 THEN 34 ELSE m.OptionType END,
                CASE WHEN r.OptionNpc = 34 THEN 268435456 ELSE m.OptionNpcFlag END,
                m.ActionMenuID
         FROM content_gossip_menu_option m
         LEFT JOIN tdb_gossip_menu_option r ON r.MenuID = m.MenuID AND r.OptionID = m.OptionID
         ORDER BY m.MenuID, m.OptionID",
    )?;
    let rows = stmt.query_map(params![], |row| {
        let broadcast_id = row.get::<_, i64>(4)? as u32;
        let option_text: Option<String> = row.get(3)?;
        let text = broadcast
            .get(&broadcast_id)
            .filter(|text| !text.is_empty())
            .cloned()
            .unwrap_or_else(|| first_non_empty([option_text]));
        Ok((
            row.get::<_, i64>(0)? as u32,
            GossipOptionData {
                option_id: row.get::<_, i64>(1)? as u32,
                icon: row.get::<_, i64>(2)? as u8,
                text,
                option_type: row.get::<_, i64>(5)? as u8,
                npc_flag: NpcFlags(row.get::<_, i64>(6)? as u64),
                action_menu_id: row.get::<_, i64>(7)? as u32,
            },
        ))
    })?;
    for row in rows {
        let (menu_id, option) = row?;
        menus.entry(menu_id).or_default().options.push(option);
    }
    Ok(())
}
