//! ZCode 适配器。
//!
//! ZCode 把供应商信息拆在**两份文件**里，两处都含明文 apiKey，所以应用时必须同时更新：
//!
//! - `v2/config.json` → `provider.<providerId>`：定义 + 模型
//! - `v2/provider_config.json` → `config`：顺序、api 类型、模型规则
//!
//! 只管理 `xiaobai_` 前缀的 provider；其余 provider、`providerOrder` 里的占位项
//! （如 `new-provider`）与所有未知字段原样保留。
//!
//! 写入是**幂等**的：同样的站点状态产出同样的两份内容。任一文件写失败时报错，用户
//! 重新应用一次即可收敛到一致状态（不做跨文件事务回滚，复杂度与收益不成正比）。

use crate::adapters::atomic::{atomic_write, backup_file, FileLock};
use crate::crypto::key_fingerprint;
use crate::domain::{ApplyStatus, SiteProtocol, SiteRow, TargetBinding, TargetKind};
use crate::error::{AppError, AppResult};
use crate::paths::{zcode_provider_config_path, zcode_provider_path, resolve_zcode_home};
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// 托管 provider 的前缀。与 Pi/Prime 的 `xiaobai_` provider、MCP 条目同一套约定。
pub const PROVIDER_PREFIX: &str = "xiaobai_";

/// `provider_config.json` 里我们写入的 group（ZCode 自己的分类字段）。
const PROVIDER_GROUP: &str = "standard-personal";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZCodeApiType {
    AnthropicMessages,
    OpenAiResponses,
    OpenAiChatCompletions,
}

impl ZCodeApiType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::AnthropicMessages => "anthropic-messages",
            Self::OpenAiResponses => "openai-responses",
            Self::OpenAiChatCompletions => "openai-chat-completions",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "anthropic-messages" => Some(Self::AnthropicMessages),
            "openai-responses" => Some(Self::OpenAiResponses),
            "openai-chat-completions" => Some(Self::OpenAiChatCompletions),
            _ => None,
        }
    }

    /// `config.json` 侧配套的 `kind`。两处必须一致，否则 ZCode 行为不可预期。
    pub fn kind(&self) -> &'static str {
        match self {
            Self::AnthropicMessages => "anthropic",
            Self::OpenAiResponses | Self::OpenAiChatCompletions => "openai-compatible",
        }
    }

    /// 站点未单独指定时的默认：按站点协议推断。
    pub fn default_for(protocol: SiteProtocol) -> Self {
        match protocol {
            SiteProtocol::Anthropic => Self::AnthropicMessages,
            SiteProtocol::OpenaiCompatible => Self::OpenAiResponses,
        }
    }
}

/// 站点在 ZCode 里的 provider id。派生自站点 id，保证重复应用得到同一个 id（不成倍增长）。
pub fn provider_id(site_id: &str) -> String {
    let compact: String = site_id.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    format!("{PROVIDER_PREFIX}{}", &compact[..compact.len().min(16)])
}

/// 该站点应使用的 api 类型：显式设置优先，否则按协议推断。
pub fn api_type_for(site: &SiteRow) -> ZCodeApiType {
    site.zcode_api_type
        .as_deref()
        .and_then(ZCodeApiType::parse)
        .unwrap_or_else(|| ZCodeApiType::default_for(site.protocol))
}

/// 构造 ZCode 的绑定记录。ZCode 没有独立的 auth 文件（key 就写在两处 provider 配置里），
/// 所以撤销/状态检测只能靠 binding 里的 provider id 与这两条路径。
pub fn build_binding(
    site: &SiteRow,
    model_id: &str,
    api_key: &str,
    write_all_models: bool,
    override_path: Option<&str>,
) -> AppResult<TargetBinding> {
    let (provider_path, rules_path) = provider_paths(override_path)?;
    Ok(TargetBinding {
        target: TargetKind::ZCode,
        site_id: Some(site.id.clone()),
        site_name_snapshot: site.name.clone(),
        model_id: model_id.to_string(),
        provider_id: Some(provider_id(&site.id)),
        key_fingerprint: key_fingerprint(api_key),
        managed_paths: vec![
            provider_path.display().to_string(),
            rules_path.display().to_string(),
        ],
        managed_env_keys: Vec::new(),
        expected_fields: HashMap::from([
            ("base_url".to_string(), site.base_url.clone()),
            ("api".to_string(), api_type_for(site).as_str().to_string()),
            ("write_all_models".to_string(), write_all_models.to_string()),
        ]),
        orphan: false,
        applied_at: 0,
        apply_record_id: None,
        api_key: Default::default(),
    })
}

