extern crate anidb;

mod mock_server;

use anidb::Anidb;
use mock_server::MockServer;
use std::thread;
use std::time::{Duration, Instant};

fn setup(port: u16) {
    let server = MockServer::new(port).expect("Server setup failed");

    thread::spawn(move || {
        server.update();
    });
}

fn login_logout(mut db: Anidb) {
    db.login("foo", "bar").expect("Login failed");
    db.logout().expect("Logout failed");
}

#[test]
fn it_works() {
    let port = 4444u16;
    setup(port);

    let mut db = Anidb::new(("127.0.0.1", port)).unwrap();
    db.ratelimit = Duration::from_secs(0);
    login_logout(db);
}

#[test]
fn ratelimit_works() {
    let port = 4445u16;
    setup(port);

    let db = Anidb::new(("127.0.0.1", port)).unwrap();
    let before = Instant::now();
    login_logout(db);
    let after = Instant::now();
    assert!(after.duration_since(before) >= Duration::from_secs(8));
}
