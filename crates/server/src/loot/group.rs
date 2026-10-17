//! Authoritative group rolls. Retail GroupLootFrame.lua/LootHistory.lua;
//! TrinityCore a352b1fa Loot.cpp: LootRoll (timeout, all-pass, winner delivery).

use std::collections::{BTreeMap, HashMap, VecDeque};

use super::*;
use crate::auth::Authenticated;
use crate::class_progression::KnownSpells;
use crate::group::GroupRegistry;
use crate::persistence::ClientId;
use rand::Rng;
use shared::components::Player;
use shared::loot::{LootDrop, LootMode, LootRollChoice, RollWindow};
use shared::protocol::{GroupLootChoice, GroupLootHistoryRequest, GroupLootPlayer, GroupLootRoll};

const HISTORY_LIMIT: usize = 50;
pub(super) const SHARED_SLOT_BASE: u8 = 128;

#[derive(Component, Default)]
pub(super) struct CorpseGroupLoot(pub Vec<GroupPool>);

pub(super) struct GroupPool {
    pub members: Vec<u64>,
    pub slots: Vec<Option<LootContent>>,
    pub pending: usize,
}

impl GroupPool {
    pub fn has_loot(&self) -> bool {
        self.pending > 0 || self.slots.iter().any(Option::is_some)
    }

    pub fn remaining(&self) -> Vec<LootSlot> {
        self.slots
            .iter()
            .enumerate()
            .filter_map(|(index, content)| {
                Some(LootSlot {
                    slot: SHARED_SLOT_BASE + index as u8,
                    content: content.clone()?,
                })
            })
            .collect()
    }
}

impl CorpseGroupLoot {
    pub fn pool(&self, character: u64) -> Option<&GroupPool> {
        self.0.iter().find(|pool| pool.members.contains(&character))
    }

    pub fn pool_mut(&mut self, character: u64) -> Option<&mut GroupPool> {
        self.0
            .iter_mut()
            .find(|pool| pool.members.contains(&character))
    }

    pub fn has_loot(&self) -> bool {
        self.0.iter().any(GroupPool::has_loot)
    }

    pub fn has_pending_rolls(&self) -> bool {
        self.0.iter().any(|pool| pool.pending > 0)
    }
}

/// RNG is injected at the roll boundary, never provided by a client.
#[derive(Resource)]
pub(super) struct GroupRollRng(pub Box<dyn FnMut() -> u8 + Send + Sync>);

impl Default for GroupRollRng {
    fn default() -> Self {
        Self(Box::new(|| rand::thread_rng().gen_range(1..=100)))
    }
}

struct PendingRoll {
    window: RollWindow,
    snapshot: GroupLootRoll,
    pool: usize,
}

#[derive(Resource, Default)]
pub(super) struct GroupRolls {
    active: BTreeMap<u64, PendingRoll>,
    history: HashMap<u64, VecDeque<GroupLootRoll>>,
    next_id: u64,
}

impl GroupRolls {
    fn remember(&mut self, snapshot: &GroupLootRoll) {
        for player in &snapshot.players {
            let entries = self.history.entry(player.character_id).or_default();
            if let Some(entry) = entries
                .iter_mut()
                .find(|entry| entry.roll_id == snapshot.roll_id)
            {
                *entry = snapshot.clone();
            } else {
                entries.push_back(snapshot.clone());
            }
            while entries.len() > HISTORY_LIMIT {
                entries.pop_front();
            }
        }
    }

    fn start(
        &mut self,
        corpse: Entity,
        pool: usize,
        item: LootContent,
        eligibility: Vec<(GroupLootPlayer, bool)>,
    ) -> GroupLootRoll {
        let LootContent::Item { item_id, count, .. } = item else {
            unreachable!("only items roll")
        };
        self.next_id += 1;
        let preferred_transmog: Vec<_> = eligibility
            .iter()
            .filter(|(_, preferred)| *preferred)
            .map(|(player, _)| player.character_id)
            .collect();
        let players: Vec<_> = eligibility.into_iter().map(|(player, _)| player).collect();
        let members: Vec<_> = players.iter().map(|player| player.character_id).collect();
        let need: Vec<_> = players
            .iter()
            .filter(|player| player.can_need)
            .map(|player| player.character_id)
            .collect();
        let mut window = RollWindow::new(
            LootDrop { item_id, count },
            &members,
            &need,
            &preferred_transmog,
        );
        window.can_transmog = players
            .iter()
            .filter(|player| player.can_transmog)
            .map(|player| player.character_id)
            .collect();
        let snapshot = GroupLootRoll {
            roll_id: self.next_id,
            corpse: corpse.to_bits(),
            item,
            seconds_remaining: window.time_remaining,
            players,
            winner: None,
            complete: false,
            delivered: false,
        };
        bevy::log::debug!(
            corpse = snapshot.corpse,
            roll_id = snapshot.roll_id,
            item_id,
            count,
            eligible_players = ?members,
            "started group loot roll"
        );
        self.remember(&snapshot);
        self.active.insert(
            snapshot.roll_id,
            PendingRoll {
                window,
                snapshot: snapshot.clone(),
                pool,
            },
        );
        snapshot
    }
}

