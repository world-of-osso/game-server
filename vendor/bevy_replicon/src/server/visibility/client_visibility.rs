use bevy::{
    ecs::entity::{EntityHashMap, EntityHashSet},
    platform::collections::hash_map::Entry,
    prelude::*,
};

use super::filters_mask::{FilterBit, FiltersMask};

/// Visibility as masks for a client.
///
/// Each bit corresponds to a visibility filter registered in the
/// [`FilterRegistry`](super::registry::FilterRegistry). This allows
/// us to avoid storing the filter data for every client.
///
/// Stores only entities that have some hidden data.
///
/// Automatically updated by observers from [`AppVisibilityExt`](super::AppVisibilityExt).
#[derive(Component, Reflect, Default)]
pub struct ClientVisibility {
    default_hidden: Option<FilterBit>,
    shown: EntityHashSet,
    /// Entities with hidden data.
    hidden: EntityHashMap<FiltersMask>,

    /// Entities and bits that became set during this tick.
    ///
    /// Stored redundantly to quickly iterate only over entities
    /// with newly hidden data.
    lost: EntityHashMap<FiltersMask>,
}

impl ClientVisibility {
    /// New clients may opt into an entity-scope whitelist before replication begins.
    /// Unknown and subsequently spawned entities stay hidden without per-entity writes.
    pub fn hide_by_default(&mut self, bit: FilterBit) {
        assert!(
            self.default_hidden.is_none(),
            "visibility default already configured"
        );
        self.default_hidden = Some(bit);
    }

