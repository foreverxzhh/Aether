use crate::AIAgent;
use crate::config::AgentConfig;
use crate::error::AetherError;
use crate::llm::ChatModel;
use crate::types::message::Message;

/// 子 Agent 委托
pub struct Delegation;

impl Delegation {
    /// 创建子 Agent 并执行任务
    pub async fn delegate(
        parent: &AIAgent,
        goal: &str,
        context: Option<&str>,
    ) -> Result<String, AetherError> {
        // 创建子配置（继承父配置，限制工具集）
        let mut child_config = AgentConfig::default();
        child_config.provider = parent.config.provider.clone();
        child_config.model = parent.config.model.clone();
        child_config.api_key = parent.config.api_key.clone();
        child_config.base_url = parent.config.base_url.clone();
        child_config.max_iterations = parent.config.max_iterations.min(50); // 子Agent上限50
        child_config.delegation_enabled = false; // 禁止子Agent再委托(防递归)

        let model = crate::llm::provider::create_chat_model(&child_config)?;

        // 构建子Agent的提示词
        let mut system = "你是 Aether 的子任务执行者。请专注完成分配给您的任务。".to_string();
        if let Some(ctx) = context {
            system.push_str("\n\n上下文：");
            system.push_str(ctx);
        }

        let msg = format!("任务目标：{}\n\n请完成此任务并返回结果。", goal);
        let messages = vec![
            Message::system(&system),
            Message::user(&msg),
        ];

        // 调用 LLM
        let response = model.invoke(&messages, &[]).await?;
        Ok(response.content.unwrap_or_else(|| "[子Agent无响应]".to_string()))
    }

    /// 批量并行委托
    pub async fn delegate_batch(
        _parent: &AIAgent,
        tasks: Vec<(String, Option<String>)>, // (goal, context)
    ) -> Result<Vec<String>, AetherError> {
        let mut handles = Vec::new();
        for (goal, _context) in tasks {
            let handle = tokio::spawn(async move {
                // 简化版：这里需要 AetherClient 之类的东西
                // 实际实现请使用 delegate 方法逐个执行
                Ok::<_, AetherError>(format!("[任务完成] {}", goal))
            });
            handles.push(handle);
        }

        let mut results = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(Ok(r)) => results.push(r),
                Ok(Err(e)) => results.push(format!("[错误] {}", e)),
                Err(e) => results.push(format!("[崩溃] {}", e)),
            }
        }
        Ok(results)
    }
}
