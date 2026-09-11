use std::{path::PathBuf, str::FromStr};

use crate::{
    addon::Addon,
    store::{GameVersion, path::MoxenPath},
};
use anyhow::{Context, Result};
use sqlx::{
    AssertSqlSafe,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions},
};

type DbEntry = (i32, String, String, i32, i32, i32);

#[derive(Debug, Clone)]
pub struct Entry {
    id: i32,
    name: String,
    slug: String,
    file_id: i32,
    mod_id: i32,
    file_name: i32,
}

impl Entry {
    pub fn from_db(entry: DbEntry) -> Self {
        Self {
            id: entry.0,
            name: entry.1,
            slug: entry.2,
            file_id: entry.3,
            mod_id: entry.4,
            file_name: entry.5,
        }
    }
}

pub struct MoxenDbStore {
    pool: SqlitePool,
    game_version: GameVersion,
}

impl MoxenDbStore {
    pub(crate) async fn load(version: GameVersion) -> Result<Self> {
        let root = MoxenPath::new()
            .context("retrieving moxen root directory")?
            .build();

        let db_name = PathBuf::from("sqlite:/").join(root).join("registry.db");
        let connection_opts = SqliteConnectOptions::from_str(&db_name.display().to_string())
            .context("loading connection options")?
            .optimize_on_close(true, None)
            .journal_mode(SqliteJournalMode::Wal)
            .create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .acquire_timeout(std::time::Duration::from_secs_f64(0.1))
            .connect_with(connection_opts)
            .await
            .context("creating database pool")?;

        Self::initialise(&pool)
            .await
            .context("initialising database")?;

        Ok(Self {
            pool,
            game_version: version,
        })
    }

    async fn initialise(pool: &SqlitePool) -> Result<()> {
        sqlx::migrate!("./migrations")
            .run(pool)
            .await
            .context("running migrations")?;

        Ok(())
    }

    pub async fn insert(&self, addon: &Addon) -> Result<()> {
        let query = format!(
            "insert into {}(id, name, slug, file_id, mod_id, file_name) values(?, ?, ?, ?, ?, ?) on conflict(id) do update set name = excluded.name, slug = excluded.slug, file_id = excluded.file_id, mod_id = excluded.mod_id, file_name = excluded.file_name",
            self.game_version
        );

        sqlx::query(AssertSqlSafe(query))
            .bind(&addon.id)
            .bind(&addon.name)
            .bind(&addon.slug)
            .bind(&addon.main_file.id)
            .bind(&addon.main_file.mod_id)
            .bind(&addon.main_file.file_name)
            .execute(&self.pool)
            .await
            .with_context(|| {
                format!(
                    "inserting addon {} ({}) into database",
                    addon.id, &addon.name
                )
            })?;

        Ok(())
    }

    pub async fn insert_cache(&self, addon_id: i32, file_id: i32, content: Vec<u8>) -> Result<()> {
        sqlx::query("insert into cache(mod_id, file_id, file) values(?, ?, ?) on conflict(mod_id) do update set file_id = excluded.file_id, file = excluded.file")
            .bind(addon_id)
            .bind(file_id)
            .bind(content)
            .execute(&self.pool)
            .await
            .with_context(|| format!("adding addon into the cache: {addon_id}"))?;

        Ok(())
    }

    pub async fn fetch_cache(&self, addon_id: i32) -> Result<Option<Vec<u8>>> {
        let content: Option<Vec<u8>> =
            sqlx::query_scalar::<_, Vec<u8>>("select file from cache where mod_id = ?")
                .bind(addon_id)
                .fetch_optional(&self.pool)
                .await
                .with_context(|| format!("fetching entry for {addon_id} from cache"))?;

        Ok(content)
    }

    pub async fn fetch(&self, id: i32) -> Result<Option<Entry>> {
        let query = format!("select * from {} where id = ?1", self.game_version);
        let entry: Option<DbEntry> = sqlx::query_as(AssertSqlSafe(query))
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .with_context(|| format!("fetching entry from {}", self.game_version))?;

        match entry {
            Some(entry) => Ok(Some(Entry::from_db(entry))),
            None => Ok(None),
        }
    }

    pub async fn fetch_all(&self) -> Result<Vec<Entry>> {
        let query = format!("select * from {}", self.game_version);
        let entries: Vec<DbEntry> = sqlx::query_as::<_, DbEntry>(AssertSqlSafe(query))
            .fetch_all(&self.pool)
            .await
            .with_context(|| {
                format!(
                    "retrieving all addons from the {} version",
                    self.game_version
                )
            })?;

        let addons = entries
            .into_iter()
            .map(|entry| Entry::from_db(entry))
            .collect::<Vec<Entry>>();

        Ok(addons)
    }
}