type GroupPlayers<'w, 's> = Query<
    'w,
    's,
    (
        &'static ClientId,
        &'static Player,
        Option<&'static KnownSpells>,
    ),
>;

#[derive(SystemParam)]
pub(super) struct GroupGeneration<'w, 's> {
    pub groups: Option<ResMut<'w, GroupRegistry>>,
    rolls: ResMut<'w, GroupRolls>,
    players: GroupPlayers<'w, 's>,
    accounts: Query<'w, 's, (&'static InWorld, &'static Authenticated)>,
    senders: Query<
        'w,
        's,
        (
            Entity,
            &'static InWorld,
            &'static mut MessageSender<GroupLootRoll>,
        ),
    >,
}

impl GroupGeneration<'_, '_> {
    pub fn distribute(
        &mut self,
        corpse: Entity,
        entry: u32,
        difficulty: u32,
        rewarded: &[u64],
        source: &LootRolls,
    ) -> (CorpseLoot, CorpseGroupLoot) {
        let mut personal = CorpseLoot::default();
        let mut shared = CorpseGroupLoot::default();
        let mut assigned = Vec::new();
        for &character in rewarded {
            if assigned.contains(&character) {
                continue;
            }
            let members = self
                .groups
                .as_deref()
                .map_or_else(Vec::new, |groups| groups.group_members(character));
            let members: Vec<_> = members
                .into_iter()
                .filter(|member| rewarded.contains(member))
                .collect();
            if members.is_empty() {
                let slots = source.roll(entry, difficulty, character, None);
                if !slots.is_empty() {
                    personal.looters.insert(character, slots);
                }
                assigned.push(character);
                continue;
            }
            assigned.extend(&members);
            let (mode, threshold) = self
                .groups
                .as_deref()
                .and_then(|groups| groups.loot_policy_of(character))
                .expect("group membership exists");
            let common = source.roll(entry, difficulty, character, Some(false));
            let pool_index = shared.0.len();
            let mut pool = GroupPool {
                members: members.clone(),
                slots: Vec::new(),
                pending: 0,
            };
            for &member in &members {
                let kind = if mode == LootMode::PersonalLoot {
                    None
                } else {
                    Some(true)
                };
                let items = source
                    .roll(entry, difficulty, member, kind)
                    .into_iter()
                    .filter(|content| !matches!(content, Some(LootContent::Money { .. })))
                    .collect();
                personal.looters.insert(member, items);
            }
            let owner = self
                .groups
                .as_deref_mut()
                .expect("group exists")
                .next_loot_owner(character, &members);
            for content in common.into_iter().flatten() {
                match content {
                    LootContent::Money { .. } => pool.slots.push(Some(content)),
                    LootContent::Item { quality, .. }
                        if mode == LootMode::GroupLoot && quality >= threshold =>
                    {
                        let eligibility = self.load_eligibility(&members, &content, source);
                        let snapshot = self.rolls.start(corpse, pool_index, content, eligibility);
                        self.send(&snapshot);
                        pool.pending += 1;
                    }
                    LootContent::Item { .. } if mode == LootMode::GroupLoot => {
                        personal
                            .looters
                            .get_mut(&owner)
                            .expect("member entry exists")
                            .push(Some(content));
                    }
                    _ => {}
                }
            }
            shared.0.push(pool);
        }
        personal
            .looters
            .retain(|_, slots| slots.iter().any(Option::is_some));
        (personal, shared)
    }

