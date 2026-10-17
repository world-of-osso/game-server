//! TC Guild.cpp2322/3002: guild membership derives perk knowledge, not class learning.
use crate::{auth::Authenticated, class_progression::KnownSpells, networking::PlayerLink};
use bevy::prelude::*;
use lightyear::prelude::MessageSender;
use shared::{
    components::GuildMembership,
    protocol::{SpellsLearned, SpellsUnlearned, TalentChannel},
};
use std::{
    collections::{BTreeSet, HashMap},
    path::Path,
};

#[derive(Resource)]
struct PerkSpells(Vec<u32>);

pub struct GuildPerksPlugin(Vec<u32>);
impl GuildPerksPlugin {
    pub fn load(root: &Path) -> Result<Self, String> {
        shared::guild_perks::load_guild_perks(root)
            .map(|rows| Self(rows.into_iter().map(|row| row.spell_id).collect()))
    }
}
impl Plugin for GuildPerksPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PerkSpells(self.0.clone()))
            // See membership mutations and deferred component inserts/removals from Update.
            .add_systems(PostUpdate, reconcile_perks);
    }
}

type ChangedPlayers<'w, 's> =
    Query<'w, 's, Entity, Or<(Changed<GuildMembership>, Changed<KnownSpells>)>>;
type PlayerKnowledge<'w, 's> =
    Query<'w, 's, (Option<&'static GuildMembership>, &'static mut KnownSpells)>;

fn reconcile_perks(
    catalog: Res<PerkSpells>,
    mut removed: RemovedComponents<GuildMembership>,
    mut queries: ParamSet<(ChangedPlayers, PlayerKnowledge)>,
    connections: Query<(Entity, &PlayerLink), With<Authenticated>>,
    mut learned: Query<&mut MessageSender<SpellsLearned>>,
    mut unlearned: Query<&mut MessageSender<SpellsUnlearned>>,
) {
    let mut changed: BTreeSet<_> = queries.p0().iter().collect();
    changed.extend(removed.read());
    if changed.is_empty() {
        return;
    }
    let owners: HashMap<_, _> = connections
        .iter()
        .map(|(connection, link)| (link.0, connection))
        .collect();
    let mut players = queries.p1();
    for entity in changed {
        let Ok((guild, mut known)) = players.get_mut(entity) else {
            continue;
        };
        let desired = if guild.is_some() {
            catalog.0.as_slice()
        } else {
            &[]
        };
        if known.guild_grants() == desired {
            continue;
        }
        let before = known.iter().collect();
        known.set_guild_grants(desired);
        let after = known.iter().collect();
        if let Some(&connection) = owners.get(&entity) {
            publish_changes(connection, &before, &after, &mut learned, &mut unlearned);
        }
    }
}

fn publish_changes(
    connection: Entity,
    before: &BTreeSet<u32>,
    after: &BTreeSet<u32>,
    learned: &mut Query<&mut MessageSender<SpellsLearned>>,
    unlearned: &mut Query<&mut MessageSender<SpellsUnlearned>>,
) {
    let additions: Vec<_> = after.difference(before).copied().collect();
    let removals: Vec<_> = before.difference(after).copied().collect();
    if !additions.is_empty() {
        learned
            .get_mut(connection)
            .expect("authenticated perk owner requires SpellsLearned sender")
            .send::<TalentChannel>(SpellsLearned { spells: additions });
    }
    if !removals.is_empty() {
        unlearned
            .get_mut(connection)
            .expect("authenticated perk owner requires SpellsUnlearned sender")
            .send::<TalentChannel>(SpellsUnlearned { spells: removals });
    }
}