fn is_managed(provider_id: &str) -> bool {
    provider_id.starts_with(PROVIDER_PREFIX)
}

fn read_json_object(path: &Path, label: &str) -> AppResult<Map<String, Value>> {
    if !path.exists() {
        return Ok(Map::new());
    }
    let text = fs::read_to_string(path)?;
    if text.trim().is_empty() {
        return Ok(Map::new());
    }
    let value: Value = serde_json::from_str(&text)
        .map_err(|error| AppError::new("invalid_config", format!("{label} 不是合法 JSON: {error}")))?;
    value.as_object().cloned().ok_or_else(|| {
        AppError::new("invalid_config", format!("{label} 的根不是对象"))
    })
}

/// `config.json` 里的 `provider` 表（形状不对就报错，不覆盖）。
fn read_provider_table(path: &Path) -> AppResult<Map<String, Value>> {
    let root = read_json_object(path, "ZCode config.json")?;
    match root.get("provider") {
        None => Ok(Map::new()),
        Some(Value::Object(map)) => Ok(map.clone()),
        Some(_) => Err(AppError::new(
            "invalid_config",
            "ZCode config.json 的 provider 不是对象",
        )),
    }
}

/// `provider_config.json` 里的 `config` 对象。
fn read_provider_config(path: &Path) -> AppResult<Map<String, Value>> {
    let root = read_json_object(path, "ZCode provider_config.json")?;
    match root.get("config") {
        None => Ok(Map::new()),
        Some(Value::Object(map)) => Ok(map.clone()),
        Some(_) => Err(AppError::new(
            "invalid_config",
            "ZCode provider_config.json 的 config 不是对象",
        )),
    }
}

fn array_of_objects<'a>(parent: &'a Map<String, Value>, key: &str) -> AppResult<Vec<Value>> {
    match parent.get(key) {
        None => Ok(Vec::new()),
        Some(Value::Array(items)) => {
            if items.iter().all(Value::is_object) {
                Ok(items.clone())
            } else {
                Err(AppError::new(
                    "invalid_config",
                    format!("ZCode provider_config 的 {key} 含非对象项"),
                ))
            }
        }
        Some(_) => Err(AppError::new(
            "invalid_config",
            format!("ZCode provider_config 的 {key} 不是数组"),
        )),
    }
}

/// 模型条目的形状：limit/modalities 的具体值我们拿不到，给一个 ZCode 能接受的最小结构。
fn model_entry() -> Value {
    json!({
        "limit": { "context": 200000, "output": 64000 },
        "modalities": { "input": ["text"], "output": ["text"] },
        "zcode": { "modalitiesConfigured": true }
    })
}

/// 构造 `config.json` 的 provider 定义。
pub fn build_provider_entry(
    name: &str,
    base_url: &str,
    api_key: &str,
    api_type: ZCodeApiType,
    model_ids: &[String],
) -> Value {
    let models: Map<String, Value> = model_ids
        .iter()
        .map(|id| (id.clone(), model_entry()))
        .collect();
    json!({
        "name": name,
        "kind": api_type.kind(),
        "options": {
            "apiKey": api_key,
            "baseURL": base_url,
            "apiKeyRequired": true,
        },
        "source": "custom",
        "models": Value::Object(models),
    })
}

/// 构造 `provider_config.json` 的一条 providerRule。
pub fn build_provider_rule(
    id: &str,
    name: &str,
    base_url: &str,
    api_key: &str,
    api_type: ZCodeApiType,
    model_ids: &[String],
) -> Value {
    json!({
        "providerId": id,
        "providerName": name,
        "config": {
            "group": PROVIDER_GROUP,
            "access": { "type": "api-key", "apiKey": api_key },
            "api": { "type": api_type.as_str(), "baseUrl": base_url },
            "personalModelIds": model_ids,
            "modelOrder": model_ids,
        },
    })
}

/// 把一个托管 provider 合并进 `config.json` 的 provider 表。
pub fn merge_provider(root: &mut Map<String, Value>, id: &str, entry: Value) {
    root.insert(id.to_string(), entry);
}

/// 把托管 provider 从 provider 表里移除。
pub fn prune_managed_providers(root: &mut Map<String, Value>) {
    root.retain(|key, _| !is_managed(key));
}

