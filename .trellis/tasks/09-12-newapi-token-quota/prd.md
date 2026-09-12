# PRD: NewAPI 令牌额度与模型列表兜底

## 背景
- AgentRouter 等站点做客户端指纹检测：标准请求返回 401 "unauthorized client detected"，模型列表拉不到。实测伪装 Claude Code UA（claude-cli/x + x-app: cli）可放行。
- new-api 生态碎片化：现有额度探测（/dashboard/billing/* + /api/usage/token + /api/status）只对部分部署有效（shuaiapi 可用）。
- ccswitch 的 New API 模板方案：GET /api/user/self + Authorization: Bearer {访问令牌} + New-Api-User: {用户ID}，data.quota/500000 = 美元（账户级钱包余额）。

## 需求 1：模型列表兜底伪装
- fetch_models 两种协议在标准请求失败（非 2xx 或解析失败）时，自动追加 Claude Code 客户端头重试一次。
- 重试头：User-Agent: claude-cli/2.0.14 (external, cli)、x-app: cli；Anthropic 路径重试用 Bearer 认证（AgentRouter 实测 Bearer 才放行）。
- 对正常站点零影响；仍然先标准后兜底。

## 需求 2：NewAPI 访问令牌（账户余额）
- sites 表新增可选字段：newapi_access_token_encrypted（加密存储，同 master.key 体系）、newapi_user_id。
- 站点编辑（高级配置）新增：访问令牌（密码框，已保存显示占位）、用户 ID；随 create/update 持久化，随 WebDAV 同步走（在 DB 内自动满足）。
- 编辑时解密查看：get_site_newapi_token 命令（对齐 get_site_api_key 模式）。
- probe_quota 探测链末尾：标准链失败/不支持 且 配置了令牌 → GET {origin}/api/user/self（Bearer accessToken + New-Api-User: userId）→ remaining = data.quota/500000，used = data.used_quota/500000，total = 两者和，source = newapi_user_self。
- 额度显示：查询失败且未配置令牌 → 站点额度区显示可点击提示，引导到站点编辑高级配置。

## 验收
- cargo test：models_fetch 伪装兜底（mock 校验 UA）、user_self 解析、站点字段 roundtrip 全部通过。
- pnpm typecheck + test:run 通过；browserMock 同步新字段。
- 现有行为不回归。
