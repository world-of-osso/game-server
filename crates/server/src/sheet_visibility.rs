//! Owner-only character sheet stats. `UnitStats`, `CombatRatings` and `DerivedStats` replicate with
//! the player entity, but only the owning connection sees them: Retail sends a
//! player's stat fields to that player alone (`UF::ActivePlayerData` owner fields).

use bevy::prelude::*;
use bevy_replicon::prelude::{AppVisibilityExt, VisibilityFilter};
use lightyear::prelude::ReplicationDirtyQueue;
use shared::components::{CombatRatings, DerivedStats, UnitStats};

use crate::networking::PlayerLink;

/// The connection that owns this player and alone sees its sheet stats.
/// A unit without it shows its stats to nobody.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
#[component(immutable)]
pub struct SheetOwner(pub Entity);

impl VisibilityFilter for SheetOwner {
    type ClientComponent = Self;
    type Scope = (UnitStats, CombatRatings, DerivedStats);

    fn is_visible(&self, client: Entity, _component: Option<&Self>) -> bool {
        client == self.0
    }
}

pub struct SheetVisibilityPlugin;

impl Plugin for SheetVisibilityPlugin {
    fn build(&self, app: &mut App) {
        app.add_visibility_filter::<SheetOwner>()
            .add_observer(own_linked_player_sheet);
    }
}

/// World entry (and a relink) gives the player's sheet stats to its connection.
fn own_linked_player_sheet(
    trigger: On<Insert, PlayerLink>,
    links: Query<&PlayerLink>,
    replication_dirty: Option<Res<ReplicationDirtyQueue>>,
    mut commands: Commands,
) {
    let connection = trigger.entity;
    let Ok(link) = links.get(connection) else {
        return;
    };
    commands.entity(link.0).insert(SheetOwner(connection));
    if let Some(dirty) = replication_dirty.as_deref() {
        dirty.mark_for_sender(link.0, connection);
    }
}
