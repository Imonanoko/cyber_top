use bevy::prelude::*;
use sha2::{Digest, Sha256};
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use std::path::PathBuf;

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
        .bind(&build.top.id)
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
            |(id, top_id, _weapon_id, _shaft_id, _chassis_id, _screw_id, note): (String, String, String, String, String, String, String)| {
                // For v0, return default build with correct IDs
                // top_id is used to look up BaseStats from registry (future)
                let mut top = crate::game::stats::base::BaseStats::default();
                top.id = top_id;
                Build {
                    id,
                    top,
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

    // ── Part CRUD (async) ──────────────────────────────────────────────

    pub async fn save_part_async(
        &self,
        slot: &str,
        kind: &str,
        id: &str,
        spec_json: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT OR REPLACE INTO parts (id, slot, kind, spec_json, balance_version) VALUES (?, ?, ?, ?, 1)",
        )
        .bind(id)
        .bind(slot)
        .bind(kind)
        .bind(spec_json)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn load_parts_by_slot_async(
        &self,
        slot: &str,
    ) -> Result<Vec<(String, String, String)>, sqlx::Error> {
        let rows: Vec<(String, String, String)> = sqlx::query_as(
            "SELECT id, kind, spec_json FROM parts WHERE slot = ?",
        )
        .bind(slot)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn delete_part_async(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM parts WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn load_all_builds_async(
        &self,
    ) -> Result<Vec<(String, String, String, String, String, String, String)>, sqlx::Error> {
        let rows: Vec<(String, String, String, String, String, String, String)> = sqlx::query_as(
            "SELECT id, top_id, weapon_id, shaft_id, chassis_id, screw_id, COALESCE(note, '') FROM builds",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn delete_build_async(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM builds WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ── Sync wrappers (use TokioRuntime resource) ──────────────────────

    pub fn save_part_sync(
        &self,
        rt: &tokio::runtime::Runtime,
        slot: &str,
        kind: &str,
        id: &str,
        spec_json: &str,
    ) -> Result<(), String> {
        rt.block_on(self.save_part_async(slot, kind, id, spec_json))
            .map_err(|e| e.to_string())
    }

    pub fn load_parts_by_slot_sync(
        &self,
        rt: &tokio::runtime::Runtime,
        slot: &str,
    ) -> Result<Vec<(String, String, String)>, String> {
        rt.block_on(self.load_parts_by_slot_async(slot))
            .map_err(|e| e.to_string())
    }

    pub fn delete_part_sync(
        &self,
        rt: &tokio::runtime::Runtime,
        id: &str,
    ) -> Result<(), String> {
        rt.block_on(self.delete_part_async(id))
            .map_err(|e| e.to_string())
    }

    pub fn save_build_sync(
        &self,
        rt: &tokio::runtime::Runtime,
        build: &Build,
    ) -> Result<(), String> {
        rt.block_on(self.save_build_async(build))
            .map_err(|e| e.to_string())
    }

    pub fn load_all_builds_sync(
        &self,
        rt: &tokio::runtime::Runtime,
    ) -> Result<Vec<(String, String, String, String, String, String, String)>, String> {
        rt.block_on(self.load_all_builds_async())
            .map_err(|e| e.to_string())
    }

    pub fn delete_build_sync(
        &self,
        rt: &tokio::runtime::Runtime,
        id: &str,
    ) -> Result<(), String> {
        rt.block_on(self.delete_build_async(id))
            .map_err(|e| e.to_string())
    }
}
