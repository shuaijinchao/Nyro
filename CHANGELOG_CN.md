# 更新日志

Nyro 的所有重要变更均记录在此文件中。

---

## v1.2.0

> 发布于 2026-03-15

#### 功能

- **新增「接入」模块**：增加 `Connect` 页面与 `代码接入` / `CLI 接入` 双标签，支持按协议选择路由并生成 Python / TypeScript / cURL 示例
- **桌面端 CLI 集成**：支持 Claude Code、Codex CLI、Gemini CLI、OpenCode 的就绪检测与配置同步/恢复
- **CLI 配置预览与复制优化**：按文件展示将更新的片段，并在预览区域内置复制能力
- **API Key 权限模型升级**：受控路由改为默认拒绝（需显式绑定路由才可访问），并统一为 `sk-<32位hex>` 密钥格式
- **配额能力扩展**：新增 `RPD`（每天请求数），打通 API Key 数据模型、管理接口、前端表单与代理鉴权限流

#### 改进

- **API Key 页面重构**：创建/编辑表单按三段式重排（基本信息、访问权限、访问限额），统一宽度策略，编辑态下有效期与 Key 不可修改
- **Provider 表单优化**：API Key 输入支持显示/隐藏，修复编辑态 API Key 回显，并对齐创建/编辑布局行为
- **Route 表单一致性**：编辑布局与创建布局对齐，单行输入/下拉保持半宽展示
- **统计时间范围统一生效**：小时筛选覆盖概览、模型、提供商统计，并在 WebUI + 后端 + Tauri 命令链路保持一致

#### 修复

- 修复 `build-and-smoke` CI 对新鉴权流程的不兼容（移除过时 `--proxy-key`，改为创建并绑定 smoke API Key）
- 修复 CLI 同步参数命名不一致（`toolId` / `apiKey`），并增强前端错误信息解析
- 回退 Codex `wire_api` 为 `responses` 以兼容最新 CLI 行为
- 优化表单下拉/搜索面板视觉一致性与访问控制开关布局细节

#### CI 与发版

- 桌面端发版流程支持自动计算并回写 Homebrew Cask 校验和
- 更新路由/API Key 设计文档与安装说明，保持文档与实现一致

---

## v1.1.0

> 发布于 2026-03-13

#### 功能

- **路由匹配重构**：从模糊 `match_pattern` 切换为 `(ingress_protocol, virtual_model)` 精确匹配，支持 OpenAI / Anthropic / Gemini 接入
- **全新 API Key 体系**：新增 `api_keys` + `api_key_routes` 数据模型及完整 CRUD，默认密钥格式为 `sk-<32位hex>`
- **路由级访问控制**：先匹配路由，再在 `access_control` 开启时校验 API Key；支持按路由绑定或全局生效
- **API Key 配额能力**：在代理鉴权链路中新增 `RPM`、`TPM`、`TPD`、状态与过期时间校验

#### 改进

- **后端迁移与兼容处理**：
  - 新增并回填路由/Provider/日志字段（`ingress_protocol`、`virtual_model`、`access_control`、`channel`、`api_key_id`）
  - 现行流程移除旧的路由/Provider fallback 与 priority 机制
- **管理接口扩展**：服务端与 Tauri 管理 API/命令新增 API Key 管理能力
- **WebUI 路由与密钥体验升级**：
  - 新增 API Keys 页面，支持可搜索多选绑定路由
  - 创建路由时将提供商/模型同排展示，并自动将目标模型回填到虚拟模型
  - Provider 创建/编辑流程持久化并自动锚定供应商与渠道标识
- **UI 组件标准化**：引入并统一使用 shadcn 风格 `Badge`、`Switch`、`Checkbox`、`Dialog`、`Combobox`、`Command`、`Popover`、`MultiSelect`、`Tabs` 等组件
- **Provider 图标策略优化**：Provider 列表主图标优先展示供应商图标（亮色彩色、暗色纯色），协议胶囊图标保持协议维度
- **版本展示自动化**：设置页版本改为构建时注入，不再写死

