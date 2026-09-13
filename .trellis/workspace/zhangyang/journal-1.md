# Journal - zhangyang (Part 1)

> AI development session journal
> Started: 2026-09-12

---



## Session 1: 真同步落地与品牌独立化
<!-- trellis-session: v=2 fp=5e62dab72faaf944 -->

**Date**: 2026-09-12
**Task**: 真同步落地与品牌独立化
**Branch**: `main`

### Summary

WebDAV 真同步全链路落地：逻辑内容指纹引擎+变更驱动守护任务（启动/聚焦/写库防抖），双机模拟 E2E 全场景验证；修复文件 hash 指纹漂移导致的同步重启循环；仓库地址/更新端点/签名公钥/应用标识符全部切换至 fork（doMySelfZy）并生成自有更新签名密钥；界面全面改为同步语义（兜底间隔/云端数据包保留/云端版本状态条/上传后自动清理）；Trellis 从 trellis-cli 切换到 mindfold-ai/Trellis（ZCode 原生支持）。

### Git Commits

| Hash | Message |
|------|---------|
| `c46b143` | fix(sync): 逻辑内容指纹替代文件 hash 修复误判循环；配置后立即决策；支持数据目录隔离 |
| `45c49f5` | chore(brand): 仓库地址、更新端点、签名公钥与应用标识符切换至 fork |
| `7f85908` | feat(sync): 同步语义改造界面——配置弹窗与弹层文案、云端版本状态条、上传后按保留数清理 |
| `c5c0751` | chore(trellis): 切换到 mindfold-ai/Trellis（ZCode 原生支持），移除 trellis-cli 脚手架 |
| `8d74284` | chore(trellis): 清理 trellis-cli 遗留的旧文档结构 |

### Status

[OK] **Completed**


## Session 2: 品牌回退为 XiaoBaiSwitch Plus + 账户余额统一 + v0.1.2 发布
<!-- trellis-session: v=2 fp=33d1f43571fc8830 -->

**Date**: 2026-09-13
**Task**: 品牌回退为 XiaoBaiSwitch Plus + 账户余额统一 + v0.1.2 发布
**Branch**: `main`

### Summary

1) 账户余额：测试按钮与主界面改为同源（共用候选 origin、金额换算、成功判定），new-api 的无限额度哨兵值不再当美元显示，额度行只留剩余金额并统一叫「账户余额」。2) 品牌回退：撤销 AnySwitch 改名，回到原作者命名体系——展示名 XiaoBaiSwitch Plus、identifier com.github.licoy.xiaobai-switch.plus、数据目录 ~/.xiaobai-switch（按数据库 mtime 新鲜度安全接管 ~/.any-switch，绝不删数据）、深链主 scheme xiaobaiswitchplus 且系统注册旧 scheme、图标恢复作者原图、删除官网 website/、弃用 Gitee。3) 仓库改名 doMySelfZy/xiaobai-switch-plus 并发布 v0.1.2（安装包 + .sig + latest.json，更新端点与签名公钥均为自有）。4) 事故与修复：安装时启用陈旧数据目录，导致同步守护把旧数据上传为云端最新版、之后每次启动回滚；已把活数据放回并把 last_synced 设为远端指纹，使同步决策翻转为上传，云端恢复为活数据（revision 25）。5) 验证：cargo test 351 passed、pnpm typecheck 通过、pnpm test:run 289 passed（2 个既有加载失败）。遗留：并行会话 09-13-mcp-unified-control 的未提交改动把 SCHEMA_VERSION 提到 2 却未在普通迁移路径补 ensure_sites_newapi_columns，仓库内该回归测试因此变红（其 WIP 未提交、未进安装包）。

### Git Commits

| Hash | Message |
|------|---------|
| `ea7935a` | chore(version): bump version to v0.1.1 |
| `6abb441` | fix(quota): 账户余额与「测试」按钮同源，只展示剩余金额 |
| `a92408a` | feat(brand)!: 回退为 XiaoBaiSwitch Plus（沿用原作者命名，更新只走自有 GitHub） |
| `11915c7` | docs: README 去掉已弃用的产品名，CI 回退名单清理 |

### Status

[OK] **Completed**


## Session 3: MCP 统一管控与跨设备同步
<!-- trellis-session: v=2 fp=636b5511401b1469 -->

**Date**: 2026-09-13
**Task**: MCP 统一管控与跨设备同步
**Branch**: `main`

### Summary

