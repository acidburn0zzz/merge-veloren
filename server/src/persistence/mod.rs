//! DB operations and schema migrations
//!
//! This code uses several [`Diesel ORM`](http://diesel.rs/) tools for DB operations:
//! - [`diesel-migrations`](https://docs.rs/diesel_migrations/1.4.0/diesel_migrations/)
//!   for managing table migrations
//! - [`diesel-cli`](https://github.com/diesel-rs/diesel/tree/master/diesel_cli/)
//!   for generating and testing migrations

pub(in crate::persistence) mod character;
pub mod character_loader;
pub mod character_updater;
mod error;
mod json_models;
mod models;
mod schema;
pub(crate) mod connection;

extern crate diesel;

use common::comp;
use diesel_migrations::embed_migrations;
use tracing::{info};
use crate::persistence::connection::VelorenConnection;

/// A tuple of the components that are persisted to the DB for each character
pub type PersistedComponents = (comp::Body, comp::Stats, comp::Inventory, comp::Loadout);

// See: https://docs.rs/diesel_migrations/1.4.0/diesel_migrations/macro.embed_migrations.html
// This macro is called at build-time, and produces the necessary migration info
// for the `embedded_migrations` call below.
//
// NOTE: Adding a useless comment to trigger the migrations being run.  Delete
// when needed.
embed_migrations!();

struct TracingOut;

impl std::io::Write for TracingOut {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        info!("{}", String::from_utf8_lossy(buf));
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> { Ok(()) }
}

/// Runs any pending database migrations. This is executed during server startup
pub fn run_migrations(connection: VelorenConnection) -> Result<(), diesel_migrations::RunMigrationsError> {
    embedded_migrations::run_with_output(
        &connection.0,
        &mut std::io::LineWriter::new(TracingOut),
    )
}



// pub fn establish_connection(db_dir: &str) -> QueryResult<VelorenConnection> {
//     let db_dir = &apply_saves_dir_override(db_dir);
//     let database_url = format!("{}/db.sqlite", db_dir);
//
//     let connection = SqliteConnection::establish(&database_url)
//         .unwrap_or_else(|_| panic!("Error connecting to {}", database_url));
//
//     // Use Write-Ahead-Logging for improved concurrency: https://sqlite.org/wal.html
//     // Set a busy timeout (in ms): https://sqlite.org/c3ref/busy_timeout.html
//     connection
//         .batch_execute(
//             "
//         PRAGMA foreign_keys = ON;
//         PRAGMA journal_mode = WAL;
//         PRAGMA busy_timeout = 250;
//         ",
//         )
//         .expect(
//             "Failed adding PRAGMA statements while establishing sqlite connection, including \
//              enabling foreign key constraints.  We will not allow connecting to the server under \
//              these conditions.",
//         );
//
//     Ok(VelorenConnection(connection))
// }


