# TokenZip

**Claude Code가 토큰을 낭비하고 있다. 5초면 고친다.**

[English](../README.md) | [한국어](#) | [日本語](README.ja.md) | [中文](README.zh.md)

---

## 5초 설치

```bash
# Homebrew (macOS/Linux)
brew install jee599/tap/tokenzip

# 또는 curl
curl -fsSL https://raw.githubusercontent.com/jee599/tokenzip/main/install.sh | bash
```

Claude Code 재시작. 끝. 모든 명령어가 자동으로 압축된다.

---

## 문제

Claude Code가 `git status`, `npm install`, `cargo test`를 실행할 때마다 원시 출력이 컨텍스트 윈도우를 잡아먹는다. `node_modules` 스택트레이스 30줄. `npm warn deprecated` 150줄. 아무도 안 읽는 ANSI 컬러 코드.

**결과:** 컨텍스트 한도에 더 빨리 도달한다. Claude가 이전 코드를 잊는다. 비용이 올라간다.

## 해결

TokenZip이 CLI 출력을 가로채서 Claude 컨텍스트에 도달하기 전에 노이즈를 제거한다. 설정 없음. 오버헤드 없음 (<10ms).

### 실제 예시

**`git status` — Before vs After**

Before (원시):
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
(12줄, ~200 토큰)

After (tokenzip):
```
* main...origin/main
M src/api/users.ts
M src/api/orders.ts
? src/api/products.ts
```
(4줄, ~40 토큰) — **80% 절감**

---

**Node.js 에러 — Before vs After**

Before (30줄, ~1,500 토큰):
```
TypeError: Cannot read properties of undefined (reading 'id')
    at getUserProfile (/app/src/api/users.ts:47:23)
    at processAuth (/app/src/middleware/auth.ts:12:5)
    at Layer.handle (/app/node_modules/express/lib/router/layer.js:95:5)
    at next (/app/node_modules/express/lib/router/route.js:144:13)
    at Route.dispatch (/app/node_modules/express/lib/router/route.js:114:3)
    ... 25 more node_modules frames
```

After (3줄, ~100 토큰):
```
TypeError: Cannot read properties of undefined (reading 'id')
  → src/api/users.ts:47         getUserProfile()
  → src/middleware/auth.ts:12   processAuth()
  (+ 27 framework frames hidden)
```
**93% 절감** — Claude가 보는 건 에러와 내 코드. Express 내부가 아니다.

---

**`npm install` — Before vs After**

Before (150줄, ~2,000 토큰):
```
npm warn deprecated inflight@1.0.6: This module is not supported...
npm warn deprecated rimraf@3.0.2: Rimraf v3 is no longer supported...
... 47 more deprecated warnings ...
added 847 packages, and audited 848 packages in 32s
143 packages are looking for funding
  run `npm fund` for details
8 vulnerabilities (2 moderate, 6 high)
```

After (3줄, ~50 토큰):
```
✓ 847 packages (32s)
⚠ 8 vulnerabilities (6 high, 2 moderate)
⚠ deprecated bcrypt@3.0.0: security vulnerability (CVE-2023-31484)
```
**95% 절감** — 보안 경고는 유지. 노이즈는 삭제.

---

**Docker 빌드 (성공) — Before vs After**

Before (50줄): 해시, 캐시 라인, 중간 컨테이너 포함 단계별 출력
After (1줄): `✓ built my-app:latest (12 steps, 8 cached)` — **96% 절감**

**Docker 빌드 (실패)** — 중요한 것만 보존: 실패 단계 + 이전 2단계 + 에러 + 종료 코드.

---

## 압축 대상

| 소스 | 제거되는 것 | 보존되는 것 | 절감률 |
|--------|---------------|-------------|---------|
| **에러 스택트레이스** | node_modules, site-packages, java.lang.reflect 프레임 | 에러 메시지 + 내 코드 프레임 | ~93% |
| **웹 페이지** | nav, footer, 광고, 쿠키, 스크립트 | 본문, 코드 블록, 테이블 | ~73% |
| **ANSI/스피너** | 컬러 코드, 프로그레스 바, 장식 | 최종 상태, 에러, 타임스탬프 | ~85% |
| **빌드 에러** | 동일 TS2322 40개 반복 | 에러 코드별 그룹화, 모든 라인 번호 보존 | ~81% |
| **패키지 설치** | deprecated, funding, resolution | 요약 + 보안 경고 | ~95% |
| **Docker 빌드** | 레이어 해시, 캐시 라인, pull 진행률 | 성공: 1줄. 실패: 컨텍스트 | ~96% |
| **CLI 출력** | git/test/ls 장황한 출력 | 핵심 정보만 (RTK 경유) | ~78% |

---

## 모든 명령어에 절감량 표시

```
$ git status
* main...origin/main
M src/api/users.ts
💾 tokenzip: 200 → 40 tokens (saved 80%)
```

누적 절감량을 언제든 확인할 수 있다:

```bash
tokenzip gain                  # 절감량 대시보드
tokenzip gain --by-feature     # 필터별 절감량
tokenzip gain --graph          # 일별 절감량 차트
tokenzip gain --history        # 최근 명령어 상세
```

---

## CLI 레퍼런스

```bash
# 훅으로 자동 적용:
git status          # → tokenzip git status (압축)
cargo test          # → tokenzip cargo test (실패만)
npm install         # → tokenzip npm install (노이즈 제거)
docker build .      # → tokenzip docker build (요약)

# 수동 명령어:
tokenzip web https://docs.example.com    # 페이지 콘텐츠 추출
tokenzip err node server.js              # 에러 중심 출력

# 분석:
tokenzip gain                  # 절감량 대시보드
tokenzip gain --by-feature     # 필터 유형별
tokenzip gain --graph          # 일별 차트
tokenzip gain --history        # 최근 명령어

# 설정:
tokenzip init -g --auto-patch  # 훅 설치 (인스톨러가 처리)
tokenzip init --show           # 설치 상태 확인
tokenzip update                # 셀프 업데이트
tokenzip uninstall             # 깔끔한 제거
```

---

## 동작 방식

1. Claude Code 훅이 bash 명령어를 가로챈다
2. TokenZip이 출력을 압축한다 (ANSI → 명령어 필터 → 에러 후처리)
3. 압축된 결과가 Claude 컨텍스트로 전달된다
4. 매 명령어마다 절감량을 볼 수 있다

**설정 없음. 오버헤드 없음. 낭비만 줄인다.**

---

## RTK 기반

TokenZip은 [RTK (Rust Token Killer)](https://github.com/rtk-ai/rtk)의 포크다. 6개의 노이즈 필터를 추가했다. RTK의 34개 명령어 모두 포함. MIT 라이선스.