为 Claude Code/Codex/Pi/Prime 增加统一 MCP 管控：一份定义可勾选应用，按各客户端原生位置写入（Claude 用 ~/.claude.json，Codex 用 config.toml 的 mcp_servers，Pi 用 mcp.json，Prime 用 settings.json）。托管条目限定 xiaobai_ 前缀，改名/禁用/删除后清理孤儿；写入前备份+原子替换+.lock 互斥。env/headers 加密存储，mcp_servers 纳入同步内容指纹。schema 升级到 v2 并修复 apply_schema 增量补齐漏分支的老库问题。cargo test 399 绿、pnpm typecheck 绿、前端 310 绿（2 个 updater 用例为既有失败）。

### Git Commits

| Hash | Message |
|------|---------|
| `f5ef700` | feat(mcp): 统一管控四个 Agent 的 MCP 配置并纳入跨设备同步 |
| `6eb9e80` | test(mcp): 覆盖改名/删除后跨四个客户端的托管条目清理 |

### Status

[OK] **Completed**


## Session 4: 站点列表额度摘要与余额自动刷新
<!-- trellis-session: v=2 fp=916822fdcfa1c7b5 -->

**Date**: 2026-09-13
**Task**: 站点列表额度摘要与余额自动刷新
**Branch**: `main`

### Summary

站点列表条目显示额度摘要：余额型显示剩余金额（经站点自报倍率换算），窗口型显示 rolling 窗口用量百分比（≥80% 橙 / ≥90% 红），不支持的站点安静不显示；悬停 Tooltip 展示各窗口用量与更新时间。按用户反馈调整：去掉列表 URL、第二行整行显示余额并固定 min-h-5 保持行高整齐，新增每 2 分钟自动刷新（仅页面可见时轮询，重新可见立即补刷），复用 probeQuota 的 in-flight 去重与 TTL。完成每日打包安装（工作树隔离构建，避免并行 MCP 会话的未提交改动进入安装包），UIA 实测真实站点余额与接口一致，日志确认自动刷新按周期触发；pnpm typecheck 绿、310 测试绿。附带排查结论：AgentRouter 是 new-api 而非 Sub2API（/api/status 自报版本与 quota_per_unit、Sub2API 的 /v1/usage 返回 404、/api/user/self 正常）；发现账户余额链路硬编码 500000 除数的隐患。

### Git Commits

| Hash | Message |
|------|---------|
| `eb236c6` | feat(sites): 列表第二行显示余额并自动刷新 |

### Status

[OK] **Completed**


## Session 5: 额度探测读站点自报倍率 + Sub2API /v1/usage 支持
<!-- trellis-session: v=2 fp=3b61b80c6df1d54a -->

**Date**: 2026-09-13
**Task**: 额度探测读站点自报倍率 + Sub2API /v1/usage 支持
**Branch**: `main`

### Summary

补齐两处额度探测缺口。①账户余额链路（/api/user/self）原先硬编码 quota_per_unit=500000 与单位 USD，改为与余额请求并发取一次站点自报的 /api/status 换算参数（quota_per_unit 作除数、quota_display_type 决定 CNY/USD/Custom），失败或缺失回退 500000/USD；乘数（token 链路）与除数（账户余额）语义严格分离并各加锚点测试。顺带修掉站点编辑「测试」按钮硬编码 $ 的符号分叉（NewApiAccessProbe 增 unit 字段，CNY 站点不再出现列表 ¥ / 测试 $）。②新增 Sub2API 风格站点的 /v1/usage（API Key Bearer）探测：实测 AiHub 的 billing 端点返回 HTTP 200 但是 65KB HTML 兜底页（被 looks_like_html 判为不支持），而 /v1/usage 返回钱包余额；探测只在标准链最终 Unsupported 后尝试，new-api 站点零额外请求；used/total 一律留空（钱包模式无此语义，避免前端算出误导进度条）；404/HTML/非法 JSON/isValid:false 全部安静降级为「不支持」。验证：cargo test 394 绿（复核子代理做变异验证证明新断言非空转）、pnpm typecheck 绿、前端 310 绿（2 个既有 updater 用例失败）；真机安装后 UIA 读取确认 AiHub 从「不支持」变为「剩余 $31.32」，SHUAI ￥27.57（CNY 正确）、JustWoker/AgentRouter 正常、OpenCode 三窗口 0%/24%/53%。本轮同时核验并归档两个已完成任务：09-13-opencode-go-quota（OpenCode 三窗口真机验证通过）、09-12-newapi-token-quota（令牌余额与模型兜底已上线）。打包踩坑：只设 TAURI_SIGNING_PRIVATE_KEY 而缺密码变量会永久挂起等 stdin，需同时设空密码或事后单独 signer sign。

