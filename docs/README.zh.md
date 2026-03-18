# TokenZip

**你的 Claude Code 正在浪费 token。5 秒搞定。**

[English](../README.md) | [한국어](README.ko.md) | [日本語](README.ja.md) | [中文](#)

---

## 5 秒安装

```bash
# Homebrew (macOS/Linux)
brew install jee599/tap/tokenzip

# 或 curl
curl -fsSL https://raw.githubusercontent.com/jee599/tokenzip/main/install.sh | bash
```

重启 Claude Code。搞定。所有命令自动压缩。

---

## 问题

Claude Code 每次执行 `git status`、`npm install`、`cargo test`，原始输出都在吞噬你的上下文窗口。30 行 `node_modules` 堆栈帧。150 行 `npm warn deprecated`。没人看的 ANSI 颜色代码。

**后果：** 上下文上限提前触达。Claude 忘掉之前的代码。费用增加。

## 解决方案

TokenZip 拦截 CLI 输出，在到达 Claude 上下文之前剥离噪音。零配置。零开销（<10ms）。

### 真实案例

**`git status` — 前后对比**

压缩前（原始）：
```
On branch main
Your branch is up to date with 'origin/main'.

Changes not staged for commit:
  (use "git add <file>..." to update what will be committed)
  (use "git restore <file>..." to discard changes in working directory)
        modified:   src/api/users.ts
        modified:   src/api/orders.ts

Untracked files:
  (use "git add <file>..." to include in what will be committed)
        src/api/products.ts

no changes added to commit
```
（12 行，约 200 token）

压缩后（tokenzip）：
```
* main...origin/main
M src/api/users.ts
M src/api/orders.ts
? src/api/products.ts
```
（4 行，约 40 token）— **节省 80%**

---

**Node.js 报错 — 前后对比**

压缩前（30 行，约 1,500 token）：
```
TypeError: Cannot read properties of undefined (reading 'id')
    at getUserProfile (/app/src/api/users.ts:47:23)
    at processAuth (/app/src/middleware/auth.ts:12:5)
    at Layer.handle (/app/node_modules/express/lib/router/layer.js:95:5)
    at next (/app/node_modules/express/lib/router/route.js:144:13)
    at Route.dispatch (/app/node_modules/express/lib/router/route.js:114:3)
    ... 25 more node_modules frames
```

压缩后（3 行，约 100 token）：
```
TypeError: Cannot read properties of undefined (reading 'id')
  → src/api/users.ts:47         getUserProfile()
  → src/middleware/auth.ts:12   processAuth()
  (+ 27 framework frames hidden)
```
**节省 93%** — Claude 看到的是报错和你的代码，不是 Express 内部实现。

---

**`npm install` — 前后对比**

压缩前（150 行，约 2,000 token）：
```
npm warn deprecated inflight@1.0.6: This module is not supported...
npm warn deprecated rimraf@3.0.2: Rimraf v3 is no longer supported...
... 47 more deprecated warnings ...
added 847 packages, and audited 848 packages in 32s
143 packages are looking for funding
  run `npm fund` for details
8 vulnerabilities (2 moderate, 6 high)
```

压缩后（3 行，约 50 token）：
```
✓ 847 packages (32s)
⚠ 8 vulnerabilities (6 high, 2 moderate)
⚠ deprecated bcrypt@3.0.0: security vulnerability (CVE-2023-31484)
```
**节省 95%** — 安全警告保留，噪音删除。

---

**Docker 构建（成功）— 前后对比**

压缩前（50 行）：包含哈希、缓存行、中间容器的逐步输出
压缩后（1 行）：`✓ built my-app:latest (12 steps, 8 cached)` — **节省 96%**

**Docker 构建（失败）** — 只保留关键信息：失败步骤 + 前 2 步上下文 + 报错 + 退出码。

---

## 压缩对象

| 来源 | 移除内容 | 保留内容 | 节省率 |
|--------|---------------|-------------|---------|
| **错误堆栈** | node_modules、site-packages、java.lang.reflect 帧 | 错误信息 + 你的代码帧 | ~93% |
| **网页** | nav、footer、广告、cookie、脚本 | 正文内容、代码块、表格 | ~73% |
| **ANSI/加载动画** | 颜色代码、进度条、装饰字符 | 最终状态、错误、时间戳 | ~85% |
| **构建错误** | 40 次重复 TS2322 | 按错误码分组，保留所有行号 | ~81% |
| **包安装** | deprecated、funding、resolution | 摘要 + 安全警告 | ~95% |
| **Docker 构建** | 层哈希、缓存行、pull 进度 | 成功：1 行。失败：上下文 | ~96% |
| **CLI 输出** | git/test/ls 冗余输出 | 仅保留关键信息（经由 RTK） | ~78% |

---

## 每条命令显示节省量

```
$ git status
* main...origin/main
M src/api/users.ts
💾 tokenzip: 200 → 40 tokens (saved 80%)
```

随时查看累计节省：

```bash
tokenzip gain                  # 节省量仪表盘
tokenzip gain --by-feature     # 按过滤器类型
tokenzip gain --graph          # 每日节省图表
tokenzip gain --history        # 最近命令详情
```

---

## CLI 参考

```bash
# 通过 hook 自动生效：
git status          # → tokenzip git status（压缩）
cargo test          # → tokenzip cargo test（仅失败）
npm install         # → tokenzip npm install（去噪）
docker build .      # → tokenzip docker build（摘要）

# 手动命令：
tokenzip web https://docs.example.com    # 提取页面内容
tokenzip err node server.js              # 错误聚焦输出

# 分析：
tokenzip gain                  # 节省量仪表盘
tokenzip gain --by-feature     # 按过滤器类型
tokenzip gain --graph          # 每日图表
tokenzip gain --history        # 最近命令

# 设置：
tokenzip init -g --auto-patch  # 安装 hook（安装器已完成）
tokenzip init --show           # 检查安装状态
tokenzip update                # 自更新
tokenzip uninstall             # 干净卸载
```

---

## 工作原理

1. Claude Code hook 拦截 bash 命令
2. TokenZip 压缩输出（ANSI → 命令过滤器 → 错误后处理）
3. 压缩结果传入 Claude 上下文
4. 每条命令后显示节省量

**零配置。零开销。只减少浪费。**

---

## 基于 RTK

TokenZip 是 [RTK (Rust Token Killer)](https://github.com/rtk-ai/rtk) 的 fork，新增 6 个噪音过滤器。包含 RTK 全部 34 个命令。MIT 许可证。
