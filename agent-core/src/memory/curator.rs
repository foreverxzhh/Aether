//! Curator — 技能生命周期管理

use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use crate::error::AetherError;

/// Curator 状态
#[derive(Debug, Serialize, Deserialize)]
pub struct CuratorState {
    pub last_run_at: Option<String>,
    pub paused: bool,
    pub run_count: u64,
}

impl Default for CuratorState {
    fn default() -> Self {
        Self { last_run_at: None, paused: false, run_count: 0 }
    }
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

/// 读取 Curator 状态
pub fn load_state(skills_dir: &Path) -> CuratorState {
    let state_path = skills_dir.join(".curator_state");
    if let Ok(content) = std::fs::read_to_string(&state_path) {
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        CuratorState::default()
    }
}

/// 保存 Curator 状态
pub fn save_state(skills_dir: &Path, state: &CuratorState) {
    let state_path = skills_dir.join(".curator_state");
    if let Ok(content) = serde_json::to_string_pretty(state) {
        std::fs::write(state_path, content).ok();
    }
}

/// 检查是否需要运行策展
pub fn should_run(skills_dir: &Path, config: &CuratorConfig) -> bool {
    let state = load_state(skills_dir);
    if state.paused {
        return false;
    }
    match &state.last_run_at {
        None => true,
        Some(last) => {
            if let Ok(last_time) = chrono::NaiveDateTime::parse_from_str(last, "%Y-%m-%d %H:%M:%S") {
                let elapsed = chrono::Utc::now().naive_utc() - last_time;
                elapsed.num_hours() >= config.interval_hours as i64
            } else {
                true
            }
        }
    }
}

/// 运行策展：审查技能状态，归档陈旧技能
pub fn run_curator(skills_dir: &Path, config: &CuratorConfig) -> Result<CuratorReport, AetherError> {
    let mut report = CuratorReport::default();

    if !skills_dir.exists() {
        return Ok(report);
    }

    let archive_dir = skills_dir.join(".archive");
    std::fs::create_dir_all(&archive_dir).ok();

    for entry in std::fs::read_dir(skills_dir).map_err(|e| {
        AetherError::IoError(format!("读取技能目录失败: {}", e))
    })? {
        let entry = entry.map_err(|e| AetherError::IoError(e.to_string()))?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }

        // 检查文件修改时间
        let metadata = std::fs::metadata(&path)
            .map_err(|e| AetherError::IoError(e.to_string()))?;
        let modified = metadata.modified()
            .map_err(|e| AetherError::IoError(e.to_string()))?;
        let modified_dt: chrono::DateTime<chrono::Utc> = modified.into();
        let age_days = chrono::Utc::now()
            .signed_duration_since(modified_dt)
            .num_days();

        if age_days >= config.archive_after_days as i64 {
            // 归档
            let dest = archive_dir.join(path.file_name().unwrap());
            std::fs::rename(&path, &dest).ok();
            report.archived.push(path.file_name().unwrap().to_string_lossy().to_string());
        } else if age_days >= config.stale_after_days as i64 {
            report.stale.push(path.file_name().unwrap().to_string_lossy().to_string());
        } else {
            report.active += 1;
        }
    }

    // 更新状态
    let mut state = load_state(skills_dir);
    state.last_run_at = Some(chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string());
    state.run_count += 1;
    save_state(skills_dir, &state);

    Ok(report)
}

#[derive(Debug, Default)]
pub struct CuratorReport {
    pub active: u64,
    pub stale: Vec<String>,
    pub archived: Vec<String>,
}
