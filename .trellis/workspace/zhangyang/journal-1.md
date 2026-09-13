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
