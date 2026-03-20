# Cowork 자동화 — ContextZip + AgentCrow 바이럴 전략

모든 Cowork 태스크를 아래에 정리. 복사해서 Claude Desktop Cowork에 넣으면 됨.

---

## Task 1: X/Twitter Daily Post
**Name:** X Daily Post
**Schedule:** 매일 오전 9시

```
Chrome을 열어서 x.com에 접속해. 로그인 되어있을 거야.

오늘 날짜를 확인하고 아래 스케줄에서 해당하는 글을 트윗으로 올려줘.
날짜가 3/30 이후면 다시 3/21부터 순환해.

## 스케줄

3/21: Building in public 🔨 First project: ContextZip — compresses CLI output by 60-90% for AI coding assistants. Rust. 1,056 tests. npx contextzip github.com/jee599/contextzip

3/22: npm install dumps 150 lines of deprecated warnings into your AI context window. ContextZip turns it into 3 lines. Security warnings? Kept. npx contextzip github.com/jee599/contextzip

3/23: Claude Code Agent Teams is powerful. But it doesn't know WHICH agents to use. AgentCrow auto-matches from 9 builtin agents. One command: npx agentcrow init github.com/jee599/agentcrow 🐦

3/24: Claude Code keeps forgetting my code. Not a bug — npm install, Docker build, stacktraces all dump noise into the context window. I built ContextZip to fix it. npx contextzip github.com/jee599/contextzip

3/25: "Build a landing page" → AgentCrow auto-decomposes into 4 subtasks, picks the right agent for each, dispatches in parallel. No manual prompt splitting. npx agentcrow init github.com/jee599/agentcrow 🐦

3/26: Docker build output: 50 lines of layer hashes, cache lines, intermediate containers. ContextZip turns it into 1 line: "✓ built my-app:latest (12 steps, 8 cached)" github.com/jee599/contextzip

3/27: Two open-source tools I built for Claude Code: ContextZip — compress noisy CLI output (60-90% savings) AgentCrow — auto-dispatch 9 specialized agents Both: npx install, zero config. github.com/jee599/contextzip github.com/jee599/agentcrow

3/28: 102 real benchmarks for ContextZip. Best: 97% saved (Go errors). Worst: -12% (Java, output grew). We show both because hiding failures is dishonest. github.com/jee599/contextzip

3/29: AgentCrow 9 agents: frontend-dev, backend-dev, qa-tester, devops, security-auditor, tech-writer, code-reviewer, refactor-specialist, performance-optimizer. All auto-matched to your prompt. npx agentcrow init github.com/jee599/agentcrow 🐦

3/30: Every command now shows how much context you saved: 💾 contextzip: 200 → 40 tokens (saved 80%) Track your total savings: contextzip gain github.com/jee599/contextzip

## 트윗 올린 후 추가 작업

1. 타임라인에서 AI, Claude, Rust, 개발 관련 트윗 3개에 좋아요 누르기
2. 그 중 가장 관련 있는 트윗 1개에 짧은 리플라이 달기. 자연스럽게. 예시:
   - "Great point. Context management is the hardest part of AI-assisted coding."
   - "Rust for CLI tools is unbeatable. Zero overhead."
   - "Been dealing with the same issue. The context window fills up fast."
3. 절대 ContextZip이나 AgentCrow를 리플라이에서 홍보하지 마. 트윗에서만 홍보하고 리플라이는 순수하게 다른 사람 글에 engagement.
```

---

## Task 2: Reddit Daily Engagement
**Name:** Reddit Daily Engagement
**Schedule:** 매일 오전 10시

```
Chrome을 열어서 reddit.com에 접속해. 로그인 되어있을 거야.

## 매일 하는 일 (karma 축적 + 커뮤니티 참여)

1. r/ClaudeAI 접속
   - 최신 또는 인기 글 3개에 upvote
   - 그 중 1개에 도움이 되는 리플라이 달기. 홍보 금지. 진짜 도움이 되는 답변만.
   - 예시: "I ran into this too. What fixed it for me was...", "Have you checked your hook settings?", "This approach works well with the latest update."

2. r/rust 접속
   - 최신 글 2개에 upvote
   - CLI/도구 관련 글이 있으면 1개에 짧은 리플라이

3. r/programming 접속
   - 최신 글 2개에 upvote

4. r/LocalLLaMA 접속
   - 최신 글 2개에 upvote
   - LLM 토큰/컨텍스트 관련 글이 있으면 1개에 리플라이

## 중요 규칙
- 절대 ContextZip이나 AgentCrow를 언급하지 마
- 셀프 프로모션 금지. 순수하게 커뮤니티 참여만
- 자연스러운 톤으로. 봇처럼 보이면 안 됨
- 매일 다른 글에 리플라이 (같은 글 반복 금지)
```

