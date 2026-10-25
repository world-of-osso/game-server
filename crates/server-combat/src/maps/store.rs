use rusqlite::Connection;
use server_map_rules::maps::store::load_map_store as read_map_store;
pub use server_map_rules::maps::store::*;

pub fn load_map_store(conn: &Connection) -> rusqlite::Result<MapStore> {
    #[cfg(any(test, feature = "test-support"))]
    return crate::test_data::catalog_cache::load_connection(conn, || read_map_store(conn));
    #[cfg(not(any(test, feature = "test-support")))]
    read_map_store(conn)
}
