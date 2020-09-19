use diesel::{Connection, SqliteConnection};
use diesel::r2d2::{ConnectionManager, PooledConnection, Pool, PoolError, CustomizeConnection};
use std::{env, fs, path::PathBuf};
use tracing::{warn};
use diesel::connection::SimpleConnection;

/// A database connection blessed by Veloren.
pub struct VelorenConnection(pub(crate) PooledConnection<ConnectionManager<SqliteConnection>>);

/// A transaction blessed by Veloren.
#[derive(Clone, Copy)]
pub struct VelorenTransaction<'a>(&'a PooledConnection<ConnectionManager<SqliteConnection>>);

impl VelorenConnection {
    /// Open a transaction in order to be able to run a set of queries against
    /// the database. We require the use of a transaction, rather than
    /// allowing direct session access, so that (1) we can control things
    /// like the retry process (at a future date), and (2) to avoid
    /// accidentally forgetting to open or reuse a transaction.
    ///
    /// We could make things even more foolproof, but we restrict ourselves to
    /// this for now.
    pub fn transaction<T, E, F>(&mut self, f: F) -> Result<T, E>
        where
            F: for<'a> FnOnce(VelorenTransaction<'a>) -> Result<T, E>,
            E: From<diesel::result::Error>,
    {
        self.0.transaction(|| f(VelorenTransaction(&self.0)))
    }
}

impl<'a> core::ops::Deref for VelorenTransaction<'a> {
    type Target = PooledConnection<ConnectionManager<SqliteConnection>>;

    fn deref(&self) -> &Self::Target { &self.0 }
}

pub type SqlitePool = Pool<ConnectionManager<SqliteConnection>>;

#[derive(Clone)]
pub struct VelorenConnectionPool
{
    pool: SqlitePool
}

impl Default for VelorenConnectionPool{
    fn default() -> Self {
        unimplemented!()
    }
}
impl VelorenConnectionPool {
    pub fn new(db_dir: &str) -> diesel::QueryResult<Self> {
        let db_dir = &apply_saves_dir_override(db_dir);
        let _ = fs::create_dir(format!("{}/", db_dir));
        let database_url = format!("{}/db.sqlite", db_dir);
        let pool = VelorenConnectionPool::init_pool(&database_url).expect("failed to init pool");

        Ok(Self {
            pool
        })
    }

    pub fn get_connection(&self) -> VelorenConnection {
        VelorenConnection(self.pool.get().expect("Failed to get connection"))
    }

    fn init_pool(database_url: &str) -> Result<SqlitePool, PoolError> {
        let manager = ConnectionManager::<SqliteConnection>::new(database_url);
        Pool::builder().connection_customizer(Box::new(VelorenConnectionCustomizer::default())).build(manager)
    }
}

#[derive(Debug)]
struct VelorenConnectionCustomizer;

impl Default for VelorenConnectionCustomizer {
    fn default() -> Self {
        Self
    }
}
impl<C, E> CustomizeConnection<C, E> for VelorenConnectionCustomizer where C: SimpleConnection{
    fn on_acquire(&self, conn: &mut C) -> Result<(), E> {
        conn
            .batch_execute(
                "
        PRAGMA foreign_keys = ON;
        PRAGMA journal_mode = WAL;
        PRAGMA busy_timeout = 250;
        ",
            )
            .expect(
                "Failed adding PRAGMA statements while establishing sqlite connection, including \
             enabling foreign key constraints.  We will not allow connecting to the server under \
             these conditions.",
            );
        Ok(())
    }
}

fn apply_saves_dir_override(db_dir: &str) -> String {
    if let Some(saves_dir) = env::var_os("VELOREN_SAVES_DIR") {
        let path = PathBuf::from(saves_dir.clone());
        if path.exists() || path.parent().map(|x| x.exists()).unwrap_or(false) {
            // Only allow paths with valid unicode characters
            if let Some(path) = path.to_str() {
                return path.to_owned();
            }
        }
        warn!(?saves_dir, "VELOREN_SAVES_DIR points to an invalid path.");
    }
    db_dir.to_string()
}