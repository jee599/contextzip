<p align="center">
  <h1 align="center">⚡ ContextZip</h1>
</p>

<p align="center">
  <strong>Claude Codeのコンテキストを60-90%圧縮。RTKにない6つのノイズフィルター。</strong>
</p>

<p align="center">
  <a href="https://github.com/jee599/contextzip/releases"><img src="https://img.shields.io/github/v/release/jee599/contextzip?style=flat-square" alt="Release" /></a>
  <a href="https://github.com/jee599/contextzip/actions"><img src="https://img.shields.io/github/actions/workflow/status/jee599/contextzip/ci.yml?style=flat-square" alt="CI" /></a>
  <a href="LICENSE"><img src="https://img.shields.io/github/license/jee599/contextzip?style=flat-square" alt="License" /></a>
  <a href="https://github.com/jee599/contextzip/stargazers"><img src="https://img.shields.io/github/stars/jee599/contextzip?style=flat-square" alt="Stars" /></a>
</p>

<p align="center">
  <a href="#-5秒セットアップ">インストール</a> •
  <a href="#-before--after">使用例</a> •
  <a href="#-ベンチマーク102テスト">ベンチマーク</a> •
  <a href="../README.md">English</a> •
  <a href="README.ko.md">한국어</a> •
  日本語 •
  <a href="README.zh.md">中文</a>
</p>

---

## 🔥 問題

Claude Codeは`git status`、`npm install`、`cargo test`を実行し、**生の出力**をコンテキストウィンドウにそのまま流し込む。

- `node_modules`のスタックトレース30行。意味があるのは3行だけ。
- `npm warn deprecated`が150行。意味があるのは0行。
- ANSIカラーコード、スピナー、プログレスバー。情報量ゼロ。

**結果：** コンテキスト上限に早く到達する。Claudeがコードを忘れる。コストが増える。

## ⚡ 解決策

ContextZipはCLI出力をインターセプトしてノイズを除去する。設定不要。オーバーヘッド<10ms。

```
Without ContextZip:          With ContextZip:
─────────────────────        ─────────────────────
Input: 2,000 tokens          Input: 2,000 tokens
       ↓                            ↓
Claude reads all 2,000       ContextZip filters
       ↓                            ↓
Context: 2,000 tokens        Context: 200 tokens
                              (saved 90%)
```

---

## 📦 5秒セットアップ

```bash
curl -fsSL https://raw.githubusercontent.com/jee599/contextzip/main/install.sh | bash
```

Claude Codeを再起動。完了。

<details>
<summary><b>その他のインストール方法</b></summary>

```bash
# Homebrew (macOS/Linux)
brew install jee599/tap/contextzip

# Cargo (Rust developers)
cargo install --git https://github.com/jee599/contextzip
```

</details>

> [!TIP]
> 確認: `contextzip --version` → `contextzip 0.1.0 (based on rtk 0.30.1)`

---

## 🚀 Quickstart

インストール後、すべてのコマンドがフック経由で自動的に圧縮される：

```bash
$ git status
* main...origin/main
M src/api/users.ts
💾 contextzip: 200 → 40 tokens (saved 80%)
```

プレフィックス不要。フックが`git status` → `contextzip git status`に透過的に変換する。

---

## 🔬 Before / After

**Node.jsエラー** — 30行 → 3行

```diff
- TypeError: Cannot read properties of undefined (reading 'id')
-     at getUserProfile (/app/src/api/users.ts:47:23)
-     at processAuth (/app/src/middleware/auth.ts:12:5)
-     at Layer.handle (/app/node_modules/express/lib/router/layer.js:95:5)
-     at next (/app/node_modules/express/lib/router/route.js:144:13)
-     ... 25 more node_modules frames

+ TypeError: Cannot read properties of undefined (reading 'id')
+   → src/api/users.ts:47         getUserProfile()
+   → src/middleware/auth.ts:12   processAuth()
+   (+ 27 framework frames hidden)
```

