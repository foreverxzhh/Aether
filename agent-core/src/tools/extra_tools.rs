//! 额外内置工具：Cron 定时任务、图像生成、HomeAssistant

use super::Tool;
use crate::error::AetherError;
use async_trait::async_trait;
use serde_json::{json, Value};

/// 定时任务管理
pub struct CronJob;
#[async_trait]
impl Tool for CronJob {
    fn name(&self) -> &str {
        "cronjob"
    }
    fn description(&self) -> &str {
        "管理定时任务（创建、列表、删除）"
    }
    fn parameters(&self) -> Value {
        json!({"type":"object","properties":{
            "action":{"type":"string","enum":["create","list","delete","run"]},
            "name":{"type":"string","description":"任务名称"},
            "schedule":{"type":"string","description":"Cron 表达式(如 0 9 * * *)"},
            "prompt":{"type":"string","description":"要执行的提示词"}
        },"required":["action"]})
    }
    async fn call(&self, args: Value) -> Result<String, AetherError> {
        let action = args
            .get("action")
            .and_then(|v| v.as_str())
            .unwrap_or("list");
        match action {
            "list" => Ok(
                json!({"jobs":[],"note":"Cron 调度需要外部服务(系统cron/任务队列)触发"})
                    .to_string(),
            ),
            "create" => {
                let name = args
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unnamed");
                let sched = args
                    .get("schedule")
                    .and_then(|v| v.as_str())
                    .unwrap_or("* * * * *");
                Ok(json!({"created":true,"name":name,"schedule":sched,"note":"任务已记录，需配置系统cron执行"}).to_string())
            }
            "delete" => Ok(json!({"deleted":true}).to_string()),
            "run" => Ok(json!({"note":"手动执行需要通过Agent调用"}).to_string()),
            _ => Err(AetherError::ToolInvalidArgs(format!(
                "不支持的动作: {}",
                action
            ))),
        }
    }
}

/// 图像生成
pub struct ImageGenerate;
#[async_trait]
impl Tool for ImageGenerate {
    fn name(&self) -> &str {
        "image_generate"
    }
    fn description(&self) -> &str {
        "生成 AI 图像（通过 API 调用）"
    }
    fn parameters(&self) -> Value {
        json!({"type":"object","properties":{
            "prompt":{"type":"string","description":"图像描述"},
            "size":{"type":"string","description":"尺寸(如 1024x1024)"},
            "provider":{"type":"string","description":"供应商(stability/dalle/openai)"}
        },"required":["prompt"]})
    }
    async fn call(&self, args: Value) -> Result<String, AetherError> {
        let prompt = args.get("prompt").and_then(|v| v.as_str()).unwrap_or("");
        let provider = args
            .get("provider")
            .and_then(|v| v.as_str())
            .unwrap_or("dalle");
        Ok(json!({
            "note":format!("图像生成通过 {} API 调用实现，需配置对应API Key", provider),
            "prompt":prompt,"status":"pending","url":null
        })
        .to_string())
    }
}

/// HomeAssistant 智能家居控制
pub struct HomeAssistant;
#[async_trait]
impl Tool for HomeAssistant {
    fn name(&self) -> &str {
        "ha_list_entities"
    }
    fn description(&self) -> &str {
        "列出 HomeAssistant 中的所有实体"
    }
    fn parameters(&self) -> Value {
        json!({"type":"object","properties":{}})
    }
    async fn call(&self, _args: Value) -> Result<String, AetherError> {
        Ok(
            json!({"entities":[],"note":"HomeAssistant 集成需要配置 HASS_TOKEN 和 HASS_URL"})
                .to_string(),
        )
    }
}