### Git Commits

| Hash | Message |
|------|---------|
| `9218d45` | feat(quota): 账户余额读站点自报倍率，新增 Sub2API /v1/usage 探测 |

### Status

[OK] **Completed**


## Session 6: 站点列表侧栏加宽 + v0.1.4 打包安装
<!-- trellis-session: v=2 fp=bae04b0287028145 -->

**Date**: 2026-09-13
**Task**: 站点列表侧栏加宽 + v0.1.4 打包安装
**Branch**: `main`

### Summary

用户反馈列表里 OpenCode 只能看到一个数值、三窗口显示不全。核实为两个独立问题叠加：①用户安装的 0.1.3 不含「列表展示全部用量窗口」（提交 f6882d4/7cce472 在 v0.1.3 发版之后，标签指向 bump 提交 25ba00d）；②侧栏 256px 时列表项文字区仅 137px，而三窗口都接近满额需 152px，必然截断。改动：SitesPage 两处侧栏 w-64 → w-72（288px，实测可用 169px，余量 17px），骨架与实际布局同步。浏览器实测最坏情况三个 100% 全部完整（clipped=false），1100px 与 900px 视口下详情面板均无横向溢出；pnpm typecheck 绿、314 测试绿（2 个既有 updater 用例失败）。升版本 0.1.4 并打包安装，真机 UIA 确认列表显示「5h 100% 周 76% 月 47%」三窗口完整，其余站点余额正常。打包踩坑复现：CARGO_TARGET_DIR 指向主仓库但签名密钥用了相对路径 → 报 Invalid symbol 46，需用绝对路径单独 signer sign 补签。

### Git Commits

| Hash | Message |
|------|---------|
| `b0a45fc` | feat(sites): 站点列表侧栏加宽至 w-72，三窗口额度摘要不再截断 |
| `f593b1e` | chore(version): bump version to v0.1.4 |

### Status

[OK] **Completed**


## Session 7: 额度口径统一为剩余 + 有进度条即显示已用
<!-- trellis-session: v=2 fp=64c20ad026de2ecb -->

**Date**: 2026-09-13
**Task**: 额度口径统一为剩余 + 有进度条即显示已用
**Branch**: `main`

### Summary

用户反馈 OpenCode 列表只显示 5 小时一个窗口、且显示的是「已用」而余额型显示「剩余」，同一列表两种相反语义。三项改动：①列表展示全部三个窗口（短标签 5h/周/月，Tooltip 用完整标签）；②统一为「剩余」口径，窗口数值与进度条都改为 100−已用（详情面板同步）；③告警阈值收敛到唯一的 quotaRemainingTone（剩余 ≤20% 橙、≤10% 红，等价于已用 ≥80%/≥90%，与改造前时机一致），余额站点也走这套，删除已被取代的 primaryQuotaWindow/quotaUsageTone。随后用户追问 new-api 进度条的分母来源，查清两条链路：/api/user/self 只返回 quota + used_quota，总额是本地相加推算（AgentRouter 实测恒为 300,000,000 = $600）；/api/usage/token/ 的 display 对象由站点自报 total（SHUAI 报 200.318454 且与 remaining+used 自洽）。据此在推算来源补显「已用」。最后一轮用户质疑「SHUAI 拿不到已用？」——核实其 display 明确含 used=172.75，上一版按 source 排除是错的：规则改为「只要有进度条就显示已用」（不管总额来源），删除 isDerivedBalanceTotal，判定依据从「来源」改为「是否画条」；无总额的 Sub2API 钱包仍不显示（没有条时孤立已用金额是噪音）。验证：pnpm typecheck 绿、320 前端测试绿（2 个既有 updater 用例失败）；浏览器实测三场景（站点自报/本地推算/无总额）；真机 UIA 确认 SHUAI 显示「剩余 ￥27.57 已用 ￥172.75」、AiHub 仅剩余、OpenCode 三窗口剩余口径。三次打包安装（0.1.3→0.1.4），每次均从干净提交建临时 worktree 构建以隔离并行会话的未提交改动，并将签名密码变量一并设置（此前只设密钥路径会挂起等 stdin）。

### Git Commits

| Hash | Message |
|------|---------|
| `f6882d4` | feat(sites): 额度统一为「剩余」口径，列表展示全部用量窗口 |
| `7cce472` | feat(sites): 推算总额的站点补显「已用」金额，消除进度条歧义 |
| `e58bba3` | fix(sites): 有进度条就显示已用，不再只限推算总额的来源 |

### Status

[OK] **Completed**