**93%削減。** エラーメッセージ + 自分のコード。Expressの内部ではない。

<details>
<summary><b>📦 その他の例</b></summary>

**`npm install`** — 150行 → 3行

```diff
- npm warn deprecated inflight@1.0.6: This module is not supported
- npm warn deprecated rimraf@3.0.2: Rimraf v3 is no longer supported
- ... 47 more deprecated warnings
- added 847 packages, and audited 848 packages in 32s
- 143 packages are looking for funding
- 8 vulnerabilities (2 moderate, 6 high)

+ ✓ 847 packages (32s)
+ ⚠ 8 vulnerabilities (6 high, 2 moderate)
+ ⚠ deprecated bcrypt@3.0.0: security vulnerability (CVE-2023-31484)
```

**95%削減。** セキュリティ警告は保持。ノイズは削除。

---

**Dockerビルド（成功）** — 50行 → 1行

```diff
- Step 1/12 : FROM node:20-alpine
-  ---> abc123def456
- Step 2/12 : WORKDIR /app
-  ---> Using cache
- ... 10 more steps with hashes and cache lines
- Successfully built abc123final
- Successfully tagged my-app:latest

+ ✓ built my-app:latest (12 steps, 8 cached)
```

**96%削減。**

---

**Dockerビルド（失敗）** — 重要な情報だけ保持：

```
✗ Docker build failed at step 7/12

Step 5/12 : COPY package*.json ./    (cached ✓)
Step 6/12 : RUN npm install          (cached ✓)
Step 7/12 : RUN npm run build        ← FAILED
  error: Module not found: 'react-dom/client'
  Exit code: 1
```

---

**Python Traceback** — フレームワークフレームを非表示：

```diff
- Traceback (most recent call last):
-   File "/app/main.py", line 10, in handler
-     process(data)
-   File "/usr/lib/python3.11/importlib/__init__.py", line 126, in import_module
-   File "/app/venv/lib/python3.11/site-packages/flask/app.py", line 1498, in __call__
- ValueError: invalid literal for int()

+ Traceback (most recent call last):
+   → /app/main.py:10         process(data)
+   (+ 2 framework frames hidden)
+ ValueError: invalid literal for int()
```

---

**Rust Panic** — std/tokioフレームを除去：

```diff
- thread 'main' panicked at 'index out of bounds', src/handler.rs:42:5
- stack backtrace:
-    0: std::panicking::begin_panic
-    1: core::panicking::panic_fmt
-    2: myapp::handler::process at ./src/handler.rs:42:5
-    3: myapp::main at ./src/main.rs:15:3
-    4: std::rt::lang_start
-    5: tokio::runtime::enter

+ thread 'main' panicked at 'index out of bounds', src/handler.rs:42:5
+   (+ 2 framework frames hidden)
+   → ./src/handler.rs:42  myapp::handler::process()
+   → ./src/main.rs:15     myapp::main()
+   (+ 2 framework frames hidden)
```

**80%削減。**

</details>

---

## 📊 ベンチマーク（102テスト）

本番相当の入力でテスト。[全結果 →](benchmark-results.md)

| カテゴリ | 件数 | 平均削減 | 最高 | 最低 |
|:---------|------:|------------:|-----:|------:|
| Dockerビルドログ | 10 | **88.2%** | 97% | 77% |
| ANSI/スピナー | 15 | **82.5%** | 98% | 41% |
| エラースタックトレース | 20 | **58.7%** | 97% | 2%* |
| ビルドエラー | 15 | **55.6%** | 90% | -10%* |
| Webページ | 15 | **42.5%** | 64% | 5% |
| CLIコマンド | 12 | **42.0%** | 78% | -56%* |
| パッケージインストール | 15 | **39.2%** | 99% | 2% |
| **全体** | **102** | **57.4%** | | |

> **加重平均: 61.1%** 削減 (326K chars in → 127K chars out)

\* マイナス = 出力が増加。極小入力でフォーマットのオーバーヘッドがノイズを超える場合に発生。