#### 修复

- 修复搜索下拉面板背景透明导致内容混叠的问题
- 修复自定义下拉搜索过滤与 hover/高亮反馈问题
- Homebrew 安装文档改为标准 `brew install --cask nyro` 流程

#### 文档

- 新增路由与 API Key 设计文档：`docs/design/route-apikey.md`
- 新增 Provider Base URL/渠道设计说明：`docs/design/provider-base-urls.md`
- 更新 `README.md` 与 `README_CN.md` 安装命令及相关说明

---

## v1.0.1

> 发布于 2026-03-10

#### 改进

- **全平台 ARM64 / aarch64 原生构建**：使用 GitHub Actions ARM runner（`ubuntu-24.04-arm`、`windows-11-arm`、`macos-latest`）原生构建，零交叉编译
  - 桌面端：Linux aarch64 AppImage、Windows ARM64 NSIS 安装包
  - 服务端：Linux aarch64、macOS aarch64、Windows ARM64 二进制
- **macOS Intel 原生构建**：使用 `macos-15-intel` runner 原生编译，不再依赖 ARM 交叉编译
- **Homebrew Cask 支持**：`brew tap shuaijinchao/nyro && brew install --cask nyro`（独立 `homebrew-nyro` tap 仓库，发版自动同步版本）
- **一键安装脚本**：macOS/Linux（`install.sh`）和 Windows（`install.ps1`），macOS 自动移除隔离属性
- **前端 chunk 拆分**：Vite `manualChunks` 拆分 react/query/charts，消除 >500kB 打包警告

#### 修复

- **CI**：`cargo check --workspace` 排除 `nyro-desktop`，避免 Linux CI 依赖 GTK
- **CI**：移除 `cargo tauri build` 不支持的 `--manifest-path` 参数
- **CI**：添加 `pkg-config` 和 `libssl-dev` 依赖

#### 清理

- 移除桌面发布中的 MSI 和 deb 包（仅保留 NSIS + AppImage）
- 移除桌面 SHA256SUMS.txt（updater `.sig` 文件已提供完整性校验）
- Homebrew Cask 迁移至独立 `homebrew-nyro` 仓库
- 修复安装脚本和 README 中 `main` → `master` 分支引用

---

## v1.0.0

> 发布于 2026-03-09

Nyro AI Gateway 首个公开版本 — 从原 OpenResty/Lua API Gateway 完整重构为纯 Rust 本地 AI 协议网关。

#### 功能

- **多协议入口**：支持 OpenAI（`/v1/chat/completions`）、Anthropic（`/v1/messages`）、Gemini（`/v1beta/models/*/generateContent`），全协议支持流式（SSE）和非流式响应
- **任意上游出口**：可路由到任意 OpenAI 兼容、Anthropic、Gemini Provider
- **Provider 管理**：创建、编辑、删除 Provider，含 base URL 和加密 API Key
- **路由规则管理**：基于优先级的路由规则，支持模型覆盖和 Fallback Provider
- **请求日志持久化**：SQLite 存储，含协议、模型、延迟、状态码、Token 用量
- **用量统计看板**：概览仪表盘，含按小时/天图表和 Provider/模型维度分布
- **API Key 加密存储**：AES-256-GCM 加密静态存储
- **Bearer Token 鉴权**：代理层和管理层支持独立鉴权配置
- **桌面应用**：基于 Tauri v2 的跨平台桌面应用（macOS / Windows / Linux）
  - 系统托盘及快捷菜单
  - 可选开机自启
  - 应用内自动更新（Tauri updater）
  - macOS 原生标题栏融合
  - 深色/浅色模式切换
  - 中文/英文语言切换
- **服务端二进制**：独立 `nyro-server` 二进制，支持服务器部署，通过 HTTP 访问 WebUI
  - 代理端口和管理端口独立绑定地址配置
  - CORS 来源白名单配置
  - 非本地绑定时强制要求鉴权 Key
- **CI/CD**：GitHub Actions 自动化构建，支持跨平台桌面安装包和服务端二进制发布
