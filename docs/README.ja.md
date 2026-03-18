# TokenZip

**あなたのClaude Codeはトークンを無駄にしている。5秒で直せる。**

[English](../README.md) | [한국어](README.ko.md) | [日本語](#) | [中文](README.zh.md)

---

## 5秒セットアップ

```bash
# Homebrew (macOS/Linux)
brew install jee599/tap/tokenzip

# または curl
curl -fsSL https://raw.githubusercontent.com/jee599/tokenzip/main/install.sh | bash
```

Claude Codeを再起動。終わり。すべてのコマンドが自動的に圧縮される。

---

## 問題

Claude Codeが`git status`、`npm install`、`cargo test`を実行するたびに、生の出力がコンテキストウィンドウを食い尽くす。`node_modules`のスタックトレース30行。`npm warn deprecated`が150行。誰も読まないANSIカラーコード。

**結果：** コンテキスト上限に早く到達する。Claudeが以前のコードを忘れる。コストが増える。

## 解決策

TokenZipはCLI出力をインターセプトし、Claudeのコンテキストに届く前にノイズを除去する。設定不要。オーバーヘッドなし（<10ms）。

### 実例

**`git status` — Before vs After**

Before（生出力）:
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
（12行、約200トークン）

After（tokenzip）:
```
* main...origin/main
M src/api/users.ts
M src/api/orders.ts
? src/api/products.ts
```
（4行、約40トークン）— **80%削減**

---

**Node.jsエラー — Before vs After**

Before（30行、約1,500トークン）:
```
TypeError: Cannot read properties of undefined (reading 'id')
    at getUserProfile (/app/src/api/users.ts:47:23)
    at processAuth (/app/src/middleware/auth.ts:12:5)
    at Layer.handle (/app/node_modules/express/lib/router/layer.js:95:5)
    at next (/app/node_modules/express/lib/router/route.js:144:13)
    at Route.dispatch (/app/node_modules/express/lib/router/route.js:114:3)
    ... 25 more node_modules frames
```

After（3行、約100トークン）:
```
TypeError: Cannot read properties of undefined (reading 'id')
  → src/api/users.ts:47         getUserProfile()
  → src/middleware/auth.ts:12   processAuth()
  (+ 27 framework frames hidden)
```
**93%削減** — Claudeが見るのはエラーとあなたのコード。Expressの内部ではない。

---

**`npm install` — Before vs After**

Before（150行、約2,000トークン）:
```
npm warn deprecated inflight@1.0.6: This module is not supported...
npm warn deprecated rimraf@3.0.2: Rimraf v3 is no longer supported...
... 47 more deprecated warnings ...
added 847 packages, and audited 848 packages in 32s
143 packages are looking for funding
  run `npm fund` for details
8 vulnerabilities (2 moderate, 6 high)
```

After（3行、約50トークン）:
```
✓ 847 packages (32s)
⚠ 8 vulnerabilities (6 high, 2 moderate)
⚠ deprecated bcrypt@3.0.0: security vulnerability (CVE-2023-31484)
```
**95%削減** — セキュリティ警告は残す。ノイズは消す。

---

**Dockerビルド（成功）— Before vs After**

Before（50行）: ハッシュ、キャッシュ行、中間コンテナを含むステップごとの出力
After（1行）: `✓ built my-app:latest (12 steps, 8 cached)` — **96%削減**

**Dockerビルド（失敗）** — 重要な情報だけ保持：失敗ステップ + 前2ステップ + エラー + 終了コード。

---

## 圧縮対象

| ソース | 除去されるもの | 保持されるもの | 削減率 |
|--------|---------------|-------------|---------|
| **エラースタックトレース** | node_modules、site-packages、java.lang.reflectフレーム | エラーメッセージ + あなたのコードフレーム | ~93% |
| **Webページ** | nav、footer、広告、Cookie、スクリプト | 記事本文、コードブロック、テーブル | ~73% |
| **ANSI/スピナー** | カラーコード、プログレスバー、装飾 | 最終ステータス、エラー、タイムスタンプ | ~85% |
| **ビルドエラー** | 同一TS2322の40回重複 | エラーコード別グループ化、全行番号保持 | ~81% |
| **パッケージインストール** | deprecated、funding、resolution | サマリー + セキュリティ警告 | ~95% |
| **Dockerビルド** | レイヤーハッシュ、キャッシュ行、pull進捗 | 成功：1行。失敗：コンテキスト | ~96% |
| **CLI出力** | git/test/lsの冗長出力 | 必要な情報のみ（RTK経由） | ~78% |

---

## すべてのコマンドに削減量を表示

```
$ git status
* main...origin/main
M src/api/users.ts
💾 tokenzip: 200 → 40 tokens (saved 80%)
```

累計削減量はいつでも確認できる：

```bash
tokenzip gain                  # 削減量ダッシュボード
tokenzip gain --by-feature     # フィルター別削減量
tokenzip gain --graph          # 日別削減量チャート
tokenzip gain --history        # 最近のコマンド詳細
```

---

## CLIリファレンス

```bash
# フック経由で自動適用：
git status          # → tokenzip git status（圧縮）
cargo test          # → tokenzip cargo test（失敗のみ）
npm install         # → tokenzip npm install（ノイズ除去）
docker build .      # → tokenzip docker build（要約）

# 手動コマンド：
tokenzip web https://docs.example.com    # ページコンテンツ抽出
tokenzip err node server.js              # エラー特化出力

# 分析：
tokenzip gain                  # 削減量ダッシュボード
tokenzip gain --by-feature     # フィルター種別ごと
tokenzip gain --graph          # 日別チャート
tokenzip gain --history        # 最近のコマンド

# セットアップ：
tokenzip init -g --auto-patch  # フックインストール（インストーラーが実行済み）
tokenzip init --show           # インストール状態確認
tokenzip update                # セルフアップデート
tokenzip uninstall             # クリーンアンインストール
```

---

## 仕組み

1. Claude Codeフックがbashコマンドをインターセプト
2. TokenZipが出力を圧縮（ANSI → コマンドフィルター → エラー後処理）
3. 圧縮結果がClaudeのコンテキストに渡される
4. コマンドごとに削減量が表示される

**設定不要。オーバーヘッドなし。無駄だけ減らす。**

---

## RTKベース

TokenZipは[RTK (Rust Token Killer)](https://github.com/rtk-ai/rtk)のフォーク。6つのノイズフィルターを追加。RTKの全34コマンドを含む。MITライセンス。
