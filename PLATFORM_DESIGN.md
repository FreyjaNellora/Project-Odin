# Project Odin — Platform Design Document

**Status:** Draft — skeletal framework, not yet implemented
**Created:** 2026-02-21
**Purpose:** Capture platform architecture decisions for the web app that will host Odin (4PC) and Fairy-Stockfish (standard chess + variants).

---

## 1. VISION

A free chess platform with 4-player chess as a first-class game mode, powered by the world's first 4PC engine (Odin). Standard chess and variants powered by Fairy-Stockfish. AI-powered coaching that explains moves in plain language at the player's skill level.

**Core differentiators:**
1. Only platform with real 4PC engine analysis, puzzles, and coaching
2. Free by default — no paywalled core features (Lichess model)
3. Small-group social model — chess club feel, not social media
4. Evidence-based moderation — no weaponized reporting
5. LLM-powered teaching that adapts to player skill level
6. Anti-enshittification by design — public pledge, no investors who extract value
7. Open source engine and platform — community can audit, contribute, and fork
8. Design honesty — zero dark patterns, zero manipulative UI
9. Community theory boards — groups publish analysis, public votes, no comments. Chess theory built by players.

---

## 2. ARCHITECTURE

### 2.1 High-Level Stack

```
┌─────────────────────────────────────┐
│           Client (Browser/PWA)       │
│  React + WebSocket client            │
│  SVG Board renderer (from Stage 5)   │
│  LLM coaching interface              │
│  Group chat UI                       │
└──────────────┬──────────────────────┘
               │ WebSocket (games, chat)
               │ REST (accounts, history, analysis)
┌──────────────┴──────────────────────┐
│           Server (Rust — Axum)       │
│  ├── matchmaking/     Matchmaking    │
│  ├── rooms/           Game rooms     │
│  ├── accounts/        Auth + profile │
│  ├── social/          Groups + chat  │
│  ├── moderation/      Report pipeline│
│  └── engine/          Engine pool    │
└──────────────┬──────────────────────┘
               │
┌──────────────┴──────────────────────┐
│           Engine Workers             │
│  Odin (4PC analysis) — Rust library  │
│  Fairy-Stockfish (standard chess)    │
│  LLM API (coaching + moderation)     │
└──────────────┬──────────────────────┘
               │
┌──────────────┴──────────────────────┐
│           Data Layer                 │
│  PostgreSQL (accounts, games, groups)│
│  Redis (matchmaking, live state)     │
│  Object storage (replays, archives)  │
└─────────────────────────────────────┘
```

### 2.2 Technology Choices

| Layer | Technology | Rationale |
|---|---|---|
| Server | Rust (Axum) | Same language as Odin; Odin runs in-process as library; excellent WebSocket perf |
| Client | React + Vite | Reuses Stage 5 UI components; PWA-capable |
| Real-time | WebSocket | Game moves, chat, live updates |
| API | REST (JSON) | Account management, game history, analysis requests |
| Database | PostgreSQL | Relational data (accounts, games, groups, reports) |
| Cache | Redis | Matchmaking queues, game room state, session tokens |
| Auth | OAuth (Google/GitHub/Discord) | No passwords to manage; passkey support future |
| Engine (4PC) | Odin as Rust library | In-process calls, full Huginn telemetry access |
| Engine (std) | Fairy-Stockfish as subprocess | Unmodified, UCI protocol, proven strength |
| Coaching | LLM API (Claude/OpenAI) | Translates engine output to natural language |
| Hosting | Single VPS to start | $20-40/month; scale horizontally later |

### 2.3 Odin Integration

Odin must compile as both a standalone binary (current) and a library. The server imports `odin_engine` as a crate and calls search functions directly — no subprocess, no stdin/stdout overhead. The Odin Protocol becomes an internal API.

**Required engine changes (post-Stage 19):**
- Export public API from `lib.rs` (search, eval, game state)
- Thread-safe search with cancellation (for concurrent game rooms)
- Huginn trace extraction as structured data (for LLM coaching pipeline)

### 2.4 Dual-Engine Training Pipeline (Post-Launch)

Two parallel Odin instances with different training regimes. One stays clean, one learns from humans, and the clean one keeps the human one honest.

```
┌───────────────────────────┐     ┌───────────────────────────┐
│       Odin-Pure            │     │       Odin-Live            │
│   (Self-play only)         │     │   (Self-play + human games)│
│                            │     │                            │
│   • No human game data     │     │   • Opt-in rated games     │
│   • No meta bias           │     │   • Filtered (anti-cheat,  │
│   • Explores lines humans  │     │     rating threshold,      │
│     never try              │     │     min moves played)      │
│   • Clean room reference   │     │   • Powers the platform    │
│                            │     │   • Understands how humans │
│                            │     │     actually play          │
└─────────────┬─────────────┘     └─────────────┬─────────────┘
              │                                   │
              │   Disagreement positions           │
              │   (where evals diverge)            │
              ├──────────────────────────────────►│
              │                                   │
              │   Odin-Pure's eval serves as      │
              │   correction signal for Odin-Live │
              └───────────────────────────────────┘
```

#### Why Two Engines

| Problem | How Dual Training Solves It |
|---|---|
| **Human meta is stale/wrong** | Odin-Pure doesn't know the meta. Finds objectively strong lines regardless of human trends. Feeds corrections to Odin-Live. |
| **Cheater contamination** | Even if cheater games leak into Odin-Live's data, Odin-Pure is unaffected. Divergence between the two engines on the same position signals investigation. |
| **Groupthink / echo chamber** | Humans all play the same opening because "that's what everyone does." Odin-Pure explores everything equally and finds what humans overlook. |
| **Overfitting to rating band** | High-rated games may have systematic blind spots. Odin-Pure doesn't care about reputation or popularity. |
| **Regression detection** | New Odin-Live weights tested against Odin-Pure. If Live starts losing positions it used to win, the human data introduced a weakness. Roll back. |

#### Cross-Pollination Pipeline

```
Odin-Pure self-play generates training positions
    ↓
Compare: where does Odin-Pure's eval diverge from Odin-Live's eval?
    ↓
Disagreement positions are the interesting ones:
    ├── Pure says +300, Live says +50
    │   → Human games taught Live to undervalue this. Correct it.
    ├── Pure says -200, Live says +100
    │   → Human games taught Live something wrong. Flag and investigate.
    └── Both agree
        → Evaluation is stable. No action needed.
    ↓
Feed disagreement positions (with Odin-Pure's eval as reference)
into Odin-Live's next training cycle
    ↓
Odin-Live learns from humans AND gets corrected by the clean room
```

#### Human Game Collection (Ethical)

| Requirement | Implementation |
|---|---|
| **Explicit opt-in** | "Your games may be used to improve the engine." Toggle in account settings. Default: OFF. |
| **Opt-out removes data** | If a player opts out, their games are removed from the training dataset within 30 days. |
| **Transparency** | Public documentation of exactly what data is used, how it's processed, and what the training pipeline looks like. |
| **Cheater filtering** | Games from accounts later banned for cheating are flagged and excluded from training data. |
| **Rating threshold** | Only include games from players above a configurable rating threshold (e.g., top 20%). Low-rated games introduce noise. |
| **Minimum game quality** | Exclude games with disconnects, timeouts, fewer than 10 moves, or where a player resigned immediately. |
| **Recency weighting** | Recent games weighted more heavily. The meta evolves; training data should reflect the current state. |
| **No PII in training** | Only board positions and moves enter the pipeline. No player names, no account IDs, no metadata. |

#### What Each Engine Contributes