    fn load_eligibility(
        &self,
        members: &[u64],
        item: &LootContent,
        source: &LootRolls,
    ) -> Vec<(GroupLootPlayer, bool)> {
        let LootContent::Item { item_id, .. } = *item else {
            unreachable!("item roll")
        };
        let (db, templates) = source.store.get().expect("generation needs item store");
        let template = templates
            .get(shared::item_data::ItemDefinitionSource::Retail, item_id)
            .expect("rolled template exists");
        members
            .iter()
            .map(|&character_id| {
                let (_, player, known) = self
                    .players
                    .iter()
                    .find(|(id, _, _)| id.0 == character_id)
                    .expect("rewarded online player");
                let inventory =
                    load_model(db, templates, character_id).expect("rewarded character inventory");
                let goods = source.store.catalog.goods.get(&item_id);
                let owned =
                    inventory.owned_count(shared::item_data::ItemDefinitionSource::Retail, item_id);
                let max_count = goods.map_or(0, |goods| goods.max_count);
                let professions = persistence::load_professions(db, character_id);
                let context = NeedContext {
                    item_id,
                    template,
                    player,
                    owned,
                    max_count,
                    known,
                    professions: &professions,
                    metadata: &templates.loot_need,
                };
                let need_reason = need_refusal(&context);
                let account = self
                    .accounts
                    .iter()
                    .find(|(world, _)| world.character_id == character_id)
                    .map(|(_, account)| account.account_id);
                let can_transmog = template.appearance_id.is_some_and(|appearance| {
                    account.is_some_and(|account| {
                        !crate::persistence::appearance_store::load_account_appearances(db, account)
                            .has(appearance)
                    })
                });
                // Warbands Transmog preference is class/usage eligibility, not
                // item-specific Need restrictions (unique limit or greed-only).
                let preferred_transmog =
                    can_transmog && class_can_need(&context) && has_required_profession(&context);
                (
                    GroupLootPlayer {
                        character_id,
                        name: player.name.clone(),
                        can_need: need_reason.is_none(),
                        need_reason,
                        can_transmog,
                        response: None,
                    },
                    preferred_transmog,
                )
            })
            .collect()
    }

    fn send(&mut self, snapshot: &GroupLootRoll) {
        for (_, world, mut sender) in &mut self.senders {
            if snapshot
                .players
                .iter()
                .any(|player| player.character_id == world.character_id)
            {
                sender.send::<LootChannel>(snapshot.clone());
            }
        }
    }
}

const CAN_ONLY_ROLL_GREED: i64 = 0x100; // ItemTemplate.h:223
const ITEM_CLASS_WEAPON: u8 = 2;
const ITEM_CLASS_ARMOR: u8 = 4;
const INVTYPE_CLOAK: u8 = 16;

struct NeedContext<'a> {
    item_id: u32,
    template: &'a crate::item_templates::ItemTemplate,
    player: &'a Player,
    metadata: &'a crate::item_templates::LootNeedMetadata,
    professions: &'a crate::profession::PlayerProfessions,
    known: Option<&'a KnownSpells>,
    owned: u32,
    max_count: u32,
}

fn need_refusal(context: &NeedContext) -> Option<String> {
    if context.template.scaling.flags2 & CAN_ONLY_ROLL_GREED != 0 {
        return Some("Need rolls are disabled for this item.".into());
    }
    if context.max_count > 0 && context.owned >= context.max_count {
        return Some("You already have the maximum amount of this item.".into());
    }
    if !class_can_need(context) {
        return Some("Your class may not roll need on this item.".into());
    }
    if !has_required_profession(context) {
        return Some("You do not have the profession required to use this item.".into());
    }
    None
}

fn has_required_profession(context: &NeedContext) -> bool {
    context
        .metadata
        .required_skills
        .get(&context.item_id)
        .is_none_or(|&(skill, rank)| context.professions.rank(skill) >= rank.max(1))
}

fn class_can_need(context: &NeedContext) -> bool {
    let template = context.template;
    if !template.allows(context.player.class, context.player.race) {
        return false;
    }
    if !matches!(
        template.scaling.class_id,
        ITEM_CLASS_ARMOR | ITEM_CLASS_WEAPON
    ) {
        return true;
    }
    let subclass_mask = 1u32 << template.scaling.subclass_id;
    match template.scaling.class_id {
        ITEM_CLASS_ARMOR if template.inventory_type != INVTYPE_CLOAK => {
            let armor = context
                .metadata
                .armor_masks
                .get(&context.player.class)
                .expect("active Retail class armor mask");
            armor & subclass_mask != 0
        }
        ITEM_CLASS_WEAPON => context.known.is_some_and(|known| {
            known.iter().any(|spell| {
                context
                    .metadata
                    .weapon_proficiencies
                    .get(&spell)
                    .is_some_and(|mask| mask & subclass_mask != 0)
            })
        }),
        _ => true,
    }
}

#[derive(SystemParam)]
pub(super) struct RollDelivery<'w, 's> {
    store: LootStore<'w>,
    commands: Commands<'w, 's>,
    corpses: Query<
        'w,
        's,
        (
            &'static mut CorpseLoot,
            &'static mut CorpseGroupLoot,
            &'static mut Corpse,
            &'static mut DeathState,
        ),
    >,
    senders: Query<'w, 's, (&'static InWorld, &'static mut MessageSender<GroupLootRoll>)>,
    failures: Query<'w, 's, (&'static InWorld, &'static mut MessageSender<LootFailed>)>,
    lootable: Query<'w, 's, (&'static InWorld, &'static mut MessageSender<CorpseLootable>)>,
}

impl RollDelivery<'_, '_> {
    fn send(&mut self, snapshot: &GroupLootRoll) {
        for (world, mut sender) in &mut self.senders {
            if snapshot
                .players
                .iter()
                .any(|player| player.character_id == world.character_id)
            {
                sender.send::<LootChannel>(snapshot.clone());
            }
        }
    }

