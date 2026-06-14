/// Hermes 兼容性测试 (T036)
///
/// 测试策略：Hermes 生成测试数据（技能文件、记忆文件、会话SQLite），
/// Aether 读取并验证解析结果一致。
///
/// 这些测试需要 Hermes Agent 的安装路径，默认在 ../hermes/
use std::path::Path;

const HERMES_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../hermes");

/// 检查 Hermes 源码是否存在
#[test]
fn test_hermes_source_exists() {
    let hermes = Path::new(HERMES_ROOT);
    assert!(hermes.exists(), "Hermes 源码不存在于 {}", HERMES_ROOT);
    assert!(hermes.join("run_agent.py").exists(), "Hermes run_agent.py 不存在");
}

/// 检查 agentskills.io 格式的技能文件能否被解析
#[test]
fn test_skill_format_compat() {
    let hermes_skills = Path::new(HERMES_ROOT).join("skills");
    if !hermes_skills.exists() {
        return;
    }
    // 遍历 Hermes 的 SKILL.md 文件，检查 agentskills.io 格式
    for entry in walkdir(&hermes_skills, 3) {
        if entry.file_name().map(|n| n.to_string_lossy() == "SKILL.md").unwrap_or(false) {
            let content = std::fs::read_to_string(&entry).unwrap_or_default();
            assert!(
                content.starts_with("---"),
                "SKILL.md {} 缺少 frontmatter",
                entry.display()
            );
        }
    }
}

/// 简单目录遍历
fn walkdir(dir: &Path, max_depth: u32) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = dir.read_dir() {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && max_depth > 0 {
                files.extend(walkdir(&path, max_depth - 1));
            } else if path.is_file() {
                files.push(path);
            }
        }
    }
    files
}