| Odin-Pure Provides | Odin-Live Provides |
|---|---|
| Novel opening theory nobody has played | Understanding of how humans actually play |
| Endgame technique in rare positions (3-player, 2-player after eliminations) | Awareness of common human mistakes to exploit |
| Refutations of popular-but-unsound strategies | Practical evaluation (objectively equal but practically hard positions) |
| Positions where engine deliberately plays against human intuition | Meta awareness (exploiting what everyone plays) |
| Clean regression baseline | The engine that powers actual gameplay |

#### Training Cadence

| Phase | Frequency | Trigger |
|---|---|---|
| **Odin-Pure self-play** | Continuous (background GPU job) | Always running, generating positions |
| **Odin-Live retraining** | Monthly or when sufficient new games accumulate | Minimum 10,000 new qualifying games since last training |
| **Cross-pollination** | Every Odin-Live training cycle | Disagreement positions from Pure injected into Live's training batch |
| **A/B validation** | Every training cycle | New weights vs. old weights in automated match (1000+ games). Ship only if measurably stronger. |
| **Weight versioning** | Every release | Old weights preserved. Instant rollback if regression detected post-deploy. |

#### Anti-Cheat Training Bonus

The dual-engine architecture has a powerful anti-cheat side effect: if a player's moves correlate strongly with Odin-Live but NOT with Odin-Pure, they may be using a cached/older version of the engine. If their moves correlate with Odin-Pure but not Odin-Live, they may have access to the self-play model somehow. The divergence between the two engines creates a fingerprint that makes engine-assisted cheating harder to disguise.

---

## 3. SOCIAL MODEL

### 3.1 Core Philosophy

**Intimate by default, scaled by choice.** Small persistent groups where players know each other. No global chat, no global forums. All social interaction happens in chosen communities.

### 3.2 Groups

| Property | Value | Rationale |
|---|---|---|
| Max group size | 20-30 | Forces intimacy, recognition, accountability |
| Max groups per user | 3-5 | Prevents social media scroll patterns |
| Group creation | Anyone can create | Low barrier to entry |
| Joining | Invite-only | Filters toxicity at the door |
| Features | Chat, shared analysis, mini-tournaments, study rooms | Chess club functionality |

### 3.3 Direct Messages

- One-on-one chat is unlimited and always available
- Post-game chat encouraged (natural interaction point)
- Same evidence-based reporting applies to DMs

### 3.4 Group Analysis Rooms

The killer social feature: groups can analyze games together with Odin running live. Members see the board, discuss moves, and the engine provides real-time analysis with LLM-translated explanations. This is something no platform offers.

### 3.5 Theory Boards (Public Voting)

Groups can publish their analysis to public **Theory Boards** — a curated, vote-driven knowledge base of chess theory built by the community.

#### How It Works

```
Group analyzes a position/strategy in their analysis room
    ↓
Group agrees on a theory submission
    ├── Position (FEN4 or PGN with key moves)
    ├── Recommended line(s)
    ├── Brief explanation (structured, max ~500 words)
    └── Category tag (opening, endgame, tactic, 4PC-specific, etc.)
    ↓
Group admin submits to Theory Board
    ↓
System auto-generates:
    ├── Engine eval score (Odin for 4PC, Fairy-Stockfish for standard)
    └── Star rating (how closely the group's line aligns with engine's top line)
    ↓
Public can browse and upvote
    ├── Anyone with an account can upvote
    ├── One upvote per account per post
    ├── Upvoting requires minimum 10 rated games played (prevents alt brigading)
    └── Upvote totals are public — everyone sees what's popular and what's overlooked
    ↓
NO COMMENTS on the board itself
    ├── Discussion happens in private groups
    ├── Discussion happens on external platforms (Reddit, Discord, etc.)
    └── The board is a library, not a forum
```

#### Three Independent Metrics

Every theory post displays three separate signals. None overrides the others. All are visible.

| Metric | Source | What It Measures | How It Works |
|---|---|---|---|
| **Star Rating** | Algorithmic (engine comparison) | How closely the group's recommended line aligns with the engine's top choices | Engine runs the position at tournament depth. Stars reflect move-by-move agreement: 5 stars = every move in the group's line is engine's #1 choice. 3 stars = mixed agreement. 1 star = line diverges significantly from engine preference. |
| **Upvotes** | Community voting | How useful/interesting players find the theory | Simple upvote count. One per account per post. Minimum 10 rated games to vote. No downvotes — low upvotes already signal disinterest. |
| **Engine Eval** | Raw engine output | What the engine thinks of the resulting position | Centipawn score (or win%) after the group's recommended line is played out. Shown as a number, not a judgment. |

**Why three metrics matter:**

| Scenario | Stars | Upvotes | Engine Eval | What It Means |
|---|---|---|---|---|
| Group found what engine already knows | High | High | Strong | Validated, popular, solid theory |
| Group found creative idea engine disagrees with | Low | High | Weak | Community sees value the engine misses — especially common in 4PC where eval is imperfect |
| Group found engine-approved line nobody cares about | High | Low | Strong | Technically correct but not practically useful or interesting |
| Group published weak theory | Low | Low | Weak | Naturally sinks to bottom — no intervention needed |

This separation prevents engine worship (where only engine-approved ideas get visibility) AND prevents popularity contests (where flashy but unsound ideas dominate). Both failure modes exist on other platforms. Three independent signals avoid both.

#### What Gets Published

| Field | Required | Description |
|---|---|---|
| **Title** | Yes | Short descriptive name ("4PC Queen-side pawn storm against passive Blue") |
| **Position** | Yes | FEN4 (4PC) or FEN/PGN (standard). Rendered as interactive board. |
| **Recommended line** | Yes | The moves the group recommends, with branches if applicable |
| **Explanation** | Yes | Structured text: what the idea is, when to use it, what to watch for. Max ~500 words. |
| **Category** | Yes | Opening / Middlegame / Endgame / Tactic / Strategy / 4PC-Specific |
| **Subcategory (4PC)** | Optional | Elimination timing / Alliance theory / Point farming / Multi-front defense / King safety |
| **Rating range** | Optional | "Most useful at 800-1200" — helps players filter by relevance |
| **Group name** | Yes | Attribution. Gives groups identity and pride. |
| **Star rating** | Auto-generated | Algorithmic: human line vs engine line agreement (1-5 stars) |
| **Upvotes** | Community | Running total of user upvotes |
| **Engine eval** | Auto-generated | Raw centipawn/win% score from Odin or Fairy-Stockfish |

#### Sorting & Filtering

Users control what they see. No algorithm decides for them.

| Sort by | Use case |
|---|---|
| **Upvotes (most)** | "What does the community value?" |
| **Upvotes (least)** | "What's been overlooked?" — hidden gems |
| **Star rating (high)** | "What does the engine validate?" |
| **Star rating (low)** | "Where does human intuition diverge from engine?" — frontier theory |
| **Engine eval (strongest)** | "What leads to the best positions?" |
| **Newest** | "What's fresh?" |
| **Category** | Filter by opening / endgame / tactic / 4PC-specific |
| **Rating range** | "Show me theory relevant to my skill level" |

#### Design Principles

1. **Read-only public space.** The board is a library, not a forum. Browse, vote, learn. No comments, no replies, no threads. This is how you get the value of crowdsourced knowledge without the toxicity of public discussion.

2. **Groups as research cells.** The theory board gives groups a purpose beyond social chat. Groups become mini-research teams. Published theories build group reputation.

3. **Three signals, no hierarchy.** Stars, upvotes, and engine eval are shown equally. The platform never labels a post as "good" or "bad" — users interpret the signals themselves. A low-star, high-upvote post is just as valid to display as a high-star, high-upvote one.

4. **No alt-account manipulation.** One upvote per account per post. Minimum 10 rated games to vote (prevents throwaway accounts). Groups cannot vote on their own posts. Coordinated vote manipulation (detected by pattern analysis) results in theory board privileges revoked for the group.

