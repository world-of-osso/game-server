//! Mirror player identity changes into the independent replicated model component.
use bevy::prelude::*;
use lightyear::prelude::ReplicationDirtyQueue;
use shared::components::{CharacterVisual, Player};

pub(crate) fn sync_player_visuals(
    mut commands: Commands,
    dirty: Option<Res<ReplicationDirtyQueue>>,
    mut players: Query<(Entity, &Player, Option<&mut CharacterVisual>), Changed<Player>>,
) {
    for (entity, player, visual) in &mut players {
        let next = CharacterVisual::from(player);
        let changed = match visual {
            Some(mut visual) if *visual != next => {
                *visual = next;
                true
            }
            Some(_) => false,
            None => {
                commands.entity(entity).insert(next);
                true
            }
        };
        if changed && let Some(dirty) = &dirty {
            dirty.mark(entity);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::components::CharacterAppearance;

    #[test]
    fn player_visual_tracks_appearance_without_changing_follower_visuals() {
        let mut app = App::new();
        app.add_systems(Update, sync_player_visuals);
        let player = app
            .world_mut()
            .spawn(Player {
                name: "Ally".into(),
                race: 10,
                class: 2,
                appearance: CharacterAppearance {
                    sex: 1,
                    hair_style: 4,
                    ..Default::default()
                },
            })
            .id();
        let follower_visual = CharacterVisual {
            race: 3,
            class: 1,
            appearance: CharacterAppearance::default(),
        };
        let follower = app.world_mut().spawn(follower_visual.clone()).id();
        app.update();
        assert_eq!(
            app.world()
                .get::<CharacterVisual>(player)
                .unwrap()
                .appearance
                .hair_style,
            4
        );
        app.world_mut()
            .get_mut::<Player>(player)
            .unwrap()
            .appearance
            .hair_style = 7;
        app.update();
        assert_eq!(
            app.world()
                .get::<CharacterVisual>(player)
                .unwrap()
                .appearance
                .hair_style,
            7
        );
        assert_eq!(
            app.world().get::<CharacterVisual>(follower),
            Some(&follower_visual)
        );
        assert!(app.world().get::<Player>(follower).is_none());
    }
}
