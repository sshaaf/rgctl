use crate::config::Config;
use crate::coolstore::CoolstoreState;
use sqlx::SqlitePool;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub config: Config,
    pub coolstore: CoolstoreState,
}
