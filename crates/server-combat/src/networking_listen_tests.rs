use super::*;
use std::net::UdpSocket;
use std::time::Duration;

fn assert_port_free(address: Res<ServerBindAddress>) {
    let socket = UdpSocket::bind(address.0).expect("port must be free during the first frame");
    drop(socket);
}

#[test]
fn server_binds_only_after_first_frame() {
    let probe = UdpSocket::bind("127.0.0.1:0").unwrap();
    let address = probe.local_addr().unwrap();
    drop(probe);

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: Duration::from_millis(50),
    });
    app.insert_resource(ServerBindAddress(address));
    configure_listener(&mut app);
    app.add_systems(
        Update,
        assert_port_free.run_if(bevy::ecs::schedule::common_conditions::run_once),
    );
    app.add_systems(
        PostUpdate,
        assert_port_free.run_if(bevy::ecs::schedule::common_conditions::run_once),
    );
    app.finish();
    app.cleanup();

    app.update();
    assert_eq!(
        UdpSocket::bind(address).unwrap_err().kind(),
        std::io::ErrorKind::AddrInUse
    );
    app.update();
    assert_eq!(
        UdpSocket::bind(address).unwrap_err().kind(),
        std::io::ErrorKind::AddrInUse
    );
    drop(app);
    UdpSocket::bind(address).expect("socket must close with the app");
}