/// 合并 `provider_config.json` 的 config：替换托管 rule、清理托管模型规则、维护顺序。
///
/// `provider_order` 里非托管项（含 `new-provider` 这类占位）原样保留且保持相对顺序，
/// 托管项统一放到末尾。
pub fn merge_provider_config(
    config: &mut Map<String, Value>,
    id: &str,
    rule: Value,
    model_ids: &[String],
) -> AppResult<()> {
    // providerConfigRules.providerRules
    let mut rules_parent = config
        .get("providerConfigRules")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let mut rules = array_of_objects(&rules_parent, "providerRules")?;
    rules.retain(|item| {
        item.get("providerId")
            .and_then(Value::as_str)
            .is_none_or(|pid| pid != id)
    });
    rules.push(rule);
    rules_parent.insert("providerRules".into(), Value::Array(rules));
    config.insert(
        "providerConfigRules".into(),
        Value::Object(rules_parent),
    );

    // modelConfigRules.providerModelRules
    let mut mcr = config
        .get("modelConfigRules")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let mut model_rules = array_of_objects(&mcr, "providerModelRules")?;
    model_rules.retain(|item| {
        item.get("providerId")
            .and_then(Value::as_str)
            .is_none_or(|pid| pid != id)
    });
    for model_id in model_ids {
        model_rules.push(json!({
            "modelId": model_id,
            "providerId": id,
            "config": { "properties": { "contextWindow": 200000 } },
        }));
    }
    mcr.insert("providerModelRules".into(), Value::Array(model_rules));
    config.insert("modelConfigRules".into(), Value::Object(mcr));

    // providerOrder：保留非托管项顺序，托管项置尾（去重）。
    let mut order: Vec<String> = match config.get("providerOrder") {
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect(),
        _ => Vec::new(),
    };
    order.retain(|pid| pid != id && !is_managed(pid));
    order.push(id.to_string());
    config.insert(
        "providerOrder".into(),
        Value::Array(order.into_iter().map(Value::String).collect()),
    );
    Ok(())
}