---

## 🔄 ContextZip vs RTK

[RTK](https://github.com/rtk-ai/rtk)ベース。RTKの全34コマンドを含み、さらに：

| ノイズソース | RTK | ContextZip |
|:-------------|:---:|:----------:|
| CLI出力 (git, test, ls) | ✓ | ✓ |
| エラースタックトレース（5言語） | ✗ | **✓** |
| Webページ取得 | ✗ | **✓** |
| ANSI/スピナー/装飾 | partial | **✓ enhanced** |
| ビルドエラーグルーピング | partial | **✓ enhanced** |
| パッケージインストールログ | ✗ | **✓** |
| Dockerビルドログ | partial | **✓ enhanced** |
| コマンド別削減量表示 | ✗ | **✓** |

---

## ⚙️ 仕組み

```
Claude Code hook intercepts bash command
       ↓
contextzip binary
  ├── [1] ANSI preprocessor ──→ strip escape codes, spinners
  ├── [2] Command router ──→ 40+ specialized filters
  ├── [3] Error post-processor ──→ compress stacktraces
  └── [4] SQLite tracking ──→ record savings
       ↓
Compressed output → Claude's context
💾 contextzip: 2,000 → 200 tokens (saved 90%)
```

---

## 📈 削減量トラッキング

```bash
$ contextzip gain
📊 ContextZip Token Savings
════════════════════════════════════
Total commands:    2,927
Tokens saved:      10.3M (89.2%)

$ contextzip gain --by-feature
Feature        Commands  Saved     Avg%
cli (RTK)      2,100     6.8M     78%
error          89        1.2M     93%
web            43        0.9M     73%
build          112       0.4M     81%
pkg            34        0.3M     95%
docker         22        0.2M     85%
```

```bash
contextzip gain --graph      # 日別削減チャート
contextzip gain --history    # 最近のコマンド詳細
```

---

## 🛡️ 安全保証

| 対象 | ルール |
|:-----|:-----|
| エラーメッセージ | **常に**保持（最初の行 + ユーザーコードフレーム） |
| ファイル:行の位置 | ビルドエラーから**絶対に**除去しない |
| セキュリティ警告 | **常に**保持（CVE, GHSA, vulnerability） |
| Docker失敗コンテキスト | **常に**保持（失敗ステップ + 前2ステップ + 終了コード） |
| 終了コード | **常に**伝播（CI/CDセーフ） |

> [!IMPORTANT]
> ContextZipは**確認済みのノイズのみ**除去する。判断に迷う場合、元の出力をそのまま通す。

---

## 🔧 CLIリファレンス

```bash
# 自動（フック経由 — プレフィックス不要）：
git status          # → contextzip git status
cargo test          # → contextzip cargo test
npm install         # → contextzip npm install

# 手動コマンド：
contextzip web https://docs.example.com    # ページコンテンツ抽出
contextzip err node server.js              # エラー特化出力

# 分析：
contextzip gain                  # 削減量ダッシュボード
contextzip gain --by-feature     # フィルター種別ごと
contextzip gain --graph          # 日別チャート
contextzip gain --history        # 最近のコマンド

# セットアップ：
contextzip init --show           # インストール状態確認
contextzip update                # セルフアップデート
contextzip uninstall             # クリーンアンインストール
```

---

## 🤝 Contributing

コントリビューション歓迎！ContextZipはRustプロジェクトです。

```bash
git clone https://github.com/jee599/contextzip.git
cd contextzip
cargo test         # 1056 tests
cargo clippy       # Lint check
```

## 📜 License

MIT — Based on [RTK](https://github.com/rtk-ai/rtk) by rtk-ai.

---

<p align="center">
  <sub>⚡ トークンを節約。もっと速くシップ。</sub>
</p>

[![Star History Chart](https://api.star-history.com/svg?repos=jee599/contextzip&type=Date)](https://star-history.com/#jee599/contextzip&Date)
