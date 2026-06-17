//! Curator — 技能生命周期管理 + 定时调度

use crate::error::AetherError;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct CuratorState {
    pub last_run_at: Option<String>,
    pub paused: bool,
    pub run_count: u64,
}

impl Default for CuratorState {
    fn default() -> Self {
        Self {
            last_run_at: None,
            paused: false,
            run_count: 0,
        }
    }
}

#[derive(Clone)]
pub struct CuratorConfig {
    pub interval_hours: u64,
    pub stale_after_days: u64,
    pub archive_after_days: u64,
}

impl Default for CuratorConfig {
    fn default() -> Self {
        Self {
            interval_hours: 168,
            stale_after_days: 30,
            archive_after_days: 90,
        }
    }
}

pub fn load_state(skills_dir: &Path) -> CuratorState {
    let p = skills_dir.join(".curator_state");
    std::fs::read_to_string(&p)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_state(skills_dir: &Path, state: &CuratorState) {
    if let Ok(s) = serde_json::to_string_pretty(state) {
        std::fs::write(skills_dir.join(".curator_state"), s).ok();
    }
}

pub fn should_run(skills_dir: &Path, config: &CuratorConfig) -> bool {
    let state = load_state(skills_dir);
    if state.paused {
        return false;
    }
    state.last_run_at.map_or(true, |last| {
        chrono::NaiveDateTime::parse_from_str(&last, "%Y-%m-%d %H:%M:%S")
            .ok()
            .map_or(true, |t| {
                (chrono::Utc::now().naive_utc() - t).num_hours() >= config.interval_hours as i64
            })
    })
}

pub fn run_curator(
    skills_dir: &Path,
    config: &CuratorConfig,
) -> Result<CuratorReport, AetherError> {
    let mut report = CuratorReport::default();
    if !skills_dir.exists() {
        return Ok(report);
    }

    let archive = skills_dir.join(".archive");
    std::fs::create_dir_all(&archive).ok();

    for entry in std::fs::read_dir(skills_dir).map_err(|e| AetherError::IoError(e.to_string()))? {
        let e = entry.map_err(|e| AetherError::IoError(e.to_string()))?;
        let path = e.path();
        if path.extension().and_then(|s| s.to_str()) != Some("md") {
            continue;
        }
        if let Ok(meta) = std::fs::metadata(&path) {
            if let Ok(mtime) = meta.modified() {
                let dt: chrono::DateTime<chrono::Utc> = mtime.into();
                let age = (chrono::Utc::now() - dt).num_days();
                let fname = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown");
                if age >= config.archive_after_days as i64 {
                    let dest = archive.join(fname);
                    std::fs::rename(&path, &dest).ok();
                    report.archived.push(fname.to_string());
                } else if age >= config.stale_after_days as i64 {
                    report.stale.push(fname.to_string());
                } else {
                    report.active += 1;
                }
            }
        }
    }

    let mut state = load_state(skills_dir);
    state.last_run_at = Some(chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string());
    state.run_count += 1;
    save_state(skills_dir, &state);
    Ok(report)
}

/// 启动后台 Curator 定时调度
pub fn start_curator_timer(skills_dir: std::path::PathBuf, config: CuratorConfig) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
            if should_run(&skills_dir, &config) {
                match run_curator(&skills_dir, &config) {
                    Ok(report) => tracing::info!(?report, "Curator 运行完成"),
                    Err(e) => tracing::warn!(%e, "Curator 运行失败"),
                }
            }
        }
    });
}

/// T-3.5: Inline curator check — 每次 chat 结束顺手判一下是否到期。
/// 不引入后台守护线程（跨平台 + SDK 形态友好）。
pub fn maybe_run_inline(skills_dir: &Path, config: &CuratorConfig) {
    if should_run(skills_dir, config) {
        match run_curator(skills_dir, config) {
            Ok(report) => tracing::info!(?report, "Curator 已运行"),
            Err(e) => tracing::warn!(%e, "Curator 运行失败"),
        }
    }
}

#[derive(Debug, Default)]
pub struct CuratorReport {
    pub active: u64,
    pub stale: Vec<String>,
    pub archived: Vec<String>,
}