/// 移除托管 provider 在两处文件里的痕迹（`provider_config.json` 那侧）。
pub fn prune_provider_config(config: &mut Map<String, Value>) -> AppResult<()> {
    if let Some(parent) = config
        .get("providerConfigRules")
        .and_then(Value::as_object)
        .cloned()
    {
        let mut parent = parent;
        let mut rules = array_of_objects(&parent, "providerRules")?;
        rules.retain(|item| {
            item.get("providerId")
                .and_then(Value::as_str)
                .is_none_or(|pid| !is_managed(pid))
        });
        parent.insert("providerRules".into(), Value::Array(rules));
        config.insert("providerConfigRules".into(), Value::Object(parent));
    }
    if let Some(parent) = config
        .get("modelConfigRules")
        .and_then(Value::as_object)
        .cloned()
    {
        let mut parent = parent;
        let mut rules = array_of_objects(&parent, "providerModelRules")?;
        rules.retain(|item| {
            item.get("providerId")
                .and_then(Value::as_str)
                .is_none_or(|pid| !is_managed(pid))
        });
        parent.insert("providerModelRules".into(), Value::Array(rules));
        config.insert("modelConfigRules".into(), Value::Object(parent));
    }
    if let Some(Value::Array(items)) = config.get("providerOrder") {
        let kept: Vec<Value> = items
            .iter()
            .filter(|item| {
                item.as_str()
                    .is_none_or(|pid| !is_managed(pid))
            })
            .cloned()
            .collect();
        config.insert("providerOrder".into(), Value::Array(kept));
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct ZCodeApplyOutcome {
    pub backup_paths: Vec<String>,
    pub message: String,
}

/// 站点应用到 ZCode：两处文件都写，写入前各自备份。
pub fn apply(
    site: &SiteRow,
    api_key: &str,
    model_ids: &[String],
    override_path: Option<&str>,
    backup_root: &Path,
) -> AppResult<ZCodeApplyOutcome> {
    let provider_path = zcode_provider_path(override_path)?;
    let rules_path = zcode_provider_config_path(override_path)?;
    if let Some(parent) = provider_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // 两个文件分别上锁：ZCode 自己也会写它们。
    let _provider_lock = FileLock::acquire(&provider_path)?;
    let _rules_lock = FileLock::acquire(&rules_path)?;

    let mut backup_paths = Vec::new();
    for path in [&provider_path, &rules_path] {
        if path.exists() {
            backup_paths.push(backup_file(path, backup_root)?.display().to_string());
        }
    }

    let id = provider_id(&site.id);
    let api_type = api_type_for(site);
    let name = site.name.clone();
    let base_url = site.base_url.clone();

    // config.json
    let mut provider_table = read_provider_table(&provider_path)?;
    prune_managed_providers(&mut provider_table);
    merge_provider(
        &mut provider_table,
        &id,
        build_provider_entry(&name, &base_url, api_key, api_type, model_ids),
    );
    let mut provider_root = read_json_object(&provider_path, "ZCode config.json")?;
    provider_root.insert("provider".into(), Value::Object(provider_table));
    atomic_write(
        &provider_path,
        (serde_json::to_string_pretty(&Value::Object(provider_root))? + "\n").as_bytes(),
        true,
    )?;

    // provider_config.json
    let mut config = read_provider_config(&rules_path)?;
    merge_provider_config(
        &mut config,
        &id,
        build_provider_rule(&id, &name, &base_url, api_key, api_type, model_ids),
        model_ids,
    )?;
    let mut rules_root = read_json_object(&rules_path, "ZCode provider_config.json")?;
    if !rules_root.contains_key("schemaVersion") {
        rules_root.insert("schemaVersion".into(), json!(1));
    }
    rules_root.insert("config".into(), Value::Object(config));
    atomic_write(
        &rules_path,
        (serde_json::to_string_pretty(&Value::Object(rules_root))? + "\n").as_bytes(),
        true,
    )?;

    Ok(ZCodeApplyOutcome {
        backup_paths,
        message: format!("Applied '{}' to ZCode (restart ZCode to pick it up)", name),
    })
}

/// 撤销：移除托管 provider 在两处的痕迹。
pub fn surgical_revert(binding: &TargetBinding, override_path: Option<&str>) -> AppResult<()> {
    let id = binding
        .provider_id
        .clone()
        .unwrap_or_else(|| provider_id(binding.site_id.as_deref().unwrap_or_default()));
    remove_provider(&id, override_path)
}

/// 恢复官方：与撤销同义——把我们的 provider 从两处清掉，其余原样。
pub fn restore_official(
    binding: Option<&TargetBinding>,
    override_path: Option<&str>,
    backup_root: &Path,
) -> AppResult<crate::adapters::RestoreOfficialOutcome> {
    let provider_path = zcode_provider_path(override_path)?;
    let rules_path = zcode_provider_config_path(override_path)?;
    let mut backup_paths = Vec::new();
    if provider_path.exists() {
        backup_paths.push(backup_file(&provider_path, backup_root)?.display().to_string());
    }
    if rules_path.exists() {
        backup_paths.push(backup_file(&rules_path, backup_root)?.display().to_string());
    }
    let _ = binding;
    remove_all_managed(override_path)?;
    Ok(crate::adapters::RestoreOfficialOutcome {
        backup_paths,
        env_keys: Vec::new(),
    })
}

fn remove_provider(id: &str, override_path: Option<&str>) -> AppResult<()> {
    let provider_path = zcode_provider_path(override_path)?;
    let rules_path = zcode_provider_config_path(override_path)?;
    let _provider_lock = FileLock::acquire(&provider_path)?;
    let _rules_lock = FileLock::acquire(&rules_path)?;

    if provider_path.exists() {
        let mut table = read_provider_table(&provider_path)?;
        table.retain(|key, _| key != id);
        let mut root = read_json_object(&provider_path, "ZCode config.json")?;
        root.insert("provider".into(), Value::Object(table));
        atomic_write(
            &provider_path,
            (serde_json::to_string_pretty(&Value::Object(root))? + "\n").as_bytes(),
            true,
        )?;
    }
    if rules_path.exists() {
        let mut config = read_provider_config(&rules_path)?;
        if let Some(parent) = config
            .get("providerConfigRules")
            .and_then(Value::as_object)
            .cloned()
        {
            let mut parent = parent;
            let mut rules = array_of_objects(&parent, "providerRules")?;
            rules.retain(|item| {
                item.get("providerId").and_then(Value::as_str).is_none_or(|pid| pid != id)
            });
            parent.insert("providerRules".into(), Value::Array(rules));
            config.insert("providerConfigRules".into(), Value::Object(parent));
        }
        if let Some(parent) = config
            .get("modelConfigRules")
            .and_then(Value::as_object)
            .cloned()
        {
            let mut parent = parent;
            let mut rules = array_of_objects(&parent, "providerModelRules")?;
            rules.retain(|item| {
                item.get("providerId").and_then(Value::as_str).is_none_or(|pid| pid != id)
            });
            parent.insert("providerModelRules".into(), Value::Array(rules));
            config.insert("modelConfigRules".into(), Value::Object(parent));
        }
        if let Some(Value::Array(items)) = config.get("providerOrder") {
            let kept: Vec<Value> = items
                .iter()
                .filter(|item| item.as_str().is_none_or(|pid| pid != id))
                .cloned()
                .collect();
            config.insert("providerOrder".into(), Value::Array(kept));
        }
        let mut root = read_json_object(&rules_path, "ZCode provider_config.json")?;
        root.insert("config".into(), Value::Object(config));
        atomic_write(
            &rules_path,
            (serde_json::to_string_pretty(&Value::Object(root))? + "\n").as_bytes(),
            true,
        )?;
    }
    Ok(())
}

fn remove_all_managed(override_path: Option<&str>) -> AppResult<()> {
    let provider_path = zcode_provider_path(override_path)?;
    let rules_path = zcode_provider_config_path(override_path)?;
    if provider_path.exists() {
        let mut table = read_provider_table(&provider_path)?;
        prune_managed_providers(&mut table);
        let mut root = read_json_object(&provider_path, "ZCode config.json")?;
        root.insert("provider".into(), Value::Object(table));
        atomic_write(
            &provider_path,
            (serde_json::to_string_pretty(&Value::Object(root))? + "\n").as_bytes(),
            true,
        )?;
    }
    if rules_path.exists() {
        let mut config = read_provider_config(&rules_path)?;
        prune_provider_config(&mut config)?;
        let mut root = read_json_object(&rules_path, "ZCode provider_config.json")?;
        root.insert("config".into(), Value::Object(config));
        atomic_write(
            &rules_path,
            (serde_json::to_string_pretty(&Value::Object(root))? + "\n").as_bytes(),
            true,
        )?;
    }
    Ok(())
}

/// 是否已应用：两处都能找到我们的 provider id 才算 Applied。
pub fn detect_status(
    binding: &TargetBinding,
    override_path: Option<&str>,
) -> AppResult<ApplyStatus> {
    let id = binding
        .provider_id
        .clone()
        .unwrap_or_else(|| provider_id(binding.site_id.as_deref().unwrap_or_default()));
    let provider_path = zcode_provider_path(override_path)?;
    let rules_path = zcode_provider_config_path(override_path)?;

    let in_provider = read_provider_table(&provider_path)
        .map(|table| table.contains_key(&id))
        .unwrap_or(false);
    let in_rules = read_provider_config(&rules_path)
        .map(|config| {
            config
                .get("providerConfigRules")
                .and_then(Value::as_object)
                .and_then(|parent| parent.get("providerRules"))
                .and_then(Value::as_array)
                .is_some_and(|rules| {
                    rules.iter().any(|item| {
                        item.get("providerId").and_then(Value::as_str) == Some(id.as_str())
                    })
                })
        })
        .unwrap_or(false);

    Ok(if in_provider && in_rules {
        ApplyStatus::Applied
    } else if in_provider || in_rules {
        // 只写了一处：多半是上次写入中断，重新应用即可收敛。
        ApplyStatus::Stale
    } else {
        ApplyStatus::NotApplied
    })
}

/// 当前 ZCode 里各 provider 的摘要（provider id → 名称）。
pub fn live_summary(override_path: Option<&str>) -> AppResult<HashMap<String, Option<String>>> {
    let mut summary = HashMap::new();
    let provider_path = zcode_provider_path(override_path)?;
    for (id, entry) in read_provider_table(&provider_path)? {
        let name = entry
            .get("name")
            .and_then(Value::as_str)
            .map(str::to_string);
        summary.insert(id, name);
    }
    Ok(summary)
}

/// 清理游离配置：没有绑定记录时把两处文件里的 `xiaobai_` provider 全部摘掉。
pub fn cleanup_orphans(override_path: Option<&str>) -> AppResult<()> {
    remove_all_managed(override_path)
}

pub fn backup_summary(dir: &Path) -> HashMap<String, Option<String>> {
    let mut summary = HashMap::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return summary;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            summary.insert(
                path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or_default()
                    .to_string(),
                None,
            );
        }
    }
    summary
}

