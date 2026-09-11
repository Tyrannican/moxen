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

type DbEntry = (i32, String, String, String, i32, String);
type AddonUpdateEntry = (i32, String);
type CacheEntry = (i32, String, String, i32, String, Vec<u8>);

#[derive(Debug, Clone)]
pub struct Entry {
    pub id: i32,
    pub name: String,
    pub slug: String,
    pub summary: String,
    pub file_id: i32,
    pub file_name: String,
}

impl Entry {
    pub fn from_db(entry: DbEntry) -> Self {
        Self {
            id: entry.0,
            name: entry.1,
            slug: entry.2,
            summary: entry.3,
            file_id: entry.4,
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
            "insert into {}(id, name, slug, summary, file_id, file_name) values(?, ?, ?, ?, ?, ?) on conflict(id) do update set name = excluded.name, slug = excluded.slug, summary = excluded.summary, file_id = excluded.file_id, file_name = excluded.file_name",
            self.game_version
        );

        sqlx::query(AssertSqlSafe(query))
            .bind(&addon.id)
            .bind(&addon.name)
            .bind(&addon.slug)
            .bind(&addon.summary)
            .bind(&addon.main_file.id)
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

    pub async fn insert_cache(&self, addon: &Addon, content: Vec<u8>) -> Result<()> {
        sqlx::query("insert into cache(mod_id, slug, version, file_id, file_name, file) values(?, ?, ?, ?, ?, ?) on conflict(mod_id) do update set slug = excluded.slug, version = excluded.version, file_id = excluded.file_id, file_name = excluded.file_name, file = excluded.file")
            .bind(addon.id)
            .bind(&addon.slug)
            .bind(self.game_version.to_string())
            .bind(addon.main_file.id)
            .bind(&addon.main_file.file_name)
            .bind(content)
            .execute(&self.pool)
            .await
            .with_context(|| format!("adding addon into the cache: {}", addon.id))?;

        Ok(())
    }

    pub async fn fetch_cache(&self, addon_id: i32) -> Result<Option<CacheEntry>> {
        let content: Option<CacheEntry> =
            sqlx::query_as::<_, CacheEntry>("select * from cache where mod_id = ? and version = ?")
                .bind(addon_id)
                .bind(self.game_version.to_string())
                .fetch_optional(&self.pool)
                .await
                .with_context(|| format!("fetching entry for {addon_id} from cache"))?;

        Ok(content)
    }

    pub async fn clear_cache(&self) -> Result<()> {
        sqlx::query("delete from cache where version = ?")
            .bind(self.game_version.to_string())
            .execute(&self.pool)
            .await
            .with_context(|| format!("clearing cache for version {}", self.game_version))?;

        Ok(())
    }

    pub async fn fetch(&self, id: i32) -> Result<Option<Entry>> {
        let query = format!("select * from {} where id = ?1", self.game_version);
        let entry: Option<DbEntry> = sqlx::query_as(AssertSqlSafe(query))
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .with_context(|| format!("fetching entry ({id}) from {}", self.game_version))?;

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

    pub async fn set_updated(&self, addon_id: i32) -> Result<()> {
        sqlx::query("insert into updated(mod_id, version) values(?, ?)")
            .bind(addon_id)
            .bind(self.game_version.to_string())
            .execute(&self.pool)
            .await
            .with_context(|| {
                format!(
                    "setting update state of {addon_id} (version = {})",
                    self.game_version
                )
            })?;

        Ok(())
    }

    pub async fn fetch_updated(&self) -> Result<Vec<i32>> {
        let entries: Vec<AddonUpdateEntry> =
            sqlx::query_as::<_, AddonUpdateEntry>("select * from updated where version = ?")
                .bind(self.game_version.to_string())
                .fetch_all(&self.pool)
                .await
                .with_context(|| {
                    format!(
                        "retrieving updated addons from the {} version",
                        self.game_version
                    )
                })?;

        Ok(entries.into_iter().map(|a| a.0).collect())
    }

    pub async fn clear_updated_state(&self, addon_id: i32) -> Result<()> {
        sqlx::query("delete from updated where mod_id = ? and version = ?")
            .bind(addon_id)
            .bind(self.game_version.to_string())
            .execute(&self.pool)
            .await
            .with_context(|| {
                format!(
                    "removing update state of {addon_id} (version = {})",
                    self.game_version
                )
            })?;

        Ok(())
    }

    pub async fn clear_update_table(&self) -> Result<()> {
        sqlx::query("delete from updated where version = ?")
            .bind(self.game_version.to_string())
            .execute(&self.pool)
            .await
            .with_context(|| {
                format!(
                    "clearing update table of addons with version '{}'",
                    self.game_version
                )
            })?;

        Ok(())
    }
}
