<p align="center">
  <h1 align="center">⚡ ContextZip</h1>
</p>

<p align="center">
  <strong>Claude Code 컨텍스트를 60-90% 압축한다. RTK에 없는 6개 노이즈 필터.</strong>
</p>

<p align="center">
  <a href="https://github.com/jee599/contextzip/releases"><img src="https://img.shields.io/github/v/release/jee599/contextzip?style=flat-square" alt="Release" /></a>
  <a href="https://github.com/jee599/contextzip/actions"><img src="https://img.shields.io/github/actions/workflow/status/jee599/contextzip/ci.yml?style=flat-square" alt="CI" /></a>
  <a href="LICENSE"><img src="https://img.shields.io/github/license/jee599/contextzip?style=flat-square" alt="License" /></a>
  <a href="https://github.com/jee599/contextzip/stargazers"><img src="https://img.shields.io/github/stars/jee599/contextzip?style=flat-square" alt="Stars" /></a>
</p>

<p align="center">
  <a href="#-5초-설치">설치</a> •
  <a href="#-before--after">예시</a> •
  <a href="#-벤치마크-102개-테스트">벤치마크</a> •
  <a href="../README.md">English</a> •
  한국어 •
  <a href="README.ja.md">日本語</a> •
  <a href="README.zh.md">中文</a>
</p>

---

## 🔥 문제

Claude Code가 `git status`, `npm install`, `cargo test`를 실행하면 **원시 출력**을 컨텍스트 윈도우에 그대로 쏟아낸다.

- `node_modules` 스택트레이스 30줄. 의미 있는 건 3줄.
- `npm warn deprecated` 150줄. 의미 있는 건 0줄.
- ANSI 컬러 코드, 스피너, 프로그레스 바. 정보량 제로.

**결과:** 컨텍스트 한도에 빨리 도달한다. Claude가 코드를 잊는다. 비용이 올라간다.

## ⚡ 해결

ContextZip이 CLI 출력을 가로채서 노이즈를 제거한다. 설정 없음. 오버헤드 <10ms.

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

## 📦 5초 설치

```bash
curl -fsSL https://raw.githubusercontent.com/jee599/contextzip/main/install.sh | bash
```

Claude Code 재시작. 끝.

<details>
<summary><b>다른 설치 방법</b></summary>

```bash
# Homebrew (macOS/Linux)
brew install jee599/tap/contextzip

# Cargo (Rust developers)
cargo install --git https://github.com/jee599/contextzip
```

</details>

> [!TIP]
> 확인: `contextzip --version` → `contextzip 0.1.0 (based on rtk 0.30.1)`

---

## 🚀 Quickstart

설치 후 모든 명령어가 훅을 통해 자동으로 압축된다:

```bash
$ git status
* main...origin/main
M src/api/users.ts
💾 contextzip: 200 → 40 tokens (saved 80%)
```

별도 접두사 불필요. 훅이 `git status` → `contextzip git status`로 투명하게 변환한다.

---

## 🔬 Before / After

**Node.js 에러** — 30줄 → 3줄

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

**93% 절감.** 에러 메시지 + 내 코드. Express 내부가 아니다.

<details>
<summary><b>📦 더 많은 예시</b></summary>

**`npm install`** — 150줄 → 3줄

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

**95% 절감.** 보안 경고는 유지. 노이즈는 삭제.

---

**Docker 빌드 (성공)** — 50줄 → 1줄

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

**96% 절감.**

---

**Docker 빌드 (실패)** — 중요한 것만 보존:

```
✗ Docker build failed at step 7/12

Step 5/12 : COPY package*.json ./    (cached ✓)
Step 6/12 : RUN npm install          (cached ✓)
Step 7/12 : RUN npm run build        ← FAILED
  error: Module not found: 'react-dom/client'
  Exit code: 1
```

---

**Python Traceback** — 프레임워크 프레임 숨김:

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

**Rust Panic** — std/tokio 프레임 제거:

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

**80% 절감.**

</details>