/// 路由切换：把托管 provider 的 baseURL 改成新地址（两处都改）。
pub fn rewrite_base_url(
    binding: &TargetBinding,
    base_url: &str,
    override_path: Option<&str>,
    backup_root: &Path,
) -> AppResult<()> {
    let id = binding
        .provider_id
        .clone()
        .unwrap_or_else(|| provider_id(binding.site_id.as_deref().unwrap_or_default()));
    let provider_path = zcode_provider_path(override_path)?;
    let rules_path = zcode_provider_config_path(override_path)?;
    let _provider_lock = FileLock::acquire(&provider_path)?;
    let _rules_lock = FileLock::acquire(&rules_path)?;

    if provider_path.exists() {
        backup_file(&provider_path, backup_root)?;
        let mut table = read_provider_table(&provider_path)?;
        if let Some(entry) = table.get_mut(&id).and_then(Value::as_object_mut) {
            if let Some(options) = entry.get_mut("options").and_then(Value::as_object_mut) {
                options.insert("baseURL".into(), Value::String(base_url.to_string()));
            }
            let mut root = read_json_object(&provider_path, "ZCode config.json")?;
            root.insert("provider".into(), Value::Object(table.clone()));
            atomic_write(
                &provider_path,
                (serde_json::to_string_pretty(&Value::Object(root))? + "\n").as_bytes(),
                true,
            )?;
        }
    }
    if rules_path.exists() {
        backup_file(&rules_path, backup_root)?;
        let mut config = read_provider_config(&rules_path)?;
        if let Some(parent) = config
            .get("providerConfigRules")
            .and_then(Value::as_object)
            .cloned()
        {
            let mut parent = parent;
            let mut rules = array_of_objects(&parent, "providerRules")?;
            for item in rules.iter_mut() {
                if item.get("providerId").and_then(Value::as_str) != Some(id.as_str()) {
                    continue;
                }
                if let Some(api) = item
                    .get_mut("config")
                    .and_then(Value::as_object_mut)
                    .and_then(|c| c.get_mut("api"))
                    .and_then(Value::as_object_mut)
                {
                    api.insert("baseUrl".into(), Value::String(base_url.to_string()));
                }
            }
            parent.insert("providerRules".into(), Value::Array(rules));
            config.insert("providerConfigRules".into(), Value::Object(parent));
            let mut root = read_json_object(&rules_path, "ZCode provider_config.json")?;
            root.insert("config".into(), Value::Object(config));
            atomic_write(
                &rules_path,
                (serde_json::to_string_pretty(&Value::Object(root))? + "\n").as_bytes(),
                true,
            )?;
        }
    }
    Ok(())
}