---

## Task 3: LinkedIn Daily Post
**Name:** LinkedIn Daily Post
**Schedule:** 매일 오전 11시

```
Chrome을 열어서 linkedin.com에 접속해. 로그인 되어있을 거야.

오늘 날짜를 확인하고 아래 스케줄에서 해당하는 글을 LinkedIn 포스트로 올려줘.
날짜가 3/30 이후면 다시 3/21부터 순환해.

## 스케줄

3/21: 🔧 Building in public — CLI 출력이 AI 컨텍스트 윈도우를 잡아먹는 문제를 해결하는 오픈소스를 만들었다. npm 경고 150줄, stacktrace 30줄, Docker 해시 50줄 → ContextZip으로 60-90% 압축. Rust, 1,056 tests. npx contextzip https://github.com/jee599/contextzip #OpenSource #AI #DeveloperTools #BuildInPublic

3/22: 💡 Claude Code로 작업하다 보면 npm install 출력이 컨텍스트를 잡아먹어서 Claude가 이전 코드를 까먹는다. ContextZip은 deprecated 경고를 제거하고 보안 경고(CVE, GHSA)만 남긴다. 150줄 → 3줄. https://github.com/jee599/contextzip #ClaudeCode #Rust #DevTools

3/23: 🐦 Claude Code Agent Teams는 강력하지만, 어떤 에이전트를 써야 할지는 알려주지 않는다. AgentCrow는 프롬프트를 분석해서 9개 전문 에이전트(frontend, backend, QA, devops, security...) 중 적합한 걸 자동 매칭한다. npx agentcrow init https://github.com/jee599/agentcrow #AI #Automation #AgentTeams

3/24: 🧵 Claude Code를 위한 두 가지 오픈소스 도구를 만들었다. 1) ContextZip — CLI 출력 60-90% 압축 2) AgentCrow — 9개 전문 에이전트 자동 디스패치 둘 다 npx 한 줄 설치, 설정 없음. https://github.com/jee599/contextzip https://github.com/jee599/agentcrow #BuildInPublic #OpenSource

3/25: 📊 ContextZip 102개 실제 벤치마크 결과. Docker build: 평균 88% 절약. Error stacktrace: 59%. Package install: 39-99%. 정직한 숫자 — worst case도 README에 공개한다. https://github.com/jee599/contextzip #Engineering #Transparency

3/26: 🐦 "랜딩 페이지 만들어줘" → AgentCrow가 자동으로 4개 서브태스크로 분해, frontend-dev + designer + qa-tester를 매칭, 병렬 디스패치. 수동 프롬프트 분할 불필요. npx agentcrow init https://github.com/jee599/agentcrow #AI #Productivity

3/27: 🦀 Rust를 직접 쓰지 않고 Rust CLI를 만들었다. Claude Code 에이전트가 구현 — 1,056 tests, 102 benchmarks, 40+ 커맨드 모듈. AI 도구를 AI가 만든 메타적 프로젝트. https://github.com/jee599/contextzip #Rust #AI #BuildInPublic

3/28: 🔒 ContextZip 안전 보장: 에러 메시지 항상 보존, 보안 경고(CVE/GHSA) 절대 제거 안 함, exit code 항상 전달. 확실한 노이즈만 제거한다. 의심되면 원본 그대로 통과. https://github.com/jee599/contextzip #Security #DevTools

3/29: 🐦 AgentCrow 9개 에이전트: frontend-dev, backend-dev, qa-tester, devops, security-auditor, tech-writer, code-reviewer, refactor-specialist, performance-optimizer. 프롬프트에 자동 매칭. https://github.com/jee599/agentcrow #AI #AgentTeams

3/30: 🚀 1인 개발자의 오픈소스 2개. ContextZip (CLI 출력 압축, Rust) + AgentCrow (에이전트 자동 매칭, TypeScript). 둘 다 npx 한 줄 설치. AI로 만들고, AI를 위해 만든 도구. https://github.com/jee599/contextzip https://github.com/jee599/agentcrow #BuildInPublic #1인개발

## 포스트 올린 후

1. 피드에서 AI, 개발, 오픈소스 관련 포스트 3개에 좋아요 누르기
2. 그 중 1개에 짧은 댓글 달기. 자연스럽게. 홍보 금지.
```