5. **Moderation.** Theory posts are reviewed for spam/off-topic before appearing publicly. LLM triage for obvious abuse; human review if flagged. Groups that consistently publish quality content get a "verified contributor" badge (visible on their posts).

#### Why This Works

- **Everyone benefits.** Solo players and lurkers get free access to community-developed theory. They don't need to join a group to learn.
- **No toxicity vector.** The #1 reason chess forums are toxic is comment sections. Remove comments, remove toxicity. People who want to argue about theory can do it in their private groups or on Reddit.
- **4PC theory doesn't exist yet.** This platform will be where 4PC theory is *invented*. The theory board becomes the canonical reference — like an evolving, community-written book on 4-player chess.
- **Organic discovery.** New users browsing the theory board see what the community values. Overlooked posts get a second chance as new users discover them. Popular posts validate community consensus.

### 3.6 No Global Social Spaces

No public forums, no global chat rooms, no comment sections. The theory board is the only public-facing content, and it's read-only with voting — no discussion. There is no stage for trolls to perform on. If players want to discuss chess publicly, they have Reddit/Discord/Twitter. The platform is for playing and learning, not broadcasting.

---

## 4. MODERATION SYSTEM

### 4.1 Core Principles

1. **Evidence required** — reporters must attach specific text/screenshots
2. **Both sides heard** — accused can submit defense with context
3. **Mute over ban** — restrict chat, never restrict chess
4. **Humans decide** — LLMs triage, humans make final calls
5. **Transparency** — players see what they're accused of

### 4.2 Report Pipeline

```
Incident occurs
    ↓
Reporter submits ticket
    ├── Must select specific text (copy/paste)
    ├── Must provide justification
    └── Empty/vague reports = low priority + reporter credibility hit
    ↓
LLM triage
    ├── Severe (slurs, threats): Temp mute accused, escalate to human
    ├── Moderate (harassment): Flag for review, no immediate action
    └── Weak evidence: Low priority queue
    ↓
Accused notified
    ├── Sees the specific evidence
    └── Submits defense (screenshot/copy-paste of broader context)
    ↓
Human reviewer
    ├── Sees both sides, LLM assessment, account histories
    └── Makes decision: Warning / Mute / Escalate / Dismiss
    ↓
Outcome applied
```

### 4.3 Status Effects

| Level | Trigger | Effect | Duration | Resolution |
|---|---|---|---|---|
| **Clean** | Default | Full access | Permanent | — |
| **Under review** | Report filed | No change yet | Until reviewed | LLM triage |
| **Temp mute** | LLM flags severe | Can play, can't chat with strangers | Until human reviews | Human decision |
| **Extended mute** | Human confirms | Can play, can't chat with strangers | 24h / 7d / 30d | Automatic expiry |
| **Probation** | Repeated violations | Can play, limited social | 90 days | Clean record resets |
| **Report-muted** | Spam reporting | Can play, can't file reports | Until human reviews | Human decision |

### 4.4 Group Admin Override

- Group admins can allow muted players to chat within their groups
- Opting in elevates monitoring on that group slightly
- Override can be revoked based on violation severity
- Certain offenses (threats, hate speech) override group admin decisions

### 4.5 Spam Report Deterrent

- Filing reports with no/weak evidence degrades reporter credibility score
- Consistently false reporters get report-muted (same mechanic as chat mute)
- Coordinated mass reports from multiple accounts are detected and flagged
- No amount of volume without evidence can trigger automated action

### 4.6 Permaban Process (Critical/Severe Only)

1. **First reviewer** — examines evidence, both sides, history. Makes recommendation.
2. **Second reviewer** — independent review, doesn't see first recommendation.
3. **Agreement** — both agree = final. Disagree = escalation to senior reviewer.
4. **Written justification** — banned player receives specific evidence and reasoning.
5. **Final appeal** — 30-day window, reviewed by uninvolved reviewer.

---

## 5. COACHING & TEACHING

### 5.1 Engine-to-LLM Pipeline

```
Engine search completes
    ↓
For 4PC: Odin Huginn trace (full search reasoning)
For std chess: Fairy-Stockfish output (eval + PV line)
    ↓
Post-game processor extracts key moments
    ├── Blunders (eval swing > threshold)
    ├── Missed tactics (engine found better move)
    ├── Turning points (eval crossed zero)
    └── Brilliant moves (player found engine's top choice in complex position)
    ↓
LLM receives: structured data + player rating + game context
    ↓
LLM generates skill-appropriate explanation
```

### 5.2 Skill-Level Adaptation

| Rating Range | Language Style |
|---|---|
| Beginner (400-800) | Plain language, no notation, highlight squares visually. "Your queen can be taken here." |
| Improving (800-1200) | Introduce concepts by name with definitions. "This is called a fork — one piece attacks two." |
| Intermediate (1200-1600) | Standard notation, assume tactical vocabulary. "Qxg8 allows Nf5+ forking K and Q." |
| Advanced (1600+) | Raw engine data with strategic context. "Qxg8 drops from +750 to +120 at depth 7." |

### 5.3 4PC-Specific Coaching (Unique to Odin)

Concepts that no platform teaches:
- Multi-opponent threat awareness ("Blue captured your pawn, but Yellow was the real threat")
- Point-lead strategy ("You're ahead — the engine suggests defense because you're a target")
- Elimination timing ("Keeping a weak opponent alive denies farming material to others")
- Alliance dynamics ("Green and Blue both benefit from attacking you, even without coordinating")

### 5.4 Standard Chess Coaching

Fairy-Stockfish provides eval + PV. LLM draws on general chess pedagogy to explain. Less deep than 4PC coaching (no Huginn trace) but still better than chess.com's "Blunder ❌" labels.

---

## 6. MONETIZATION & PLATFORM PHILOSOPHY

### 6.1 Core Principles (Immutable)

These are not guidelines — they are the identity of the platform. Every decision filters through them.

1. **No advertisements. Ever.** No banner ads, no sponsored content, no "promoted" anything. Ads degrade user experience, create perverse incentives (optimize for engagement over quality), and signal that users are the product. We reject this entirely.

2. **Community built and driven.** The platform exists to serve players, not extract value from them. User feedback guides all future development. Features are prioritized by what the community needs, not what generates revenue.

3. **Engine health, website health, user experience — in that order.** The anchoring goals. If the engine is broken, nothing else matters. If the site is down, nothing else matters. If users are frustrated, cosmetics don't matter. This hierarchy is absolute.

4. **Critical issues are always top priority.** Bugs, security vulnerabilities, performance regressions, moderation failures — these take precedence over everything. No feature work proceeds while critical issues are open.

5. **Cosmetics are dead last.** Board themes, badges, profile decorations — only pursued if the team has the time, energy, and long-term resources to support them without disrupting or taking away from the core focus. If cosmetics ever compete with infrastructure for resources, cosmetics lose. Every time.

### 6.2 Revenue Model

| Source | Description |
|---|---|
| **Free tier** | All core features: games, analysis, engine, groups, coaching. No exceptions. |
| **Optional supporter subscription** | Small monthly amount, visible badge, transparent about server costs. Supporters fund infrastructure — they don't buy advantages. |
| **Funding dashboard** | "Server costs this month: $X. Funded: $Y." Full transparency. Users see exactly where money goes. |
| **Opt-out** | Subscribers can cancel instantly. No dark patterns, no annual auto-renewal tricks, no guilt trips, no "are you sure?" dialogs. One click, done. |
| **Optional cosmetics** | Board themes, piece sets, profile badges. No gameplay advantage. Only developed if resources allow (see Principle 5). |

### 6.3 What We Will Never Do