/// 供 dock/状态展示：ZCode 配置目录是否存在。
pub fn home_exists(override_path: Option<&str>) -> bool {
    resolve_zcode_home(override_path)
        .map(|home| home.exists())
        .unwrap_or(false)
}

pub fn provider_paths(override_path: Option<&str>) -> AppResult<(PathBuf, PathBuf)> {
    Ok((
        zcode_provider_path(override_path)?,
        zcode_provider_config_path(override_path)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{ClaudeAuthKeyStyle, SiteKeyState, TargetKind};

    fn site(id: &str, protocol: SiteProtocol, api_type: Option<&str>) -> SiteRow {
        SiteRow {
            id: id.to_string(),
            name: "Relay".into(),
            base_url: "https://api.example.com".into(),
            base_urls: vec![],
            api_key_encrypted: String::new(),
            key_prefix: "sk-…".into(),
            protocol,
            claude_auth_key_style: ClaudeAuthKeyStyle::AnthropicAuthToken,
            notes: None,
            enabled: true,
            sort_order: 0,
            selected_model_id: None,
            last_model_fetch_at: None,
            last_model_fetch_latency_ms: None,
            last_model_fetch_error: None,
            created_at: 0,
            updated_at: 0,
            capabilities: Default::default(),
            keys: SiteKeyState::default(),
            newapi_access_token_encrypted: None,
            newapi_user_id: None,
            proxy_headers_encrypted: None,
            proxy_header_count: 0,
            zcode_api_type: api_type.map(str::to_string),
        }
    }

    fn binding_for(site_id: &str) -> TargetBinding {
        TargetBinding {
            target: TargetKind::ZCode,
            site_id: Some(site_id.to_string()),
            site_name_snapshot: "Relay".into(),
            model_id: "m".into(),
            provider_id: Some(provider_id(site_id)),
            key_fingerprint: String::new(),
            managed_paths: vec![],
            managed_env_keys: vec![],
            expected_fields: HashMap::new(),
            orphan: false,
            applied_at: 0,
            apply_record_id: None,
            api_key: Default::default(),
        }
    }

    struct TempHome(PathBuf, tempfile::TempDir);

    impl TempHome {
        fn new() -> Self {
            let dir = tempfile::tempdir().unwrap();
            let home = dir.path().join("zcode");
            fs::create_dir_all(home.join("v2")).unwrap();
            Self(home, dir)
        }
        fn path(&self) -> &str {
            self.0.to_str().unwrap()
        }
    }

    #[test]
    fn provider_id_is_stable_and_namespaced() {
        assert_eq!(provider_id("3f8ac91c-0a03-4574-8314-0cf559c83228"), "xiaobai_3f8ac91c0a034574");
        // 同一站点反复应用必须得到同一个 id（否则会越写越多）。
        assert_eq!(
            provider_id("3f8ac91c-0a03-4574-8314-0cf559c83228"),
            provider_id("3f8ac91c0a034574831400cf559c8322")
        );
    }

    #[test]
    fn api_type_defaults_follow_protocol_but_explicit_wins() {
        assert_eq!(
            api_type_for(&site("a", SiteProtocol::Anthropic, None)),
            ZCodeApiType::AnthropicMessages
        );
        assert_eq!(
            api_type_for(&site("a", SiteProtocol::OpenaiCompatible, None)),
            ZCodeApiType::OpenAiResponses
        );
        // 调研发现用户的选择与站点协议不一致，所以显式设置必须优先。
        assert_eq!(
            api_type_for(&site("a", SiteProtocol::OpenaiCompatible, Some("anthropic-messages"))),
            ZCodeApiType::AnthropicMessages
        );
    }

    #[test]
    fn kind_pairs_with_api_type_on_both_sides() {
        assert_eq!(ZCodeApiType::AnthropicMessages.kind(), "anthropic");
        assert_eq!(ZCodeApiType::OpenAiResponses.kind(), "openai-compatible");
        assert_eq!(ZCodeApiType::OpenAiChatCompletions.kind(), "openai-compatible");
        assert_eq!(ZCodeApiType::OpenAiChatCompletions.as_str(), "openai-chat-completions");
    }

    #[test]
    fn apply_writes_both_files_and_preserves_foreign_entries() {
        let home = TempHome::new();
        let provider_path = PathBuf::from(home.path()).join("v2/config.json");
        let rules_path = PathBuf::from(home.path()).join("v2/provider_config.json");
        // 预置用户的 provider 与 providerOrder 占位项
        fs::write(
            &provider_path,
            serde_json::to_string_pretty(&json!({
                "provider": { "user-own": { "name": "mine", "kind": "anthropic" } }
            }))
            .unwrap(),
        )
        .unwrap();
        fs::write(
            &rules_path,
            serde_json::to_string_pretty(&json!({
                "schemaVersion": 1,
                "config": {
                    "providerOrder": ["user-own", "new-provider"],
                    "providerConfigRules": { "providerRules": [
                        { "providerId": "user-own", "providerName": "mine", "config": {} }
                    ]},
                    "modelConfigRules": { "providerModelRules": [
                        { "modelId": "m1", "providerId": "user-own", "config": {} }
                    ]}
                }
            }))
            .unwrap(),
        )
        .unwrap();

        let backup_root = home.0.join("backups");
        let server = site("site-1", SiteProtocol::Anthropic, Some("anthropic-messages"));
        let models = vec!["claude-sonnet-5".to_string()];
        apply(&server, "sk-secret", &models, Some(home.path()), &backup_root).unwrap();

        let provider: Value =
            serde_json::from_str(&fs::read_to_string(&provider_path).unwrap()).unwrap();
        let id = provider_id("site-1");
        assert_eq!(provider["provider"][&id]["kind"], "anthropic");
        assert_eq!(provider["provider"][&id]["options"]["baseURL"], "https://api.example.com");
        assert_eq!(provider["provider"][&id]["options"]["apiKey"], "sk-secret");
        assert_eq!(provider["provider"]["user-own"]["name"], "mine");
        assert!(provider["provider"][&id]["models"]["claude-sonnet-5"].is_object());

        let rules: Value =
            serde_json::from_str(&fs::read_to_string(&rules_path).unwrap()).unwrap();
        let order = rules["config"]["providerOrder"].as_array().unwrap();
        // 用户项与占位项保留，托管项追加在末尾
        assert_eq!(order[0], "user-own");
        assert_eq!(order[1], "new-provider");
        assert_eq!(order.last().unwrap(), &Value::String(id.clone()));
        let api = rules["config"]["providerConfigRules"]["providerRules"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["providerId"] == id.as_str())
            .unwrap();
        assert_eq!(api["config"]["api"]["type"], "anthropic-messages");
        assert_eq!(api["config"]["api"]["baseUrl"], "https://api.example.com");
    }

    #[test]
    fn reapply_replaces_instead_of_duplicating() {
        let home = TempHome::new();
        let server = site("site-1", SiteProtocol::Anthropic, None);
        let backup_root = home.0.join("backups");
        apply(&server, "k1", &["m1".to_string()], Some(home.path()), &backup_root).unwrap();
        apply(&server, "k2", &["m2".to_string()], Some(home.path()), &backup_root).unwrap();

        let rules_path = PathBuf::from(home.path()).join("v2/provider_config.json");
        let rules: Value = serde_json::from_str(&fs::read_to_string(&rules_path).unwrap()).unwrap();
        let id = provider_id("site-1");
        let matching = rules["config"]["providerConfigRules"]["providerRules"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["providerId"] == id.as_str())
            .count();
        assert_eq!(matching, 1, "重复应用不能产生多条 rule");
        let order = rules["config"]["providerOrder"].as_array().unwrap();
        assert_eq!(order.iter().filter(|v| **v == Value::String(id.clone())).count(), 1);

        let provider_path = PathBuf::from(home.path()).join("v2/config.json");
        let provider: Value =
            serde_json::from_str(&fs::read_to_string(&provider_path).unwrap()).unwrap();
        assert_eq!(provider["provider"][&id]["options"]["apiKey"], "k2", "应更新为新 key");
    }

    #[test]
    fn revert_removes_only_managed_provider() {
        let home = TempHome::new();
        let provider_path = PathBuf::from(home.path()).join("v2/config.json");
        fs::write(
            &provider_path,
            serde_json::to_string_pretty(&json!({
                "provider": { "user-own": { "name": "mine" } }
            }))
            .unwrap(),
        )
        .unwrap();
        let backup_root = home.0.join("backups");
        let server = site("site-1", SiteProtocol::Anthropic, None);
        apply(&server, "k", &["m1".to_string()], Some(home.path()), &backup_root).unwrap();

        let binding = binding_for("site-1");
        assert_eq!(detect_status(&binding, Some(home.path())).unwrap(), ApplyStatus::Applied);
        surgical_revert(&binding, Some(home.path())).unwrap();
        assert_eq!(detect_status(&binding, Some(home.path())).unwrap(), ApplyStatus::NotApplied);

        let provider: Value =
            serde_json::from_str(&fs::read_to_string(&provider_path).unwrap()).unwrap();
        assert!(provider["provider"]["user-own"].is_object(), "用户 provider 必须保留");
    }

    #[test]
    fn detect_status_flags_half_written_state_as_stale() {
        let home = TempHome::new();
        let server = site("site-1", SiteProtocol::Anthropic, None);
        let backup_root = home.0.join("backups");
        apply(&server, "k", &["m1".to_string()], Some(home.path()), &backup_root).unwrap();

        // 模拟只写了一处（上次中断）
        let rules_path = PathBuf::from(home.path()).join("v2/provider_config.json");
        let mut rules: Value = serde_json::from_str(&fs::read_to_string(&rules_path).unwrap()).unwrap();
        rules["config"]["providerConfigRules"]["providerRules"] = json!([]);
        fs::write(&rules_path, rules.to_string()).unwrap();

        let binding = binding_for("site-1");
        assert_eq!(detect_status(&binding, Some(home.path())).unwrap(), ApplyStatus::Stale);
    }

    #[test]
    fn malformed_shape_is_reported_and_file_kept() {
        let home = TempHome::new();
        let provider_path = PathBuf::from(home.path()).join("v2/config.json");
        let original = r#"{"provider": "oops"}"#;
        fs::write(&provider_path, original).unwrap();

        let server = site("site-1", SiteProtocol::Anthropic, None);
        let err = apply(&server, "k", &[], Some(home.path()), &home.0.join("b")).unwrap_err();
        assert!(err.to_string().contains("provider"), "{err}");
        assert_eq!(fs::read_to_string(&provider_path).unwrap(), original, "必须原样保留");
    }

    #[test]
    fn base_url_rewrite_updates_both_files() {
        let home = TempHome::new();
        let server = site("site-1", SiteProtocol::Anthropic, None);
        let backup_root = home.0.join("backups");
        apply(&server, "k", &["m1".to_string()], Some(home.path()), &backup_root).unwrap();

        let binding = binding_for("site-1");
        rewrite_base_url(&binding, "https://new.example.com", Some(home.path()), &backup_root)
            .unwrap();

        let provider: Value = serde_json::from_str(
            &fs::read_to_string(PathBuf::from(home.path()).join("v2/config.json")).unwrap(),
        )
        .unwrap();
        let rules: Value = serde_json::from_str(
            &fs::read_to_string(PathBuf::from(home.path()).join("v2/provider_config.json")).unwrap(),
        )
        .unwrap();
        let id = provider_id("site-1");
        assert_eq!(provider["provider"][&id]["options"]["baseURL"], "https://new.example.com");
        let api = rules["config"]["providerConfigRules"]["providerRules"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["providerId"] == id.as_str())
            .unwrap();
        assert_eq!(api["config"]["api"]["baseUrl"], "https://new.example.com");
    }
}