    fn complete(&mut self, roll: &mut PendingRoll) {
        roll.snapshot.complete = true;
        roll.snapshot.seconds_remaining = 0.0;
        roll.snapshot.winner = roll.window.resolve();
        let item = roll.snapshot.item.clone();
        let refused = roll.snapshot.winner.and_then(|winner| {
            // Rolls exist only on creature corpses (`generate_corpse_loot`).
            self.store
                .take(winner, &item, super::LootKind::CORPSE)
                .err()
                .map(|error| (winner, error))
        });
        roll.snapshot.delivered = roll.snapshot.winner.is_some() && refused.is_none();
        let corpse = Entity::try_from_bits(roll.snapshot.corpse).expect("server entity bits");
        if let Ok((mut loot, mut shared, mut decay, mut death)) = self.corpses.get_mut(corpse) {
            let pool = &mut shared.0[roll.pool];
            pool.pending -= 1;
            if let Some((winner, _)) = refused {
                loot.looters.entry(winner).or_default().push(Some(item));
            } else if roll.snapshot.winner.is_none() {
                pool.slots.push(Some(item));
            }
            if !shared.has_pending_rolls() {
                self.commands.entity(corpse).remove::<PendingGroupLoot>();
            }
            if loot.looters.is_empty() && !shared.has_loot() {
                corpse_looted(&mut decay, &mut death);
            }
        }
        if let Some((winner, error)) = refused {
            for (world, mut sender) in &mut self.failures {
                if world.character_id == winner {
                    sender.send::<LootChannel>(LootFailed {
                        corpse: roll.snapshot.corpse,
                        error,
                    });
                }
            }
        }
        self.notify_finished_looters(corpse, &roll.snapshot);
        refresh_responses(roll);
        self.send(&roll.snapshot);
    }

    fn notify_finished_looters(&mut self, corpse: Entity, snapshot: &GroupLootRoll) {
        for (world, mut sender) in &mut self.lootable {
            if !snapshot
                .players
                .iter()
                .any(|player| player.character_id == world.character_id)
            {
                continue;
            }
            let remaining = self
                .corpses
                .get(corpse)
                .ok()
                .and_then(|(loot, shared, ..)| {
                    remaining_for(loot, Some(shared), world.character_id)
                });
            if remaining.is_none() {
                sender.send::<LootChannel>(CorpseLootable {
                    corpse: corpse.to_bits(),
                    lootable: false,
                });
            }
        }
    }
}

fn refresh_responses(roll: &mut PendingRoll) {
    roll.snapshot.seconds_remaining = roll.window.time_remaining.max(0.0);
    for player in &mut roll.snapshot.players {
        player.response = roll
            .window
            .responses
            .iter()
            .find(|entry| entry.player_id == player.character_id)
            .copied();
    }
}

pub(super) fn process_rolls(
    mut requests: Query<(&InWorld, &mut MessageReceiver<GroupLootChoice>)>,
    mut state: ResMut<GroupRolls>,
    mut rng: ResMut<GroupRollRng>,
    mut delivery: RollDelivery,
    time: Res<Time>,
) {
    for (world, mut receiver) in &mut requests {
        for request in receiver.receive() {
            let Some(roll) = state.active.get_mut(&request.roll_id) else {
                continue;
            };
            let value = if request.choice == LootRollChoice::Pass {
                0
            } else {
                (rng.0)()
            };
            if roll
                .window
                .respond(world.character_id, request.choice, value)
            {
                refresh_responses(roll);
                delivery.send(&roll.snapshot);
            }
        }
    }
    let mut completed = Vec::new();
    for (&id, roll) in &mut state.active {
        if roll.window.tick(time.delta_secs()) {
            delivery.complete(roll);
            completed.push(id);
        }
    }
    let snapshots: Vec<_> = state
        .active
        .values()
        .map(|roll| {
            let mut snapshot = roll.snapshot.clone();
            snapshot.seconds_remaining = roll.window.time_remaining.max(0.0);
            snapshot
        })
        .collect();
    for snapshot in snapshots {
        state.remember(&snapshot);
    }
    for id in completed {
        state.active.remove(&id);
    }
}

pub(super) fn replay_history(
    mut requests: Query<(
        &InWorld,
        &mut MessageReceiver<GroupLootHistoryRequest>,
        &mut MessageSender<GroupLootRoll>,
    )>,
    rolls: Res<GroupRolls>,
) {
    for (world, mut receiver, mut sender) in &mut requests {
        if receiver.receive().count() == 0 {
            continue;
        }
        if let Some(entries) = rolls.history.get(&world.character_id) {
            for entry in entries {
                sender.send::<LootChannel>(entry.clone());
            }
        }
    }
}