- Sell user data
- Show advertisements of any kind
- Paywall core features (analysis, coaching, engine access, groups)
- Create "premium-only" game modes or time controls
- Use dark patterns to retain subscribers
- Optimize for engagement metrics over user wellbeing
- Let revenue concerns override user experience decisions
- Partner with entities whose values conflict with these principles

### 6.4 Anti-Enshittification Pledge

Enshittification is the documented lifecycle where platforms start great for users, then shift value to advertisers/investors, then extract maximum value from everyone until the platform dies. It killed MySpace, degraded Facebook, Twitter, Reddit, and is actively eroding chess.com. We refuse this trajectory.

**Commitments:**
1. **No investor-driven value extraction.** If the platform takes funding, it's from users (supporters), not from entities who want to monetize the user base. No venture capital that demands growth-at-all-costs.
2. **The free tier never gets worse.** Features available for free today remain free forever. We never cripple the free experience to push upgrades.
3. **No engagement optimization.** We don't use addictive design patterns (streaks, FOMO notifications, gamified progression that punishes absence). Users play chess when they want to. The platform doesn't guilt them into coming back.
4. **Public audit trail.** Major decisions affecting users are documented publicly with reasoning. Users know why things change.

### 6.5 Design Honesty (Anti-Dark-Patterns)

Every UI interaction is honest. The platform never tricks, pressures, or confuses users into actions they didn't intend.

| Dark Pattern | Our Commitment |
|---|---|
| **Difficult cancellation** | Unsubscribe is one click, same page as subscribe. No confirmation loops, no "are you sure?", no "talk to an agent." |
| **Manipulative defaults** | All defaults favor user privacy and minimal data collection. Users opt IN to sharing, never opt out. |
| **Fake urgency** | No countdown timers, no "X people are looking at this," no "limited time offer." |
| **Confirm-shaming** | No guilt-trip language. "Cancel subscription" — not "No, I don't want to support chess." |
| **Hidden costs** | All costs visible before any commitment. No surprise charges, no "processing fees." |
| **Forced continuity** | Subscriptions show renewal date prominently. Reminder sent 7 days before renewal. Easy to pause or cancel. |
| **Visual manipulation** | Accept and decline buttons are the same size, same weight, same visual prominence. |

### 6.6 Feature Discipline

The platform does three things: **play chess, learn chess, share chess with friends.** Everything serves one of these. Nothing else gets built.

1. **Core before new.** Existing features improve before new features ship. Bug fixes and performance beat feature launches.
2. **No trend-chasing.** No NFTs, no blockchain, no cryptocurrency, no metaverse, no AI-generated content for its own sake. Technology serves the three goals or it doesn't get used.
3. **Removable additions.** If a feature is added and the community doesn't use it or doesn't like it, it gets removed. No sunk cost attachment to shipped features.
4. **Scope boundary.** The platform is not a social media site, not a marketplace, not a content platform, not a streaming service. Staying small and focused is a feature.

### 6.7 User Feedback Loop

Users aren't just heard — they're shown they were heard.

1. **Public roadmap.** What's being worked on, what's planned, what's been requested. Influenced by community votes.
2. **"State of the platform" updates.** Regular (monthly or quarterly) honest updates: what went well, what broke, what's next, what we decided against and why.
3. **Acknowledged feedback.** Every piece of feedback gets at least "We saw this." Even when the answer is no, users get a reason.
4. **No silent changes.** Features are never removed, repriced, or altered without notice and explanation. Changelog is public and written in plain language.

### 6.8 Privacy Ratchet

Data collection can only **decrease** over time, never increase.

1. If we collect a piece of data, we explain why in plain language.
2. Users can always see exactly what we have on them (full data export).
3. Users can always delete their data (right to erasure).
4. Analytics serve the platform, never advertisers. We measure "is the site fast?" not "how do we keep users scrolling?"
5. If we realize we're collecting something we don't need, we stop collecting it and delete existing data.

### 6.9 Open Source Commitment

Open source builds trust. Users can verify claims. Community can contribute. Lichess proves this model sustains a world-class chess platform.

1. **Engine is open source.** Odin's code is public. Anyone can audit it, learn from it, contribute to it.
2. **Platform core is open source where practical.** Game logic, analysis pipeline, and protocol are public. Deployment configs and security-sensitive infrastructure are not.
3. **Community contributions welcome.** Clear contribution guidelines, code of conduct, and maintainer responsiveness.
4. **Forkable by design.** If we ever fail these principles, the community can take the code and build something better. This is a feature, not a risk.

---

## 7. BUILD PHASES

### Phase 1: Play Chess Online
1. Rust server (Axum) with WebSocket game rooms
2. OAuth account system (Google/GitHub/Discord)
3. Matchmaking (random opponent, rated games)
4. Web client with SVG board renderer
5. Game clock + result recording
6. Deploy on single VPS

### Phase 2: Engine Analysis
7. Odin workers for 4PC analysis
8. Fairy-Stockfish workers for standard chess
9. Post-game review UI
10. LLM coaching integration

### Phase 3: Social
11. Groups (small-community model)
12. One-on-one chat
13. Evidence-based moderation system
14. Group analysis rooms

### Phase 4: Community Knowledge
15. Theory Board (public voting, group publishing)
16. Puzzles generated from engine analysis

### Phase 5: Growth
17. Rating system + leaderboards
18. Tournaments
19. PWA mobile optimization

**Prerequisite:** Odin engine complete (Stages 8-19).

---

## 8. DIRECTORY STRUCTURE (Planned)

```
odin-platform/
├── Cargo.toml              # Workspace: server + shared types
├── server/
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs          # Axum server entry
│       ├── config.rs        # Environment config
│       ├── matchmaking/     # Queue, pairing, rating
│       ├── rooms/           # Game room lifecycle
│       ├── accounts/        # Auth, profiles, OAuth
│       ├── social/          # Groups, DMs, invites
│       ├── moderation/      # Reports, triage, review queue
│       ├── engine/          # Odin + Fairy-Stockfish pool
│       ├── coaching/        # LLM pipeline
│       └── db/              # PostgreSQL queries, migrations
├── shared/
│   ├── Cargo.toml
│   └── src/
│       ├── protocol.rs      # Shared message types (WS + REST)
│       ├── game_types.rs    # Game mode enum, time controls
│       └── rating.rs        # Glicko-2 implementation
├── client/
│   ├── package.json
│   ├── vite.config.ts
│   └── src/
│       ├── App.tsx
│       ├── pages/           # Home, Play, Analysis, Groups, Profile
│       ├── components/      # Board, Chat, GameControls (reuse from Stage 5)
│       ├── hooks/           # useWebSocket, useGame, useEngine, useCoaching
│       ├── lib/             # Protocol parser, board constants (reuse from Stage 5)
│       └── styles/
├── migrations/              # PostgreSQL schema migrations
└── docker-compose.yml       # Local dev: Postgres + Redis
```

---

## 9. DATABASE SCHEMA (Draft)