---

## Task 4: HN Daily Engagement (karma 축적)
**Name:** HN Daily Comment
**Schedule:** 매일 오후 1시

```
Chrome을 열어서 news.ycombinator.com에 접속해. 로그인 되어있을 거야.

1. 첫 페이지에서 AI, Rust, CLI, developer tools 관련 글을 찾아.
2. 그 글의 댓글 섹션에 가서 유용한 댓글 1개를 달아.
3. 댓글은 기술적이고 도움이 되는 내용이어야 해. 예시:
   - "We solved a similar problem by compressing CLI output before it reaches the context window. The key insight was that 80%+ of stacktrace lines are framework internals."
   - "Rust's lazy_static for regex compilation makes a huge difference in CLI tool startup time. We measured <10ms overhead."
   - "The real challenge with LLM context is not the model's limit — it's the noise ratio. Most CLI output is 60-90% irrelevant."
4. 자연스럽게 경험을 공유하되, 프로젝트 이름(ContextZip, AgentCrow)은 직접 언급하지 마. 누가 물어보면 그때 공유.
5. 다른 흥미로운 글 2개에 upvote 하기.

## 중요
- 하루에 댓글 1개만. 많으면 스팸으로 보임.
- 기술적 깊이가 있는 댓글만. "Great post!" 같은 건 안 됨.
- karma 20+ 될 때까지 매일 꾸준히.
```

---

## Task 5: D-Day 런칭 (화요일 3/24 밤 10시 — 1회성)
**Name:** D-Day Launch
**Schedule:** 3/24 22:00 (1회)

