use crate::game::parts::Build;
use crate::game::stats::base::BaseStats;
use crate::game::stats::effective::EffectiveStats;

/// Repository trait for build/top/part data access.
pub trait BuildRepository: Send + Sync {
    fn load_build(&self, id: &str) -> Option<Build>;
    fn save_build(&self, build: &Build) -> Result<(), String>;
    fn list_builds(&self) -> Vec<String>;

    fn load_effective_cache(&self, build_id: &str, balance_version: u32) -> Option<EffectiveStats>;
    fn save_effective_cache(
        &self,
        build_id: &str,
        stats: &EffectiveStats,
        balance_version: u32,
    ) -> Result<(), String>;
}
