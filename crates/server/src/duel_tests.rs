use super::*;

#[test]
fn start_pending_rejects_self_duel() {
    let mut state = DuelServerState::default();
    let player = Entity::from_bits(1);

    let result = state.start_pending(player, player);

    assert_eq!(result, Err("cannot duel yourself"));
}

#[test]
fn build_duel_boundary_uses_midpoint() {
    let boundary = build_duel_boundary(
        &Position {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        &Position {
            x: 20.0,
            y: 0.0,
            z: 10.0,
        },
    );

    assert_eq!(boundary.center_x, 10.0);
    assert_eq!(boundary.center_z, 5.0);
    assert_eq!(boundary.radius, DUEL_BOUNDARY_RADIUS);
}

#[test]
fn resolve_active_duel_detects_out_of_bounds_loss() {
    let challenger = Entity::from_bits(1);
    let target = Entity::from_bits(2);
    let resolution = resolve_duel_state(
        &Position {
            x: 100.0,
            y: 0.0,
            z: 0.0,
        },
        &Position {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        100.0,
        100.0,
        DuelBoundary {
            center_x: 0.0,
            center_z: 0.0,
            radius: 30.0,
        },
        challenger,
        target,
    );

    assert_eq!(
        resolution,
        Some(DuelResolution::WinLoss {
            winner: target,
            loser: challenger,
            defeated: false,
        })
    );
}

#[test]
fn resolve_active_duel_detects_health_loss() {
    let challenger = Entity::from_bits(1);
    let target = Entity::from_bits(2);
    let resolution = resolve_duel_state(
        &Position {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        &Position {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        0.0,
        50.0,
        DuelBoundary {
            center_x: 0.0,
            center_z: 0.0,
            radius: 30.0,
        },
        challenger,
        target,
    );

    assert_eq!(
        resolution,
        Some(DuelResolution::WinLoss {
            winner: target,
            loser: challenger,
            defeated: true,
        })
    );
}

mod combat {
    //! Duel opponents attack each other with spells and melee only while the
    //! duel is in progress (AzerothCore `Unit::GetReactionTo` "duel - always
    //! hostile to opponent", `Unit::_IsValidAttackTarget` "check duel").

    use super::*;
    use crate::auth::InWorld;
    use crate::spell_cast::test_harness::{Harness, harness, spawn_player, varied};
    use lightyear::connection::host::HostClient;
    use shared::components::PowerType;
    use shared::protocol::{CombatLogEvent, CombatLogKind, MissKind};

    const MORTAL_STRIKE: u32 = 12294;

    struct Duel {
        h: Harness,
        opponent: Entity,
        opponent_connection: Entity,
    }

    fn duel_messages() -> impl Bundle {
        (
            MessageSender::<InitiateDuel>::default(),
            MessageReceiver::<InitiateDuel>::default(),
            MessageSender::<AcceptDuel>::default(),
            MessageReceiver::<AcceptDuel>::default(),
            MessageSender::<DuelStateUpdate>::default(),
            MessageReceiver::<DuelStateUpdate>::default(),
        )
    }

    /// Warrior caster and a second Human warrior at 3 yd (friendly factions).
    fn duel_harness() -> Option<Duel> {
        let mut h = harness(Vec::new(), &[MORTAL_STRIKE], 1)?;
        // The harness app is already finished; register the plugin directly.
        DuelPlugin.build(&mut h.app);
        let opponent = spawn_player(&mut h.app, 1, 3.0);
        let opponent_connection = h
            .app
            .world_mut()
            .spawn((
                RemoteId(PeerId::Local(2)),
                Connected,
                HostClient { buffer: Vec::new() },
                Authenticated { account_id: 2 },
                InWorld { character_id: 2 },
                PlayerLink(opponent),
                duel_messages(),
                MessageSender::<CombatLogEvent>::default(),
                MessageReceiver::<CombatLogEvent>::default(),
            ))
            .id();
        let connection = h.connection;
        h.app
            .world_mut()
            .entity_mut(connection)
            .insert(duel_messages());
        h.advance(1);
        h.set_power(PowerType::Rage, 1000.0);
        Some(Duel {
            h,
            opponent,
            opponent_connection,
        })
    }

    impl Duel {
        fn start(&mut self) {
            let (connection, opponent) = (self.h.connection, self.opponent);
            self.h
                .app
                .world_mut()
                .get_mut::<MessageSender<InitiateDuel>>(connection)
                .unwrap()
                .send::<DuelChannel>(InitiateDuel {
                    target_entity: Some(opponent.to_bits()),
                });
            self.h.advance(3);
            self.h
                .app
                .world_mut()
                .get_mut::<MessageSender<AcceptDuel>>(self.opponent_connection)
                .unwrap()
                .send::<DuelChannel>(AcceptDuel);
            self.h.advance(3);
            assert!(self.h.app.world().get::<DuelOpponent>(opponent).is_some());
        }

        fn damage_to_opponent(&self) -> Vec<i32> {
            self.h
                .inbox
                .log
                .iter()
                .filter(|e| {
                    e.kind == CombatLogKind::Damage && e.target == Some(self.opponent.to_bits())
                })
                .map(|e| e.amount)
                .collect()
        }
    }

    #[test]
    fn spells_cannot_hit_a_friendly_player_outside_a_duel() {
        let Some(mut d) = duel_harness() else {
            return;
        };
        let opponent = d.opponent;
        d.h.cast_and_deliver(MORTAL_STRIKE, Some(opponent));
        assert!(d.damage_to_opponent().is_empty());
        assert_eq!(d.h.inbox.failed.len(), 1);
        assert_eq!(d.h.health(opponent), 1000.0);
    }

    #[test]
    fn spells_damage_the_duel_opponent() {
        let Some(mut d) = duel_harness() else {
            return;
        };
        d.start();
        let opponent = d.opponent;
        d.h.cast_and_deliver(MORTAL_STRIKE, Some(opponent));
        // 5.2012 AP coefficient × 100 AP, ± 2.5% variance.
        let damage = d.damage_to_opponent();
        assert_eq!(damage.len(), 1);
        let damage = damage[0] as f32;
        assert!(varied(520.12).contains(&damage), "{damage}");
        assert_eq!(d.h.health(opponent), 1000.0 - damage);
        assert!(d.h.inbox.failed.is_empty());
    }

    #[test]
    fn melee_hits_the_duel_opponent() {
        let Some(mut d) = duel_harness() else {
            return;
        };
        d.start();
        let opponent = d.opponent;
        d.h.attack(opponent);
        d.h.advance(3);
        // One unarmed swing: 1-2 + 100 AP / 6 × 2.0 s = 34-35, or a 5% miss.
        let swings: Vec<_> =
            d.h.inbox
                .log
                .iter()
                .filter(|e| e.spell_id.is_none() && e.target == Some(opponent.to_bits()))
                .map(|e| (e.kind, e.amount))
                .collect();
        match swings[..] {
            [(CombatLogKind::Damage, amount)] => {
                assert!((34..=35).contains(&amount), "{amount}");
                assert_eq!(d.h.health(opponent), 1000.0 - amount as f32);
            }
            [(CombatLogKind::Miss(MissKind::Miss), _)] => {
                assert_eq!(d.h.health(opponent), 1000.0)
            }
            _ => panic!("{swings:?}"),
        }
    }

    /// Unit::DealDamage: the opponent's lethal hit leaves the loser at 1
    /// health, stops both duelists' combat and completes the duel won.
    #[test]
    fn lethal_duel_damage_leaves_the_loser_at_one_health_and_ends_the_duel() {
        let Some(mut d) = duel_harness() else {
            return;
        };
        d.start();
        let (caster, opponent) = (d.h.caster, d.opponent);
        d.h.app.world_mut().entity_mut(opponent).insert(Health {
            current: 300.0,
            max: 1000.0,
        });
        d.h.attack(opponent);
        d.h.advance(1);
        d.h.cast_and_deliver(MORTAL_STRIKE, Some(opponent));

        assert_eq!(d.h.health(opponent), 1.0);
        let log = std::mem::take(&mut d.h.inbox.log);
        assert!(log.iter().all(|e| e.kind != CombatLogKind::Death));
        let world = d.h.app.world();
        assert!(world.get::<DuelOpponent>(caster).is_none());
        assert!(world.get::<DuelOpponent>(opponent).is_none());
        assert!(world.get::<crate::combat::AutoAttack>(caster).is_none());
        // The duel's end stops both duelists' combat at once, not 5 s after the hit.
        for duelist in [caster, opponent] {
            assert_eq!(
                world.get::<shared::components::CombatStatus>(duelist),
                Some(&shared::components::CombatStatus(false))
            );
        }
        assert!(
            world
                .get::<crate::death::PlayerDeathState>(opponent)
                .is_none_or(|state| state.state == shared::death::DeathState::Alive)
        );

        // The duel is over: no more damage by spell or melee.
        d.h.cast_and_deliver(MORTAL_STRIKE, Some(opponent));
        d.h.advance(60);
        assert!(d.damage_to_opponent().is_empty());
        // Out of combat the loser regenerates again (Player::RegenerateHealth).
        assert!(d.h.health(opponent) >= 1.0);
    }

    /// TC Player::DuelComplete: only DUEL_WON credits WinDuel76/LoseDuel77; DUEL_FLED does not.
    #[test]
    fn achievements_criteria_beaten_duelist_credits_win_and_loss_but_fleeing_does_not() {
        use crate::achievement::{PlayerAchievementState, load_achievement_catalog};
        let Some(mut d) = duel_harness() else {
            return;
        };
        let conn = rusqlite::Connection::open(crate::test_data::world_db_path()).unwrap();
        d.h.app
            .insert_resource(load_achievement_catalog(&conn).unwrap());
        d.h.app.init_resource::<crate::maps::ServerClock>();
        let (caster, opponent) = (d.h.caster, d.opponent);
        for player in [caster, opponent] {
            d.h.app
                .world_mut()
                .entity_mut(player)
                .insert(PlayerAchievementState::default());
        }
        // Statistics Duels won319 (criterion3629) and Duels lost320 (3630).
        let counters = |d: &Duel| {
            [
                (caster, 3629),
                (caster, 3630),
                (opponent, 3629),
                (opponent, 3630),
            ]
            .map(|(player, criterion)| {
                d.h.app
                    .world()
                    .get::<PlayerAchievementState>(player)
                    .unwrap()
                    .criteria_progress
                    .get(&criterion)
                    .copied()
            })
        };
        d.start();
        d.h.app.world_mut().get_mut::<Position>(opponent).unwrap().x += 200.0;
        d.h.advance(3);
        assert!(d.h.app.world().get::<DuelOpponent>(opponent).is_none());
        assert_eq!(counters(&d), [None; 4]);
        d.h.app.world_mut().get_mut::<Position>(opponent).unwrap().x -= 200.0;
        d.start();
        d.h.app.world_mut().entity_mut(opponent).insert(Health {
            current: 300.0,
            max: 1000.0,
        });
        d.h.attack(opponent);
        d.h.advance(1);
        d.h.cast_and_deliver(MORTAL_STRIKE, Some(opponent));
        d.h.advance(3);
        assert_eq!(d.h.health(opponent), 1.0);
        assert_eq!(counters(&d), [Some(1), None, None, Some(1)]);
    }
}
