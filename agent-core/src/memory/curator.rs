//! Curator - 技能策展人（TODO: Phase 6 实现）

use std::path::Path;
use crate::error::AetherError;

/// 技能生命周期状态
pub enum SkillState {
    Active,
    Stale,
    Archived,
}

/// 策展人配置
pub struct CuratorConfig {
    pub interval_hours: u64,
    pub stale_after_days: u64,
    pub archive_after_days: u64,
}

impl Default for CuratorConfig {
    fn default() -> Self {
        Self {
            interval_hours: 168,     // 7天
            stale_after_days: 30,
            archive_after_days: 90,
        }
    }
}

/// 运行策展（检查技能状态，归档陈旧技能）
pub async fn run_curator(
    _hermes_home: &Path,
    _config: &CuratorConfig,
) -> Result<(), AetherError> {
    // TODO: Phase 6 实现
    Ok(())
}