```sql
-- Core
accounts (id, display_name, oauth_provider, oauth_id, rating_4pc, rating_std,
          sportsmanship_score, created_at)

-- Games
games (id, mode, variant, time_control, started_at, ended_at, result,
       pgn_or_moves, fen4_final)
game_players (game_id, player_slot, account_id, rating_before, rating_after,
              points_scored)

-- Social
groups (id, name, creator_id, max_members, created_at)
group_members (group_id, account_id, role, joined_at)  -- role: admin/member
messages (id, sender_id, context_type, context_id, content, created_at)
    -- context_type: 'dm' | 'group' | 'game'
    -- context_id: recipient_id | group_id | game_id

-- Moderation
reports (id, reporter_id, accused_id, evidence_text, justification,
         status, llm_severity, created_at)
defenses (id, report_id, accused_id, defense_text, created_at)
reviews (id, report_id, reviewer_id, decision, reasoning, created_at)
account_status (account_id, status, reason, expires_at, applied_by)
reporter_credibility (account_id, total_reports, accurate_reports, score)

-- Engine / Coaching
analysis_jobs (id, game_id, engine, status, result_json, created_at)
coaching_reviews (id, game_id, account_id, key_moments_json,
                  explanations_json, player_rating_at_time)

-- Theory Board
theory_posts (id, group_id, author_account_id, title, category, subcategory,
              position_data, recommended_line, explanation, rating_range_low,
              rating_range_high, game_mode, status, created_at, published_at,
              -- Three independent metrics:
              star_rating, -- 1-5, algorithmic: human line vs engine top line agreement
              upvote_count, -- running total of community upvotes
              engine_eval_cp, -- centipawn score from engine analysis
              engine_eval_json) -- full engine analysis output (depth, PV, etc.)
    -- status: 'draft' | 'pending_review' | 'published' | 'removed'
    -- game_mode: 'standard' | '4pc'
    -- star_rating computed on publish: engine runs position, compares group's
    --   recommended line move-by-move against engine's top choices
theory_upvotes (theory_post_id, account_id, created_at)
    -- upvote only (no downvotes — absence of upvote is signal enough)
    -- unique constraint on (theory_post_id, account_id)
    -- account must have >= 10 rated games to vote
group_theory_stats (group_id, total_posts, total_upvotes,
                    avg_star_rating, verified_contributor, verified_at)
```

---

## 10. SECURITY & ANTI-ABUSE

### 10.1 Account Security

| Measure | Description |
|---|---|
| **OAuth-only login** | No passwords stored. Google/GitHub/Discord handle auth. Eliminates password breach risk entirely. |
| **Passkey support (future)** | WebAuthn/FIDO2 passkeys as primary auth. Phishing-proof. |
| **Session management** | JWT with short expiry (15 min) + refresh tokens. Revocable per-device. |
| **2FA on sensitive actions** | Changing email, deleting account, transferring group ownership require re-auth via OAuth provider. |
| **Login anomaly detection** | Flag logins from new devices/locations. Notify user via email. Optional "trusted devices" list. |
| **Account recovery** | Through OAuth provider only. No security questions, no SMS (SIM-swap vulnerable). |
| **Rate limiting** | Per-IP and per-account rate limits on all API endpoints. Exponential backoff on failed auth. |

### 10.2 Ban Evasion & VPN Policy

**The problem:** Permabanned users create new accounts via VPN/proxy to evade bans. Chess.com struggles with this constantly.

**Layered approach:**

| Layer | What it catches | How |
|---|---|---|
| **Device fingerprinting** | Same browser/device, new account | Canvas fingerprint, WebGL renderer, screen resolution, timezone, installed fonts. Not foolproof but catches casual evasion. |
| **Behavioral fingerprinting** | Same player, new identity | Play style analysis — opening preferences, time-per-move patterns, mouse movement patterns, blunder rate at specific positions. Hard to fake. |
| **VPN/proxy detection** | Known VPN/datacenter IPs | IP reputation databases (IPQualityScore, MaxMind). Don't block VPNs outright — flag accounts created from VPN IPs for elevated monitoring. |
| **Phone verification (optional)** | Mass account creation | Required only after a ban or when behavioral flags trigger. Not required for normal signup (keeps barrier low). |
| **OAuth provider limits** | One account per OAuth identity | Google/GitHub/Discord accounts are harder to mass-create than email accounts. |

**Policy:**
- VPNs are NOT banned for normal users (privacy is valid)
- New accounts from VPN IPs get a "new + VPN" flag — slightly elevated monitoring for first 10 games
- If behavioral fingerprinting matches a banned account with >80% confidence, flag for human review
- Confirmed ban evaders get the new account banned + the detection added to their fingerprint profile
- Never auto-ban based on VPN alone — too many false positives (travelers, privacy-conscious users)

### 10.3 Anti-Cheating

| Layer | Method | Scope |
|---|---|---|
| **Statistical detection** | Compare player's move accuracy against their rating band. Flag games where accuracy exceeds 95th percentile for their rating. | All rated games |
| **Engine correlation** | Compare player moves against Odin (4PC) or Fairy-Stockfish (std) top-3 engine moves. High correlation over many games = suspicious. | Flagged games |
| **Behavioral signals** | Tab-switching frequency, consistent move timing (bots move in fixed intervals), copy-paste detection in browser | Real-time monitoring |
| **Odin advantage** | For 4PC, we control the ONLY engine. If someone's moves correlate with Odin's output, we know they're using our engine to cheat. No other 4PC engine exists to correlate against. | 4PC games only |
| **Crowd-sourced review** | High-rated players can volunteer to review flagged games. Multiple reviewers per case. Compensated with cosmetics/badges. | After statistical flag |
| **Transparent process** | Accused player sees: which games were flagged, what the statistical analysis found, and can submit defense (same evidence-based system as chat moderation). | All cheat accusations |

**Anti-cheat philosophy:**
- Statistical flags are NOT bans. They're the start of investigation.
- Players can see why they were flagged and defend themselves.
- False positive rate matters more than catch rate. Better to miss a cheater than ban a legitimate player.
- Cheating bans follow the same layered human review as permabans (Section 4.6).

### 10.4 Anti-Scraping & API Protection

| Threat | Countermeasure |
|---|---|
| **Mass game scraping** | Rate-limited public API. Authenticated requests get higher limits. Bulk data exports available on request (be open, not hostile). |
| **Bot account creation** | OAuth-only eliminates email spam signups. CAPTCHA on account creation if abuse detected (not by default — CAPTCHAs are hostile UX). |
| **API abuse** | Token-based auth with scoped permissions. Read-only public endpoints, write endpoints require auth. |
| **DDoS** | Cloudflare or equivalent CDN/WAF in front of the server. WebSocket connections authenticated before upgrade. |
| **XSS / injection** | React handles DOM escaping by default. Server-side input validation on all endpoints. CSP headers. No inline scripts. |
| **CSRF** | SameSite cookies + CSRF tokens on state-changing requests. |
| **Data exfiltration** | PII (email, OAuth tokens) never exposed via API. Display names are public, everything else is private by default. |

**Philosophy on scraping:** Be open with game data (researchers, tool builders want access), but protect user PII and prevent abuse. Provide an official API with reasonable limits rather than forcing scrapers to circumvent protections. This is the Lichess approach and it works.

### 10.5 Data Privacy

| Data | Visibility | Retention |
|---|---|---|
| Games played | Public (by default, player can make private) | Permanent |
| Rating history | Public | Permanent |
| Display name | Public | Until changed |
| OAuth email | Never exposed, server-side only | Until account deletion |
| Chat messages | Visible to participants only | 90 days (group), 30 days (DM) |
| Moderation reports | Visible to involved parties + reviewers only | 1 year after resolution |
| Engine analysis | Visible to requesting player only | 30 days (auto-delete) |
| IP addresses | Never stored long-term | Session duration only (logged for rate limiting) |
| Device fingerprints | Never stored raw | Hashed, used for ban-evasion detection only |

**GDPR compliance:** Full data export on request. Full deletion on request (anonymize game records, delete everything else). Cookie consent for analytics only (core functionality requires zero tracking cookies).

---

## 11. LEGAL & POLICY REQUIREMENTS

### 11.1 Required Legal Documents

Every public-facing platform needs these. Non-negotiable for launch.

