//! Test stepper for running lightyear client+server in-process.
//!
//! Uses crossbeam channels (no real UDP) and RawServer/RawClient connection layer.
//! Based on lightyear's own `ClientServerStepper` from `lightyear_tests`.

use bevy::MinimalPlugins;
use bevy::app::PluginsState;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy::time::TimeUpdateStrategy;
use core::net::{Ipv4Addr, SocketAddr};
use core::time::Duration;
use lightyear::prelude::client::*;
use lightyear::prelude::server::*;
use lightyear::prelude::*;
use lightyear_crossbeam::CrossbeamIo;
use lightyear_raw_connection::client::RawClient;
use lightyear_raw_connection::server::RawServer;
use shared::ProtocolPlugin;

const TICK_DURATION: Duration = Duration::from_millis(10);

/// In-process client-server test harness using crossbeam channels.
pub struct Stepper {
    pub client_apps: Vec<App>,
    pub server_app: App,
    pub client_entities: Vec<Entity>,
    pub server_entity: Entity,
    pub client_of_entities: Vec<Entity>,
    pub tick_duration: Duration,
    pub current_time: bevy::platform::time::Instant,
}

impl Stepper {
    /// Create a stepper with one connected client (raw connection, crossbeam IO).
    pub fn single_client() -> Self {
        let mut stepper = Self::new_server();
        stepper.add_client();
        stepper.init();
        stepper
    }

    /// Create a stepper with two connected clients.
    pub fn two_clients() -> Self {
        let mut stepper = Self::new_server();
        stepper.add_client();
        stepper.add_client();
        stepper.init();
        stepper
    }

    fn new_server() -> Self {
        let mut server_app = App::new();
        server_app.add_plugins((MinimalPlugins, TransformPlugin, StatesPlugin));
        server_app.add_plugins(server::ServerPlugins {
            tick_duration: TICK_DURATION,
        });
        server_app.add_plugins(ProtocolPlugin);

        let server_entity = server_app.world_mut().spawn(RawServer).id();

        Self {
            client_apps: vec![],
            server_app,
            client_entities: vec![],
            server_entity,
            client_of_entities: vec![],
            tick_duration: TICK_DURATION,
            current_time: bevy::platform::time::Instant::now(),
        }
    }

    fn add_client(&mut self) -> usize {
        let client_id = self.client_entities.len();
        let (crossbeam_client, crossbeam_server) = CrossbeamIo::new_pair();

        let mut client_app = Self::build_client_app(self.tick_duration);
        let client_entity = Self::spawn_client_entity(&mut client_app, crossbeam_client);
        let client_of_entity = self.spawn_client_of_entity(client_id, crossbeam_server);

        self.client_entities.push(client_entity);
        self.client_of_entities.push(client_of_entity);
        self.client_apps.push(client_app);
        client_id
    }

    fn build_client_app(tick_duration: Duration) -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, TransformPlugin, StatesPlugin));
        app.add_plugins(client::ClientPlugins { tick_duration });
        app.add_plugins(ProtocolPlugin);
        app.finish();
        app.cleanup();
        app
    }

    fn spawn_client_entity(app: &mut App, crossbeam_io: CrossbeamIo) -> Entity {
        app.world_mut()
            .spawn((
                RawClient,
                PingManager::new(PingConfig {
                    ping_interval: Duration::default(),
                }),
                ReplicationSender::default(),
                ReplicationReceiver::default(),
                crossbeam_io,
            ))
            .id()
    }

    fn spawn_client_of_entity(&mut self, client_id: usize, crossbeam_io: CrossbeamIo) -> Entity {
        self.server_app
            .world_mut()
            .spawn((
                LinkOf {
                    server: self.server_entity,
                },
                PingManager::new(PingConfig {
                    ping_interval: Duration::default(),
                }),
                ReplicationSender::default(),
                ReplicationReceiver::default(),
                Link::new(None),
                PeerAddr(SocketAddr::new(
                    core::net::IpAddr::V4(Ipv4Addr::LOCALHOST),
                    client_id as u16,
                )),
                // Crossbeam is instantly linked — no handshake
                Linked,
                crossbeam_io,
            ))
            .id()
    }

    fn init(&mut self) {
        self.finish_server_plugins();
        self.start_server_and_connect_clients();
        self.wait_for_connection();
    }

    fn finish_server_plugins(&mut self) {
        if matches!(
            self.server_app.plugins_state(),
            PluginsState::Ready | PluginsState::Adding
        ) {
            self.server_app.finish();
            self.server_app.cleanup();
        }
    }

    fn start_server_and_connect_clients(&mut self) {
        let now = bevy::platform::time::Instant::now();
        self.current_time = now;
        self.server_app
            .world_mut()
            .get_resource_mut::<Time<Real>>()
            .unwrap()
            .update_with_instant(now);

        // For crossbeam, the server has no real IO — manually mark it as Linked
        // so that RawServer's observer fires and inserts Started.
        self.server_app
            .world_mut()
            .entity_mut(self.server_entity)
            .insert(Linked);
        self.server_app.world_mut().trigger(Start {
            entity: self.server_entity,
        });
        self.server_app.world_mut().flush();

        for i in 0..self.client_entities.len() {
            self.client_apps[i]
                .world_mut()
                .get_resource_mut::<Time<Real>>()
                .unwrap()
                .update_with_instant(now);
            self.client_apps[i].world_mut().trigger(Connect {
                entity: self.client_entities[i],
            });
        }
    }

    /// Tick until all clients are connected (up to 50 ticks).
    fn wait_for_connection(&mut self) {
        for _ in 0..50 {
            let all_connected = (0..self.client_entities.len()).all(|i| {
                self.client_apps[i]
                    .world()
                    .entity(self.client_entities[i])
                    .contains::<Connected>()
            });
            if all_connected {
                return;
            }
            self.tick_step(1);
        }
        panic!("Clients failed to connect within 50 ticks");
    }

    /// Advance time and update all apps by n ticks.
    pub fn tick_step(&mut self, n: usize) {
        for _ in 0..n {
            self.current_time += self.tick_duration;
            for app in &mut self.client_apps {
                app.insert_resource(TimeUpdateStrategy::ManualInstant(self.current_time));
            }
            self.server_app
                .insert_resource(TimeUpdateStrategy::ManualInstant(self.current_time));

            for app in &mut self.client_apps {
                app.update();
            }
            self.server_app.update();
        }
    }

    /// Advance only the server, leaving client ACKs unread.
    pub fn server_tick_only(&mut self, n: usize) {
        for _ in 0..n {
            self.current_time += self.tick_duration;
            self.server_app
                .insert_resource(TimeUpdateStrategy::ManualInstant(self.current_time));
            self.server_app.update();
        }
    }

    /// Get the client entity reference in the client app.
    pub fn client(&self, id: usize) -> EntityRef<'_> {
        self.client_apps[id]
            .world()
            .entity(self.client_entities[id])
    }

    /// Get the server entity reference.
    pub fn server(&self) -> EntityRef<'_> {
        self.server_app.world().entity(self.server_entity)
    }

    /// Get the server-side "client_of" entity reference.
    pub fn client_of(&self, id: usize) -> EntityRef<'_> {
        self.server_app.world().entity(self.client_of_entities[id])
    }

    /// Get mutable access to the server app world.
    pub fn server_world_mut(&mut self) -> &mut World {
        self.server_app.world_mut()
    }

    /// Get mutable access to the client app world.
    pub fn client_world_mut(&mut self, id: usize) -> &mut World {
        self.client_apps[id].world_mut()
    }
}