    /// Explicit entity-scope grants when this client uses default-hidden visibility.
    pub fn explicitly_visible(&self) -> Option<impl Iterator<Item = Entity> + '_> {
        self.default_hidden.map(|_| self.shown.iter().copied())
    }

    /// Returns iterator over all entities that lose any visibility during this tick.
    pub(crate) fn iter_lost(&self) -> impl Iterator<Item = (Entity, FiltersMask)> {
        self.lost.iter().map(|(&e, &m)| (e, m))
    }

    /// Clears all entities that lost any visibility during this tick, returning them as an iterator.
    pub(crate) fn drain_lost(&mut self) -> impl Iterator<Item = (Entity, FiltersMask)> {
        self.lost.drain()
    }

    /// Removes a despawned entity tracked by this client.
    ///
    /// Since observers can't be ordered, we can't distinguish between
    /// a despawn and removal of a visibility filter. As a workaround, we record
    /// all changes and remove all despawned entities when processing despawns
    /// during replication.
    pub(crate) fn remove_despawned(&mut self, entity: Entity) {
        self.hidden.remove(&entity);
        self.lost.remove(&entity);
        self.shown.remove(&entity);
    }

    /**
    Sets the visibility for a [`VisibilityScope`](crate::shared::replication::visibility::VisibilityScope) by updating
    the associated filter bit on the given entity.

    Registered [`VisibilityFilter`](super::VisibilityFilter)s automatically call this method
    on insertion and removal.

    This method can also be used to manually control visibility for bits registered via
    [`FilterRegistry::register_scope`](super::registry::FilterRegistry::register_scope).
    This is useful when visibility needs to be updated only for specific entities or clients,
    because re-inserting a filter component would trigger recalculation of
    [`VisibilityFilter::is_visible`](super::VisibilityFilter::is_visible)
    for all affected entities.

    # Examples

    Update visibility based on distance:

    ```
    use bevy::prelude::*;
    use bevy_replicon::{
        server::visibility::{
            client_visibility::ClientVisibility, filters_mask::FilterBit, registry::FilterRegistry,
        },
        shared::replication::registry::ReplicationRegistry,
    };
    # let mut app = App::new();
    # app.add_systems(Update, update_range_visibility);

    fn update_range_visibility(
        mid_bit: Res<MidRangeBit>,
        far_bit: Res<FarRangeBit>,
        players: Query<(Entity, Ref<GlobalTransform>, &Owner)>,
        mut clients: Query<&mut ClientVisibility>,
    ) {
        for [
            (player_a, transform_a, &owner_a),
            (player_b, transform_b, &owner_b),
        ] in players.iter_combinations::<2>()
        {
            let [mut visibility_a, mut visibility_b] =
                clients.get_many_mut([*owner_a, *owner_b]).unwrap();

            let distance = transform_a
                .translation()
                .distance(transform_b.translation());

            let mid_range = distance <= 500.0;
            let far_range = distance <= 1000.0;

            visibility_a.set(player_b, **mid_bit, mid_range);
            visibility_b.set(player_a, **mid_bit, mid_range);
            visibility_a.set(player_b, **far_bit, far_range);
            visibility_b.set(player_a, **far_bit, far_range);
        }
    }

    #[derive(Component, Deref, Clone, Copy)]
    struct Owner(Entity);

    /// Visibility for [`Health`] and [`Stats`].
    #[derive(Resource, Deref)]
    struct MidRangeBit(FilterBit);

    impl FromWorld for MidRangeBit {
        fn from_world(world: &mut World) -> Self {
            let bit = world.resource_scope(|world, mut filter_registry: Mut<FilterRegistry>| {
                world.resource_scope(|world, mut registry: Mut<ReplicationRegistry>| {
                    filter_registry.register_scope::<(Health, Stats)>(world, &mut registry)
                })
            });
            Self(bit)
        }
    }

    #[derive(Component)]
    struct Health;

    #[derive(Component)]
    struct Stats {
        // ...
    }

    /// Visibility for the entire entity.
    #[derive(Resource, Deref)]
    struct FarRangeBit(FilterBit);

    impl FromWorld for FarRangeBit {
        fn from_world(world: &mut World) -> Self {
            let bit = world.resource_scope(|world, mut filter_registry: Mut<FilterRegistry>| {
                world.resource_scope(|world, mut registry: Mut<ReplicationRegistry>| {
                    filter_registry.register_scope::<Entity>(world, &mut registry)
                })
            });
            Self(bit)
        }
    }
    ```
    */
    pub fn set(&mut self, entity: Entity, bit: FilterBit, visible: bool) {
        if self.default_hidden == Some(bit) {
            if visible {
                self.shown.insert(entity);
            } else if !self.shown.remove(&entity) {
                return;
            }
        }
        if visible {
            if let Entry::Occupied(mut mask) = self.hidden.entry(entity) {
                mask.get_mut().remove(bit);
                if mask.get().is_empty() {
                    mask.remove();
                }

                if let Entry::Occupied(mut lost_mask) = self.lost.entry(entity) {
                    lost_mask.get_mut().remove(bit);
                    if lost_mask.get().is_empty() {
                        lost_mask.remove();
                    }
                }
            }
        } else {
            let mask = self.hidden.entry(entity).or_default();
            if !mask.contains(bit) {
                mask.insert(bit);
                self.lost.entry(entity).or_default().insert(bit);
            }
        }
    }

    /// Returns bits for all filters that affect visibility of the given entity.
    pub(crate) fn get(&self, entity: Entity) -> FiltersMask {
        let mut mask = self.hidden.get(&entity).copied().unwrap_or_default();
        if let Some(bit) = self.default_hidden
            && !self.shown.contains(&entity)
        {
            mask.insert(bit);
        }
        mask
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_hidden_tracks_visible_losses_and_other_component_filters() {
        let mut visibility = ClientVisibility::default();
        let entity = Entity::PLACEHOLDER;
        let bit = FilterBit::new(0);
        visibility.hide_by_default(bit);
        assert!(visibility.get(entity).contains(bit));
        visibility.set(entity, bit, true);
        assert!(visibility.get(entity).is_empty());
        visibility.set(entity, FilterBit::new(1), false);
        assert!(!visibility.get(entity).contains(bit));
        assert!(visibility.get(entity).contains(FilterBit::new(1)));
        visibility.set(entity, bit, false);
        assert!(visibility.get(entity).contains(bit));
        assert!(visibility.get(entity).contains(FilterBit::new(1)));
        assert!(
            visibility
                .iter_lost()
                .any(|(lost, mask)| lost == entity && mask.contains(bit))
        );
        visibility.set(entity, bit, true);
        visibility.set(entity, FilterBit::new(1), true);
        assert!(visibility.get(entity).is_empty());
        visibility.remove_despawned(entity);
        assert!(visibility.get(entity).contains(bit));
    }

    #[test]
    fn single() {
        let mut visibility = ClientVisibility::default();

        visibility.set(Entity::PLACEHOLDER, FilterBit::new(0), false);
        assert!(!visibility.get(Entity::PLACEHOLDER).is_empty());
        assert!(visibility.lost.contains_key(&Entity::PLACEHOLDER));

        visibility.set(Entity::PLACEHOLDER, FilterBit::new(0), true);
        assert!(visibility.get(Entity::PLACEHOLDER).is_empty());
        assert!(!visibility.lost.contains_key(&Entity::PLACEHOLDER));
    }

    #[test]
    fn multiple() {
        let mut visibility = ClientVisibility::default();

        visibility.set(Entity::PLACEHOLDER, FilterBit::new(0), false);
        visibility.set(Entity::PLACEHOLDER, FilterBit::new(1), false);
        assert!(!visibility.get(Entity::PLACEHOLDER).is_empty());
        assert!(visibility.lost.contains_key(&Entity::PLACEHOLDER));

        visibility.set(Entity::PLACEHOLDER, FilterBit::new(0), true);
        assert!(!visibility.get(Entity::PLACEHOLDER).is_empty());
        assert!(visibility.lost.contains_key(&Entity::PLACEHOLDER));

        visibility.set(Entity::PLACEHOLDER, FilterBit::new(1), true);
        assert!(visibility.get(Entity::PLACEHOLDER).is_empty());
        assert!(!visibility.lost.contains_key(&Entity::PLACEHOLDER));
    }

    #[test]
    fn already_visible() {
        let mut visibility = ClientVisibility::default();
        assert!(visibility.get(Entity::PLACEHOLDER).is_empty());

        visibility.set(Entity::PLACEHOLDER, FilterBit::new(0), true);
        assert!(visibility.get(Entity::PLACEHOLDER).is_empty());
        assert!(!visibility.lost.contains_key(&Entity::PLACEHOLDER));
    }
}