```
오늘이 런칭 D-Day다. 아래를 순서대로 20분 안에 모두 실행해.

## 1. Hacker News Show HN (22:00)

Chrome에서 news.ycombinator.com/submit 접속.

title: Show HN: ContextZip – Compress CLI output by 70% for AI coding assistants (Rust)
url: https://github.com/jee599/contextzip

제출 후 바로 첫 댓글 달기:

"Hi HN, I built ContextZip because Claude Code kept forgetting my code — the context window was full of npm warnings, node_modules stacktraces, and Docker layer hashes.

It hooks into Claude Code's PreToolUse system and compresses CLI output before it reaches the context. Fork of RTK with 6 additional filters:

1. Error stacktraces — hides framework frames for Node.js, Python, Rust, Go, Java
2. Package install — removes deprecated noise, keeps security warnings (CVE/GHSA)
3. Docker build — success = 1 line, failure = context preserved
4. ANSI/spinners — strips escape codes, progress bars
5. Build errors — groups 40 identical TS2322 into one, all line numbers preserved
6. Web pages — strips nav/footer/ads, keeps content

102 real-world benchmarks, weighted avg 61% savings. Honest numbers — some edge cases have negative savings.

Tech: Rust, single-threaded, <10ms overhead, 1,056 tests.

Install: npx contextzip (macOS/Linux/Windows)

Happy to answer questions."

## 2. Twitter/X 런칭 스레드 (22:05)

x.com 접속. 스레드로 올리기:

트윗 1: "Claude Code keeps forgetting my code. Not because of a bug. Because npm install dumps 150 lines of deprecated warnings into the context window. I fixed it. Open source: npx contextzip"

트윗 2: "Before → After: Node.js error: 30 lines → 3 lines (92%) npm install: 150 lines → 3 lines Docker build: 50 lines → 1 line Security warnings? Kept. Error messages? Kept. Noise? Gone."

트윗 3: "102 real benchmarks. Best: 97% (Go errors) Worst: -12% (Java, output grew) We put the worst numbers in the README. github.com/jee599/contextzip"

트윗 4: "Install: npx contextzip Restart Claude Code. Done. Rust binary. <10ms. 1,056 tests. Also built AgentCrow — auto-dispatch 9 specialized agents for Claude Code Agent Teams: npx agentcrow init github.com/jee599/contextzip github.com/jee599/agentcrow ⭐"

## 3. Reddit r/ClaudeAI (22:10)

reddit.com/r/ClaudeAI/submit 접속.

Flair: Coding

Title: I built two open-source tools for Claude Code — ContextZip (compress CLI output 60-90%) and AgentCrow (auto-dispatch 9 specialized agents)

Body:
"I kept hitting Claude Code's context limit because of noise — npm install dumping 150 lines of deprecated warnings, Node.js stacktraces with 30 lines of node_modules frames, Docker builds with 50 lines of layer hashes.

So I built two tools:

**ContextZip** — compresses CLI output before it reaches Claude's context window.
- Node.js error: 30 lines → 3 lines (92% saved)
- npm install: 150 lines → 3 lines (security warnings preserved)
- Docker build: 50 lines → 1 line
- Install: npx contextzip

**AgentCrow** — auto-matches prompts to specialized agents for Claude Code Agent Teams.
- 9 builtin agents (frontend, backend, QA, devops, security, etc.)
- Auto-decomposes prompts into subtasks
- Install: npx agentcrow init

Both are open source, MIT license.

- ContextZip: https://github.com/jee599/contextzip (Rust, 1,056 tests, 102 benchmarks)
- AgentCrow: https://github.com/jee599/agentcrow (TypeScript, 78 tests)

Happy to answer questions!"

## 4. Reddit r/programming (22:15)

reddit.com/r/programming/submit 접속.

Title: ContextZip: Rust CLI that compresses terminal output by 61% for AI coding assistants (102 benchmarks, 1,056 tests)

Body:
"Built a Rust CLI proxy that compresses command output before it reaches AI coding assistants like Claude Code.

Real examples:
- Node.js stacktrace: 30 lines → 3 lines (hides node_modules, keeps user code)
- npm install: 150 lines → 3 lines (removes deprecated, keeps CVE warnings)
- Docker build: 50 lines → 1 line

102 real-world benchmarks, weighted average 61% savings. Some edge cases have negative savings (output grows) — those are in the README too.

Rust, single-threaded, <10ms overhead, 1,056 tests.

Install: npx contextzip

GitHub: https://github.com/jee599/contextzip"

## 5. LinkedIn 런칭 포스트 (22:20)

linkedin.com 접속. 포스트 올리기:

"🚀 오늘 두 개의 오픈소스 프로젝트를 공식 런칭합니다.

1) ContextZip — AI 코딩 어시스턴트의 CLI 출력을 60-90% 압축합니다. npm install 150줄 → 3줄. Docker build 50줄 → 1줄. 보안 경고는 보존.

2) AgentCrow — Claude Code Agent Teams를 위한 자동 에이전트 매칭. 프롬프트를 분석해서 9개 전문 에이전트 중 적합한 걸 자동 디스패치.

둘 다 npx 한 줄로 설치, MIT 라이선스.

ContextZip: https://github.com/jee599/contextzip (Rust, 1,056 tests)
AgentCrow: https://github.com/jee599/agentcrow (TypeScript, 78 tests)

1인 개발자로 AI를 활용해서 AI를 위한 도구를 만들었습니다. 피드백 환영합니다!

#OpenSource #AI #ClaudeCode #BuildInPublic #DeveloperTools"

## 완료 후

모든 플랫폼에서 올린 글의 댓글/리플라이가 오면 1시간 내로 답변해야 한다. 이건 수동으로 해야 할 수도 있음.
```

---

## 요약

| Task | 채널 | 시간 | 빈도 |
|------|------|------|------|
| 1 | X/Twitter | 매일 9시 | 트윗 1개 + 좋아요 3 + 리플라이 1 |
| 2 | Reddit | 매일 10시 | 좋아요 7 + 리플라이 1 (karma) |
| 3 | LinkedIn | 매일 11시 | 포스트 1개 + 좋아요 3 |
| 4 | HN | 매일 13시 | 댓글 1개 + upvote 2 (karma) |
| 5 | D-Day 런칭 | 3/24 22시 | HN+X+Reddit+LinkedIn 동시 (1회) |