| Document | Purpose | Status |
|---|---|---|
| **Terms of Service (ToS)** | Legal contract between platform and user. Defines acceptable use, liability limits, account termination rules, dispute resolution. | Not drafted |
| **Privacy Policy** | What data we collect, why, how it's stored, who sees it, how users can access/delete it. Legally required if we collect any personal data. | Section 10.5 has the framework; needs formal legal language |
| **Cookie Policy** | What cookies we use, why, how to opt out. Required by EU ePrivacy Directive and GDPR. Separate from Privacy Policy or a subsection. | Not drafted |
| **Community Guidelines** | What behavior is expected, what gets you muted/banned, how moderation works. Human-readable version of the moderation system. | Section 4 has the framework; needs user-facing version |
| **DMCA Policy** | How copyright holders report infringement on user-generated content (chat, shared analysis). Requires designated DMCA agent registered with Copyright Office. | Not drafted |
| **Acceptable Use Policy (AUP)** | Technical limits: no bots without permission, no scraping, no exploiting vulnerabilities, no impersonation. Can be part of ToS or standalone. | Not drafted |
| **Responsible Disclosure Policy** | How security researchers report vulnerabilities. Safe harbor language so they don't get sued for finding bugs. | Not drafted |

### 11.2 Children's Privacy & Safety (Critical)

Chess attracts young players. This isn't optional — it's a legal minefield with billion-dollar fines.

#### COPPA (U.S. — Children's Online Privacy Protection Act)

Applies to children under 13. FTC finalized major amendments effective June 2025, with full compliance required by April 2026.