---

## 📊 벤치마크 (102개 테스트)

프로덕션 수준 입력으로 테스트. [전체 결과 →](benchmark-results.md)

| 카테고리 | 건수 | 평균 절감 | 최고 | 최저 |
|:---------|------:|------------:|-----:|------:|
| Docker 빌드 로그 | 10 | **88.2%** | 97% | 77% |
| ANSI/스피너 | 15 | **82.5%** | 98% | 41% |
| 에러 스택트레이스 | 20 | **58.7%** | 97% | 2%* |
| 빌드 에러 | 15 | **55.6%** | 90% | -10%* |
| 웹 페이지 | 15 | **42.5%** | 64% | 5% |
| CLI 명령어 | 12 | **42.0%** | 78% | -56%* |
| 패키지 설치 | 15 | **39.2%** | 99% | 2% |
| **전체** | **102** | **57.4%** | | |

> **가중 평균: 61.1%** 절감 (326K chars in → 127K chars out)

\* 음수 = 출력이 늘어남. 입력이 극소량일 때 포맷 오버헤드가 노이즈보다 큰 경우 발생.

---

## 🔄 ContextZip vs RTK

[RTK](https://github.com/rtk-ai/rtk) 기반. RTK 34개 명령어 전부 포함 + 추가 기능:

| 노이즈 소스 | RTK | ContextZip |
|:-------------|:---:|:----------:|
| CLI 출력 (git, test, ls) | ✓ | ✓ |
| 에러 스택트레이스 (5개 언어) | ✗ | **✓** |
| 웹 페이지 추출 | ✗ | **✓** |
| ANSI/스피너/장식 | partial | **✓ enhanced** |
| 빌드 에러 그룹핑 | partial | **✓ enhanced** |
| 패키지 설치 로그 | ✗ | **✓** |
| Docker 빌드 로그 | partial | **✓ enhanced** |
| 명령어별 절감량 표시 | ✗ | **✓** |

---

## ⚙️ 동작 방식

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

## 📈 절감량 추적

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
contextzip gain --graph      # 일별 절감 차트
contextzip gain --history    # 최근 명령어 상세
```

---

## 🛡️ 안전 보장

| 대상 | 규칙 |
|:-----|:-----|
| 에러 메시지 | **항상** 보존 (첫 줄 + 사용자 코드 프레임) |
| 파일:라인 위치 | 빌드 에러에서 **절대** 제거하지 않음 |
| 보안 경고 | **항상** 유지 (CVE, GHSA, vulnerability) |
| Docker 실패 컨텍스트 | **항상** 보존 (실패 단계 + 이전 2단계 + 종료 코드) |
| 종료 코드 | **항상** 전파 (CI/CD 안전) |

> [!IMPORTANT]
> ContextZip은 **확인된 노이즈만** 제거한다. 확신이 없으면 원본 출력을 그대로 통과시킨다.

---

## 🔧 CLI 레퍼런스

```bash
# 자동 (훅 경유 — 접두사 불필요):
git status          # → contextzip git status
cargo test          # → contextzip cargo test
npm install         # → contextzip npm install

# 수동 명령어:
contextzip web https://docs.example.com    # 페이지 콘텐츠 추출
contextzip err node server.js              # 에러 중심 출력

# 분석:
contextzip gain                  # 절감량 대시보드
contextzip gain --by-feature     # 필터 유형별
contextzip gain --graph          # 일별 차트
contextzip gain --history        # 최근 명령어

# 설정:
contextzip init --show           # 설치 상태 확인
contextzip update                # 셀프 업데이트
contextzip uninstall             # 깔끔한 제거
```

---

## 🤝 Contributing

기여 환영! ContextZip은 Rust 프로젝트다.

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
  <sub>⚡ 토큰을 아끼고. 더 빠르게 배포하고.</sub>
</p>

[![Star History Chart](https://api.star-history.com/svg?repos=jee599/contextzip&type=Date)](https://star-history.com/#jee599/contextzip&Date)
