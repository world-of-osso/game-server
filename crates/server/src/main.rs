fn main() {
    let mut app = bevy::prelude::App::new();
    server::configure_app(&mut app);
    app.run();
}