| Requirement | Our Approach |
|---|---|
| **Age gate** | During OAuth signup, ask date of birth. If under 13, block account creation. We don't collect the DOB — just check age threshold and discard. |
| **No data collection from under-13s** | If a child somehow creates an account, we collect nothing beyond what OAuth provides. No behavioral tracking, no fingerprinting, no analytics. |
| **Parental consent for under-13** | We choose NOT to implement parental consent flows (they're complex and create liability). Instead: under-13 cannot create accounts. Simple, safe. |
| **Data minimization** | Already our philosophy (Section 6.8). Collect only what's needed, delete when it's not. |
| **COPPA-compliant privacy notice** | Must be clear, prominent, and written for parents to understand. Separate from adult privacy policy. |

**Decision:** Minimum age to create an account is **13**. This is the Lichess approach and avoids the entire COPPA consent machinery. Users who lie about their age are in violation of ToS.

#### KOSA (U.S. — Kids Online Safety Act, 2025)

Applies to platforms with users under 17. Even with a 13+ age gate, we'll have minors.

| Requirement | Our Approach |
|---|---|
| **Safeguards for minors** | Our small-group social model already limits stranger interaction. Chat muting (Section 4.3) already exists. |
| **Limit compulsive usage features** | We already reject engagement optimization (Section 6.4). No streaks, no FOMO, no addictive mechanics. |
| **Parental tools** | If a user identifies as under 17, provide parental controls: chat restrictions, playtime visibility, ability for parent to link to child's account. |
| **Default privacy for minors** | Accounts identified as under 17 default to maximum privacy: games private, no DMs from strangers, group join requires parental approval. |

#### UK Online Safety Act (2023, enforced 2025+)

If accessible from the UK (we're a website, so yes), platforms have a duty of care to protect children from harmful content.

| Requirement | Our Approach |
|---|---|
| **Age assurance** | Age declared at signup. No invasive verification (no ID scanning). |
| **Risk assessment** | Document what risks exist on the platform for children (exposure to harmful chat, predatory behavior). Our limited social model is inherently lower risk. |
| **Content moderation** | Already comprehensive (Section 4). LLM triage + human review. |
| **Transparency report** | Annual report on moderation actions, types of content removed, number of reports. We already planned public audit trails (Section 6.4). |

#### GDPR Article 8 (EU — Children's Data)

Children under 16 (or 13-16 depending on member state) need parental consent for data processing.

| Requirement | Our Approach |
|---|---|
| **Age threshold** | 13 minimum to create account (aligns with COPPA). EU users under 16 get enhanced privacy defaults. |
| **Consent for under-16** | OAuth providers handle authentication. We don't process children's data beyond gameplay. Parental consent flow for EU under-16 users if they want social features. |
| **Right to erasure** | All users, including minors, can delete all data at any time. Already in Section 10.5. |

### 11.3 GDPR Compliance (Detailed)

Applies to any user from the EU, regardless of where the platform is hosted.

| GDPR Principle | Implementation |
|---|---|
| **Lawful basis** | Consent (explicit opt-in) for analytics and non-essential cookies. Legitimate interest for essential platform operation. Contract for account features. |
| **Data minimization** | Collect only what's needed. Already core philosophy (Section 6.8). |
| **Purpose limitation** | Data collected for chess is used for chess. Never sold, never repurposed. |
| **Storage limitation** | Retention periods defined in Section 10.5. Auto-delete when expired. |
| **Right of access** | Users can export all their data (games, chat history, analysis, profile) in machine-readable format. One-click in account settings. |
| **Right to erasure** | Full account deletion: anonymize game records (replace name with "deleted_user_XXXX"), delete everything else. One-click, no dark patterns. |
| **Right to rectification** | Users can edit their display name, linked accounts, and preferences at any time. |
| **Data breach notification** | Notify affected users within 72 hours. Notify supervisory authority within 72 hours. See Section 11.5 (Incident Response). |
| **Data Protection Officer** | Required if processing data at scale. Designate a DPO contact (can be the founder initially). |
| **Privacy by design** | Every new feature goes through a privacy impact assessment before launch. Default settings always favor privacy. |
| **Cookie consent** | Explicit opt-in for non-essential cookies. Essential cookies (session, authentication) don't require consent. No dark-pattern cookie banners. Reject button same size as accept. |

### 11.4 Accessibility Requirements

People with disabilities play chess. The platform must be usable by everyone.

#### WCAG 2.2 Level AA (Target Standard)

| Category | Requirements |
|---|---|
| **Perceivable** | Alt text for all images, proper heading hierarchy, sufficient color contrast (4.5:1 for normal text, 3:1 for large), captions for any video content. |
| **Operable** | Full keyboard navigation (critical for chess — mouse isn't the only input). No time limits that can't be extended (game clocks are exempt — they're part of chess). Focus indicators visible on all interactive elements. |
| **Understandable** | Consistent navigation, predictable UI behavior, error identification and suggestions, plain language in instructions. |
| **Robust** | Semantic HTML, ARIA labels where needed, works with screen readers (NVDA, JAWS, VoiceOver). Board state must be accessible to screen readers. |

#### Chess-Specific Accessibility

| Feature | Implementation |
|---|---|
| **Board narration** | Screen reader announces piece positions, legal moves, and game state changes. "White pawn on e4. Your turn." |
| **Keyboard move input** | Type moves in algebraic notation (e.g., "e2e4") as alternative to clicking. |
| **High-contrast mode** | Board and pieces with maximum contrast. No reliance on color alone to distinguish pieces (shape differences sufficient). |
| **Colorblind support** | Player colors (Red/Blue/Yellow/Green in 4PC) distinguished by patterns/shapes, not just color. |
| **Reduced motion** | Option to disable animations (piece sliding, highlighting). Instant moves for users who need it. |
| **Font scaling** | All text respects browser zoom and OS font size settings. Nothing breaks at 200% zoom. |

### 11.5 Incident Response Plan

When (not if) something goes wrong.

#### Data Breach Protocol

```
Breach detected or reported
    ↓
1. CONTAIN (immediate)
    ├── Isolate affected systems
    ├── Revoke compromised credentials/tokens
    └── Assess scope: what data, how many users
    ↓
2. ASSESS (within 24 hours)
    ├── What happened? Root cause.
    ├── What data was exposed?
    ├── How many users affected?
    └── Is it ongoing?
    ↓
3. NOTIFY (within 72 hours — GDPR requirement)
    ├── Affected users: what happened, what data, what to do
    ├── Supervisory authority (if EU users affected)
    ├── Public disclosure if widespread
    └── No downplaying. Honest, specific, actionable.
    ↓
4. REMEDIATE
    ├── Fix the vulnerability
    ├── Audit for similar issues
    ├── Update security measures
    └── Document lessons learned (public post-mortem)
    ↓
5. POST-MORTEM (within 30 days)
    ├── Public write-up: what happened, what we did, what changed
    ├── Timeline of events
    └── Preventive measures implemented
```

#### Security Vulnerability Disclosure

```
Researcher finds vulnerability
    ↓
Reports via security@[domain] or dedicated form
    ↓
We acknowledge within 48 hours
    ↓
We assess severity and fix (target: 7 days critical, 30 days others)
    ↓
Researcher credited publicly (if they want)
    ↓
Vulnerability disclosed after fix is deployed
```

**Safe harbor:** Security researchers acting in good faith will not face legal action. We explicitly state this in our Responsible Disclosure Policy.

### 11.6 Content Policy & Community Guidelines (User-Facing)

The formal, human-readable version of Section 4 (Moderation System). This is what users actually see.

**Structure:**

```
Community Guidelines
├── What we expect
│   ├── Be respectful
│   ├── Play fair
│   ├── Report honestly (with evidence)
│   └── Respect privacy
├── What's not allowed
│   ├── Hate speech, slurs, threats (zero tolerance)
│   ├── Harassment, stalking, doxxing
│   ├── Cheating (engine use in rated games)
│   ├── Multi-accounting (one account per person)
│   ├── Spam, advertising, promotion
│   ├── Impersonation
│   └── Sharing others' personal information
├── What happens if you break the rules
│   ├── Mute (temporary, chat only — you can always play)
│   ├── Extended mute (repeat violations)
│   ├── Probation (persistent issues)
│   └── Permaban (extreme cases, human-reviewed, appealable)
├── How reporting works
│   ├── You must provide evidence (copy/paste or screenshot)
│   ├── Accused can defend themselves
│   ├── Both sides are heard before action
│   └── False/spam reports hurt your credibility, not theirs
└── Your rights
    ├── See what you're accused of
    ├── Submit a defense
    ├── Appeal any decision
    └── Know who reviewed your case (role, not name)
```

### 11.7 International Compliance Summary

| Jurisdiction | Key Laws | Our Obligations |
|---|---|---|
| **United States** | COPPA, KOSA, CAN-SPAM, DMCA, state privacy laws (CCPA/CPRA, etc.) | 13+ age gate, DMCA agent, email opt-out, California user rights |
| **European Union** | GDPR, ePrivacy Directive, Digital Services Act (DSA) | Consent-based processing, data export/erasure, cookie consent, transparency reports, DMCA-equivalent notice-and-action |
| **United Kingdom** | UK GDPR, Online Safety Act | Duty of care for children, risk assessments, transparency reports |
| **Canada** | PIPEDA | Consent for data collection, access/correction rights |
| **Australia** | Privacy Act, Online Safety Act | Reasonable steps to protect data, age-appropriate design |
| **Global** | WCAG 2.2, OWASP ASVS | Accessibility standards, security baseline |

**Approach:** Follow GDPR as the baseline (strictest mainstream regulation). Everything else is either a subset or has specific additions. Build for GDPR compliance and layer jurisdiction-specific requirements on top.

### 11.8 Legal Entity & Liability

| Decision | Options | Notes |
|---|---|---|
| **Legal structure** | Nonprofit (like Lichess) vs LLC vs benefit corporation | Nonprofit aligns with values but limits funding options. Benefit corp is a middle ground. |
| **Jurisdiction** | US (Delaware) vs EU vs other | Affects which laws are primary, where disputes are resolved |
| **Liability insurance** | Cyber liability, D&O insurance | Needed once the platform has real users |
| **DMCA agent registration** | File with US Copyright Office | Required before launch if US users can post content |
| **Legal counsel** | Attorney specializing in internet/gaming law | Needed for ToS/Privacy Policy review before launch. Templates are not enough. |

---

## 12. UX & INFRASTRUCTURE (Identified Gaps)

### 12.1 Onboarding Flow (First 5 Minutes)

New users need a guided path from signup to first game. Without this, retention drops catastrophically.

```
OAuth signup complete
    ↓
Welcome screen
    ├── "Play 4-player chess" (primary CTA — our differentiator)
    ├── "Play standard chess" (familiar territory)
    └── "Learn the rules" (interactive tutorial)
    ↓
First game
    ├── Matchmaking finds opponent(s)
    ├── Brief UI tour overlay ("This is your clock. This is the move list.")
    └── Post-game: "Good game! Here's what the engine thinks."
    ↓
Group discovery
    ├── Suggest "starter" public groups (seeded at launch)
    ├── "Invite a friend" prominent
    └── Auto-suggest groups by rating band and language
```

**The chicken-and-egg problem:** Invite-only groups are great for established communities but hostile to new users on a new platform. Solution: maintain a small number of **public lobby groups** (regional, rating-banded) that any user can join. These are moderated more heavily and serve as on-ramps. Users graduate to private groups naturally.

### 12.2 Connection Loss & Game Recovery

```
Client detects WebSocket disconnect
    ↓
Client shows "Reconnecting..." overlay
    ├── Auto-retry with exponential backoff (1s, 2s, 4s, 8s, max 30s)
    └── Clock PAUSES for disconnected player (grace period: 60 seconds)
    ↓
If reconnected within grace period:
    ├── Server sends full game state (authoritative)
    ├── Client rebuilds board from server state
    └── Clock resumes, game continues
    ↓
If grace period expires:
    ├── Opponent(s) offered: "Claim win" or "Wait longer"
    └── If all opponents claim: disconnected player forfeits
    ↓
If server restarts during game:
    ├── All active games stored in PostgreSQL (not just Redis)
    ├── On restart, server broadcasts "reconnect" to all game rooms
    └── Games resume from last confirmed move
```

### 12.3 Clock Synchronization

Online chess clocks are deceptively hard. Network latency creates disagreements.

| Problem | Solution |
|---|---|
| **Client-server time drift** | Server is authoritative on all time. Client displays estimated time; server validates. |
| **Network latency unfairness** | Measure round-trip latency per move. Compensate by adding half RTT back to player's clock. |
| **Move timestamp validation** | Server records when move was received, not when client claims it was made. |
| **Lag spikes** | If RTT exceeds 2 seconds, pause clock for that move's transmission time. Cap at 5 seconds to prevent abuse. |
| **Premove** | Allow premove (input move before opponent finishes). Zero time consumed if premove is legal. |

### 12.4 Notification System

| Event | Channel | Default |
|---|---|---|
| Game starting / your turn | In-app (banner) | ON |
| Game starting / your turn | Push notification (PWA) | OFF (opt-in) |
| Group message | In-app (badge count) | ON |
| Group message | Push notification | OFF (opt-in) |
| Moderation update | In-app + email | ON (cannot disable moderation emails) |
| Friend came online | In-app (subtle indicator) | ON |
| Weekly digest | Email | OFF (opt-in) |
| Platform announcements | In-app (banner) | ON, dismissable |

**Philosophy:** Notifications serve the user, not engagement metrics. Default to quiet. Never guilt. Never FOMO. Users control everything except moderation-related notifications.

### 12.5 Spectator Mode

- Any rated game can be spectated (live, with 1-move delay to prevent cheating assistance)
- Tournament games featured on homepage
- Spectators see board + move list + optional engine analysis (delayed)
- No spectator chat on individual games (reduces toxicity)
- Tournament spectator chat moderated by tournament organizer

### 12.6 Error States

| Scenario | User Sees |
|---|---|
| **Server down** | Cached offline page: "We're having technical difficulties. Your games are saved." + status page link |
| **Matchmaking timeout (>3 min)** | "No opponents found yet. Expand search?" (offer wider rating range or switch game mode) |
| **Engine analysis timeout** | "Analysis is taking longer than expected. We'll notify you when it's ready." |
| **OAuth provider down** | "Your login provider is having issues. Try again in a few minutes." (show status of each provider) |
| **Game state corruption** | Flag game for review, offer draw to both players, log incident for investigation |
| **WebSocket failure (persistent)** | Fall back to polling (slower but functional). Show "Limited connectivity mode." |

### 12.7 Internationalization (i18n)

Chess is global. Translation support must be built from day one (retrofitting is 10x harder).

| Requirement | Implementation |
|---|---|
| **Translation framework** | All UI strings in translation files (JSON/YAML), never hardcoded. React-i18next or equivalent. |
| **Launch languages** | English (primary). Community translations for Spanish, Portuguese, Hindi, Russian, Chinese, Arabic, French, German. These cover ~80% of online chess players. |
| **RTL support** | Arabic and Hebrew require right-to-left layout. CSS logical properties (margin-inline-start, not margin-left). |
| **Date/time formatting** | Intl.DateTimeFormat per locale. No hardcoded date formats. |
| **Number formatting** | Intl.NumberFormat per locale. "1,000" vs "1.000" vs "1 000". |
| **Unicode display names** | Full Unicode support. No ASCII-only restrictions. Handle emoji in names. |
| **Chess notation** | Algebraic notation is universal — but piece letters vary by language (K/D/T/L/S/B in German). Support localized notation display. |

### 12.8 Disaster Recovery

| Component | Backup Strategy | RPO | RTO |
|---|---|---|---|
| **PostgreSQL** | Daily full backup + WAL archiving for point-in-time recovery | 5 minutes | 1 hour |
| **Redis** | No backup (ephemeral). Reconstruct from PostgreSQL on restart. | N/A (ephemeral) | Minutes |
| **Game state** | Active games persisted to PostgreSQL every move. Redis is cache only. | 0 (every move saved) | Minutes |
| **User uploads** | Object storage with versioning (S3-compatible) | 0 | Minutes |
| **Configuration** | Infrastructure-as-code in git. Reproducible deployments. | 0 | 30 minutes |
| **SSL certificates** | Auto-renewed via Let's Encrypt. Backup copy in secure storage. | N/A | Minutes |

**Monitoring stack:** Prometheus (metrics) + Grafana (dashboards) + Sentry (error tracking) + UptimeRobot (external ping). Alert to email + Discord webhook. One-person on-call is fine at launch scale.

### 12.9 Content Creator & API Support

| Feature | Description |
|---|---|
| **Embeddable board viewer** | `<iframe>` or JS widget. Anyone can embed a game/position on their site. |
| **Streamer mode** | Hide rating, hide opponent name, larger board, simplified UI. |
| **Public API** | REST API for game data, player stats, puzzle data. Rate-limited. No auth needed for public data. |
| **WebSocket API** | Real-time game events for overlay tools (OBS integration). |
| **PGN/FEN4 export** | Every game downloadable in standard format. Bulk export available. |
| **Game import** | Import PGN from chess.com/Lichess. Rating calibration games for new users. |
| **Open source ecosystem** | Clear API docs, example integrations, community-built tools encouraged. |

### 12.10 Open Source License

| Option | Pros | Cons | Used By |
|---|---|---|---|
| **AGPL-3.0** | Prevents proprietary forks. Anyone who deploys must share changes. | Some companies avoid AGPL code entirely. | Lichess |
| **MIT** | Maximum adoption. Anyone can use it anywhere. | Someone can fork, close, and commercialize. | Many JS projects |
| **Apache-2.0** | Like MIT but with patent protection. | Same risk of proprietary forks. | Rust ecosystem |
| **GPL-3.0** | Strong copyleft but library linking is complex in Rust. | Compatibility concerns with some dependencies. | GNU projects |

**Recommendation:** AGPL-3.0 for the engine and platform (matches Lichess, prevents exploitation). MIT for any standalone libraries/tools we publish (maximizes adoption).

---

## 13. OPEN DECISIONS

| Decision | Options | Notes |
|---|---|---|
| Rating system | Glicko-2 vs TrueSkill for 4PC | Glicko-2 is standard for chess; TrueSkill handles multiplayer natively |
| Matchmaking for 4PC | 4 random vs fill-as-available | Need to handle 3-player wait states |
| Fairy-Stockfish communication | UCI subprocess vs embedded | Subprocess is simpler; embedded is faster |
| LLM provider | Claude API vs OpenAI vs local model | Cost vs quality vs privacy tradeoffs |
| PWA vs native mobile | PWA first vs React Native later | PWA covers 90% of mobile use cases |
| Puzzle generation | Engine-analyzed blunders from real games | Same approach as Lichess puzzles |
| Domain name | TBD | odinchess.com? playodin.com? |
| VPN detection provider | IPQualityScore vs MaxMind vs self-hosted | Cost vs accuracy vs privacy |
| Device fingerprinting library | FingerprintJS vs custom | FingerprintJS is industry standard but adds dependency |
| Anti-cheat statistical model | Custom vs adapt existing (Lichess is open source) | Lichess model is proven for 2-player; 4PC needs custom |
| Data retention policy | 30/90/365 day tiers | Balance privacy with usefulness |
| Legal entity type | Nonprofit vs LLC vs benefit corporation | Defines tax status, funding options, governance |
| Legal jurisdiction | US (Delaware) vs EU | Affects primary legal framework |
| Minimum age | 13 (COPPA) vs 16 (strict GDPR) | 13 is Lichess standard, 16 avoids EU parental consent complexity |
| WCAG compliance level | AA (target) vs AAA (aspirational) | AA is standard; AAA is very strict but maximizes access |
| Cookie consent implementation | Custom vs Cookiebot vs Osano | Must support IAB TCF 2.2 + Google Consent Mode v2 |
| Open source license | AGPL-3.0 (Lichess model) vs MIT vs Apache-2.0 | AGPL prevents proprietary forks; see Section 12.10 |
| Connection grace period | 30s vs 60s vs 90s | Balance fairness (don't punish disconnects) vs opponent waiting |
| Public lobby groups | Yes (moderated) vs No (pure invite-only) | Needed to solve chicken-and-egg problem for new platform |
| Translation framework | react-i18next vs react-intl vs custom | Must support RTL and pluralization rules |
| Monitoring stack | Prometheus+Grafana vs Datadog vs custom | Self-hosted is cheaper; managed is less work |
| Game state persistence | Every move to PostgreSQL vs periodic flush | Every-move is safest but higher write load |
| Spectator delay | 0 moves vs 1 move vs 2 moves | Delay prevents real-time cheating assistance |
| Premove support | Yes vs No | Reduces time pressure for fast games but adds UI complexity |
| Training opt-in default | OFF (privacy-first) vs ON (more data) | OFF aligns with privacy ratchet (Section 6.8) |
| Training rating threshold | Top 10% vs top 20% vs top 30% | Higher threshold = cleaner data but less volume |
| Self-play GPU infrastructure | Cloud GPU (rental) vs donated compute vs local | Cost vs availability vs control |
| Training data format | Custom binary vs standard ML formats | Interoperability vs performance |
| Cross-pollination frequency | Every Live retrain vs quarterly | More frequent = tighter correction but more compute |

---

*This document is a living design. Update as decisions are made. Implementation begins after Odin engine is complete (Stage 19).*
