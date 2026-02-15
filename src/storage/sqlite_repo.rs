use bevy::prelude::*;
use sha2::{Digest, Sha256};
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use std::path::PathBuf;

use super::repo::BuildRepository;
use crate::game::parts::Build;
use crate::game::stats::effective::EffectiveStats;

/// SQLite-backed repository (Bevy Resource).
#[derive(Resource)]
pub struct SqliteRepo {
    pool: SqlitePool,
}

impl SqliteRepo {
    pub async fn new(db_path: &PathBuf) -> Result<Self, sqlx::Error> {
        if let Some(parent) = db_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let url = format!("sqlite://{}?mode=rwc", db_path.display());
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&url)
            .await?;

        // Run migrations
        sqlx::migrate!("./migrations").run(&pool).await?;

        Ok(Self { pool })
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn save_build_async(&self, build: &Build) -> Result<(), sqlx::Error> {
        let spec_json =
            serde_json::to_string(build).unwrap_or_default();
        let weapon_id = &build.weapon.id;
        let shaft_id = &build.shaft.id;
        let chassis_id = &build.chassis.id;
        let screw_id = &build.screw.id;
        let note = build.note.as_deref().unwrap_or("");

        sqlx::query(
            r#"INSERT OR REPLACE INTO builds (id, top_id, weapon_id, shaft_id, chassis_id, screw_id, note)
               VALUES (?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(&build.id)
        .bind(&build.top_id)
        .bind(weapon_id)
        .bind(shaft_id)
        .bind(chassis_id)
        .bind(screw_id)
        .bind(note)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn load_build_async(&self, id: &str) -> Result<Option<Build>, sqlx::Error> {
        let row: Option<(String, String, String, String, String, String, String)> = sqlx::query_as(
            r#"SELECT id, top_id, weapon_id, shaft_id, chassis_id, screw_id, COALESCE(note, '')
               FROM builds WHERE id = ?"#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(
            |(id, top_id, _weapon_id, _shaft_id, _chassis_id, _screw_id, note)| {
                // For v0, return default build with correct IDs
                Build {
                    id,
                    top_id,
                    note: if note.is_empty() {
                        None
                    } else {
                        Some(note)
                    },
                    ..Default::default()
                }
            },
        ))
    }

    pub async fn save_effective_cache_async(
        &self,
        build_id: &str,
        stats: &EffectiveStats,
        balance_version: u32,
    ) -> Result<(), sqlx::Error> {
        let stats_json = serde_json::to_string(stats).unwrap_or_default();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let mut hasher = Sha256::new();
        hasher.update(stats_json.as_bytes());
        let hash = hex::encode(hasher.finalize());

        sqlx::query(
            r#"INSERT OR REPLACE INTO effective_cache (build_id, effective_stats_json, computed_at, balance_version, hash)
               VALUES (?, ?, ?, ?, ?)"#,
        )
        .bind(build_id)
        .bind(&stats_json)
        .bind(now)
        .bind(balance_version as i64)
        .bind(&hash)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn load_effective_cache_async(
        &self,
        build_id: &str,
        balance_version: u32,
    ) -> Result<Option<EffectiveStats>, sqlx::Error> {
        let row: Option<(String, i64)> = sqlx::query_as(
            r#"SELECT effective_stats_json, balance_version FROM effective_cache WHERE build_id = ?"#,
        )
        .bind(build_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.and_then(|(json, ver)| {
            if ver as u32 != balance_version {
                return None;
            }
            serde_json::from_str(&json).ok()
        }))
    }
}
