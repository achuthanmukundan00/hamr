<template>
  <div class="landing">
    <!-- Nav -->
    <nav class="nav">
      <a href="/" class="nav-brand">
        <span class="nav-mark">⚒</span>
        <span class="nav-text">hamr</span>
      </a>
      <div class="nav-links">
        <a href="/guide/getting-started">Guide</a>
        <a href="/guide/tool-call-parsing">Parsers</a>
        <a href="/guide/providers">Providers</a>
        <a href="/guide/relay">Relay</a>
        <a href="https://github.com/skaft/hamr" rel="noopener">GitHub</a>
      </div>
    </nav>

    <!-- Hero -->
    <section class="hero">
      <div class="term hero-term reveal">
        <div class="term-bar">
          <span class="term-dot d-r"></span><span class="term-dot d-y"></span><span class="term-dot d-g"></span>
          <span class="term-title">hamr — model switching on relay <span class="term-rec"></span></span>
        </div>
        <div id="hero-cast" class="term-cast"></div>
        <div class="term-status">
          <span>~/workspace/hamr (main)</span>
          <span class="term-status-model">⬡ qwen3.6-35b · relay</span>
        </div>
        <div class="term-caption">Live on Relay · model-adaptive colors shift with each switch</div>
      </div>

      <div class="hero-text reveal delay-1">
        <div class="hero-label">
          <span class="nav-mark">⚒</span> hamr
          <span class="hero-pill"><span class="hero-pill-dot"></span> v0.4.0</span>
        </div>
        <h1 class="hero-tagline">Your local models.<br>Your terminal.<br><em>Your rules.</em></h1>
        <p class="hero-desc">The coding agent for developers running models on consumer GPUs. Built for <a href="/guide/relay">Relay</a> and any OpenAI-compatible inference.</p>
        <div class="cta-row">
          <a href="/guide/getting-started" class="cta cta-primary">Get started →</a>
          <a href="https://github.com/skaft/hamr" rel="noopener" class="cta cta-ghost">GitHub</a>
        </div>
        <div class="hero-install"><span class="dollar">$</span> npm install -g @skaft/hamr</div>
      </div>
    </section>

    <!-- §1 Relay -->
    <div class="bg-alt">
      <section class="section section-reverse">
        <div class="section-cast reveal">
          <div class="term">
            <div class="term-bar">
              <span class="term-dot d-r"></span><span class="term-dot d-y"></span><span class="term-dot d-g"></span>
              <span class="term-title">hamr doctor <span class="term-rec"></span></span>
            </div>
            <div id="doctor-cast" class="term-cast"></div>
            <div class="term-caption">Probes Relay, models, skills, and memory in one command.</div>
          </div>
        </div>
        <div class="section-text reveal delay-1">
          <div class="section-kicker">Relay</div>
          <h3>One GPU, many models. Switch mid-session.</h3>
          <p><a href="/guide/relay" class="relay-link">Relay</a> is the compatibility gateway between your local llama.cpp server and real agent tooling. Model lifecycle, graceful switching, prefix-cache pre-warming. Point Hamr at Relay and go.</p>
          <p>Switch models with <code>/model</code>. Each family gets its own color and glyph. The conversation keeps going.</p>
          <div class="model-strip">
            <span class="model-chip"><span style="color:#875fff">⬡</span> Qwen</span>
            <span class="model-chip"><span style="color:#d08030">▲</span> Claude</span>
            <span class="model-chip"><span style="color:#0087ff">∞</span> Llama</span>
            <span class="model-chip"><span style="color:#f06030">◈</span> Mistral</span>
            <span class="model-chip"><span style="color:#005faf">◖</span> DeepSeek</span>
            <span class="model-chip"><span style="color:#cccccc">●</span> GPT</span>
          </div>
          <div class="relay-box">
            <strong>Relay</strong> makes local llama.cpp servers look like hosted APIs. OpenAI + Anthropic compatible. Graceful model switching. No dropped requests. <a href="https://github.com/skaft/relay">→ Relay docs</a>
          </div>
          <p style="margin-top:16px"><a href="/guide/providers" class="section-link">Provider setup →</a></p>
        </div>
      </section>
    </div>

    <!-- §2 Agent loop -->
    <section class="section">
      <div class="section-cast reveal">
        <div class="term">
          <div class="term-bar">
            <span class="term-dot d-r"></span><span class="term-dot d-y"></span><span class="term-dot d-g"></span>
            <span class="term-title">hamr — agent workflow <span class="term-rec"></span></span>
          </div>
          <div id="agent-cast" class="term-cast"></div>
          <div class="term-status">
            <span>~/workspace/hamr (main)</span>
            <span>4 steps · 6 calls · 2.1K tokens</span>
          </div>
          <div class="term-caption">read → find bug → edit → test → pass. Bounded loop, 64 max steps.</div>
        </div>
      </div>
      <div class="section-text reveal delay-1">
        <div class="section-kicker">Agent loop</div>
        <h3>think · read · edit · test · verify</h3>
        <p>A bounded model↔tool loop designed for local inference. Read cache, identical-read detection, escalating nudges, deterministic compaction, repair with verification contracts.</p>
        <p>Up to 64 model steps and 192 tool calls. Context is a hard budget — not a suggestion. Won't burn your window. Won't loop forever.</p>
        <p><a href="/guide/agent-loop" class="section-link">Agent loop docs →</a></p>
      </div>
    </section>

    <!-- §3 Themes -->
    <div class="bg-alt">
      <section class="section section-reverse">
        <div class="section-cast reveal">
          <div class="term">
            <div class="term-bar">
              <span class="term-dot d-r"></span><span class="term-dot d-y"></span><span class="term-dot d-g"></span>
              <span class="term-title">hamr — theme engine <span class="term-rec"></span></span>
            </div>
            <div id="theme-cast" class="term-cast"></div>
            <div class="term-caption">60+ color tokens, hot-reload, Pi-compatible.</div>
          </div>
        </div>
        <div class="section-text reveal delay-1">
          <div class="section-kicker">Theme engine</div>
          <h3>60+ tokens. Hot-reload. Pi-compatible.</h3>
          <p>JSON themes with variables, 60+ color tokens — syntax, markdown, editor, diffs, all themed. Ship your own or drop in a Pi theme directly.</p>
          <p>Model-adaptive coloring tints the entire UI to the active model's brand. Qwen is purple. Claude is orange. Llama is blue. Switch models, watch the TUI shift.</p>
          <p>Edit a theme JSON, the TUI hot-reloads. No restart.</p>
          <p><a href="/guide/getting-started#themes" class="section-link">Theme reference →</a></p>
        </div>
      </section>
    </div>

    <!-- §4 Parsers -->
    <section class="section">
      <div class="section-cast reveal">
        <div class="parser-grid">
          <div v-for="p in parsers" :key="p.id" class="parser-card">
            <span class="parser-glyph" :style="{ color: p.color }">{{ p.glyph }}</span>
            <div class="parser-name">{{ p.name }}</div>
            <div class="parser-id">{{ p.id }}</div>
          </div>
        </div>
      </div>
      <div class="section-text reveal delay-1">
        <div class="section-kicker">Tool-call parsing</div>
        <h3>12 native parsers. No vLLM required.</h3>
        <p>Local models emit malformed XML, leaked <code>&lt;think&gt;</code> tags, broken JSON, mixed final answers. Most harnesses need vLLM to normalize.</p>
        <p>Hamr doesn't. Twelve native parsers, auto-detected from your model name. JSON and XML get salvaged, reasoning tags get stripped, truncated calls get recovered.</p>
        <p><a href="/guide/tool-call-parsing" class="section-link">Parser matrix →</a></p>
      </div>
    </section>

    <!-- §5 Features + SDK -->
    <div class="bg-alt">
      <section class="section-wide reveal">
        <div class="section-kicker">And more</div>
        <h3 class="wide-heading">Sessions, memory, sub-agents, skills, SDK</h3>
        <div class="feature-grid">
          <div v-for="f in features" :key="f.title" class="feature-card">
            <span class="feature-icon">{{ f.icon }}</span>
            <h4>{{ f.title }}</h4>
            <p>{{ f.desc }}</p>
          </div>
        </div>
        <pre class="code-block"><span class="kw">import</span> { Session } <span class="kw">from</span> <span class="str">'@skaft/hamr'</span>
<span class="kw">import</span> { createAgentClient } <span class="kw">from</span> <span class="str">'@skaft/hamr/llm'</span>

<span class="kw">const</span> session = <span class="kw">new</span> <span class="cls">Session</span>({
  repoRoot: process.<span class="fn">cwd</span>(),
  client: <span class="fn">createAgentClient</span>({
    provider: <span class="str">'relay'</span>,
    model: <span class="str">'qwen3.6'</span>,
  }),
})
<span class="kw">const</span> result = <span class="kw">await</span> session.<span class="fn">startTurnWithRecovery</span>(<span class="str">'Find all TODOs in src/'</span>)</pre>
      </section>
    </div>

    <!-- Closer -->
    <section class="closer reveal">
      <h3>Built for local inference.</h3>
      <p>No cloud required. No vLLM required. No Docker required. One GPU, Relay, and <code>hamr chat</code>. Everything else is optional.</p>
      <div class="cta-row" style="justify-content:center">
        <a href="/guide/getting-started" class="cta cta-primary">Get started →</a>
        <a href="https://github.com/skaft/hamr" rel="noopener" class="cta cta-ghost">GitHub</a>
      </div>
      <div class="closer-row">
        <span class="closer-item">✕ No cloud lock-in</span>
        <span class="closer-item">✕ No vLLM dependency</span>
        <span class="closer-item">✕ No Docker required</span>
        <span class="closer-item">✕ No API key to start</span>
      </div>
    </section>

    <!-- Footer -->
    <footer class="footer">
      <p class="footer-line">
        <span style="color:var(--accent)">⚒</span>
        <strong>Hamr</strong>
        <span class="footer-sep">·</span>
        <a href="https://skaft.dev">Skaft Software</a>
        <span class="footer-sep">·</span>
        <a href="https://github.com/skaft/hamr">GitHub</a>
        <span class="footer-sep">·</span>
        <a href="https://github.com/skaft/relay">Relay</a>
        <span class="footer-sep">·</span>
        <a href="https://github.com/skaft/hamr/blob/main/LICENSE">MIT</a>
      </p>
      <p class="footer-sub">© 2026 Skaft Software</p>
    </footer>
  </div>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted } from 'vue'

const parsers = [
  { glyph: '⬡', name: 'Qwen 3', id: 'qwen3_xml', color: '#875fff' },
  { glyph: '∞', name: 'Llama 3', id: 'llama3_json', color: '#0087ff' },
  { glyph: '◈', name: 'Mistral', id: 'mistral', color: '#f06030' },
  { glyph: '◖', name: 'DeepSeek', id: 'deepseek_v3', color: '#005faf' },
  { glyph: '◆', name: 'Hermes', id: 'hermes', color: '#61afef' },
  { glyph: '◆', name: 'GLM-4', id: 'glm45', color: '#00afaf' },
  { glyph: 'g', name: 'Gemma', id: 'functiongemma', color: '#5098e8' },
  { glyph: '∞', name: 'Llama 4', id: 'llama4_pythonic', color: '#aaa' },
  { glyph: '◆', name: 'Granite', id: 'granite', color: '#888' },
  { glyph: '◆', name: 'OLMo3', id: 'olmo3', color: '#888' },
  { glyph: '☾', name: 'Kimi', id: 'kimi', color: '#aaa' },
  { glyph: 'ƒ', name: 'xLAM', id: 'xlam', color: '#888' },
]

const features = [
  { icon: '📋', title: 'Sessions', desc: 'JSONL event logs. Resume, fork, replay. Every tool call recorded.' },
  { icon: '🧠', title: 'Holographic memory', desc: 'SQLite FTS5 search across all sessions. Pattern recall, not just history.' },
  { icon: '⚡', title: 'Sub-agents', desc: 'Parallel orchestration via explicit delegation. Dependency-aware task splitting.' },
  { icon: '📦', title: 'Skills', desc: 'SKILL.md files auto-discovered from ~/.hamr/skills/ and .hamr/skills/.' },
  { icon: '🔌', title: 'Extensions & MCP', desc: 'TypeScript extensions for custom tools, commands, and events.' },
  { icon: '🔧', title: 'SDK', desc: 'Embed the Session, EventBus, and tools in your own apps.' },
]

const CASTS: [string, string][] = [
  ['hero-cast', '/recordings/hero-relay.cast'],
  ['doctor-cast', '/recordings/doctor-relay.cast'],
  ['agent-cast', '/recordings/agent-loop.cast'],
  ['theme-cast', '/recordings/theme-engine.cast'],
]

let styleEl: HTMLLinkElement | null = null
let scriptEl: HTMLScriptElement | null = null
let observer: IntersectionObserver | null = null

function ensurePlayer(): void {
  if (typeof document === 'undefined') return
  if (!styleEl) {
    styleEl = document.createElement('link')
    styleEl.rel = 'stylesheet'
    styleEl.href = '/asciinema-player.css'
    document.head.appendChild(styleEl)
  }
  if (!scriptEl) {
    scriptEl = document.createElement('script')
    scriptEl.src = '/asciinema-player.min.js'
    scriptEl.onload = mountCasts
    document.head.appendChild(scriptEl)
  } else if ((window as any).AsciinemaPlayer) {
    mountCasts()
  }
}

function mountCasts(): void {
  const w = window as any
  if (!w.AsciinemaPlayer) return
  CASTS.forEach(([id, src], idx) => {
    const el = document.getElementById(id)
    if (!el || el.dataset.mounted === '1') return
    el.dataset.mounted = '1'
    setTimeout(() => {
      try {
        w.AsciinemaPlayer.create(src, el, {
          autoPlay: true,
          loop: true,
          speed: 1.4,
          idleTimeLimit: 2,
          controls: false,
          // fit: 'width' scales the recording's native grid to the
          // container width on every device — no fixed cols, no
          // letterboxing, no tall-narrow squeeze. Font size follows.
          fit: 'width',
          terminalFontFamily: 'JetBrainsMono Nerd Font Mono, JetBrains Mono, monospace',
          theme: 'asciinema',
        })
      } catch { /* player can fail in SSR */ }
    }, 200 * idx)
  })
}

function setupReveal(): void {
  if (typeof IntersectionObserver === 'undefined') return
  observer = new IntersectionObserver((entries) => {
    entries.forEach(e => { if (e.isIntersecting) e.target.classList.add('in') })
  }, { threshold: 0.1 })
  document.querySelectorAll('.reveal').forEach(el => observer!.observe(el))
}

onMounted(() => {
  ensurePlayer()
  setupReveal()
})

onUnmounted(() => {
  if (scriptEl) scriptEl.onload = null
  observer?.disconnect()
})
</script>

<style scoped>
/* ── Base ─────────────────────────────── */
.landing {
  --bg: #0a0a0e;
  --surface: #111116;
  --surface2: #18181f;
  --text: #d4d4d8;
  --text2: #a1a1aa;
  --muted: #71717a;
  --dim: #52525b;
  --accent: #8abeb7;
  --accent-hot: #b5e8e1;
  --relay: #568dd0;
  --mono: 'JetBrainsMono Nerd Font Mono', 'JetBrains Mono', 'Fira Code', ui-monospace, monospace;
  --sans: 'Inter', system-ui, -apple-system, sans-serif;
  --size-body: 15px;
  --size-heading: 28px;

  background: var(--bg);
  color: var(--text);
  font-family: var(--sans);
  font-size: var(--size-body);
  line-height: 1.65;
  -webkit-font-smoothing: antialiased;
  min-height: 100vh;
  overflow-x: hidden;
}
* { margin: 0; padding: 0; box-sizing: border-box; }
a { color: inherit; text-decoration: none; }
code { font-family: var(--mono); font-size: 0.9em; background: var(--surface2); padding: 2px 7px; border-radius: 4px; color: var(--text2); }

/* ── Reveal ─────────────────────────── */
.reveal { opacity: 0; transform: translateY(20px); transition: opacity 0.7s cubic-bezier(0.22,1,0.36,1), transform 0.7s cubic-bezier(0.22,1,0.36,1); }
.reveal.in { opacity: 1; transform: none; }
.delay-1 { transition-delay: 0.12s; }

/* ── Nav ──────────────────────────────── */
.nav { position: sticky; top: 0; z-index: 100; display: flex; align-items: center; justify-content: space-between; padding: 16px 40px; background: rgba(10,10,14,0.9); backdrop-filter: blur(12px); -webkit-backdrop-filter: blur(12px); font-family: var(--mono); font-size: var(--size-body); }
.nav-brand { display: flex; align-items: center; gap: 10px; font-weight: 700; }
.nav-mark { color: var(--accent); font-size: 20px; }
.nav-text { font-size: 16px; }
.nav-links { display: flex; gap: 28px; }
.nav-links a { color: var(--muted); transition: color 0.2s; }
.nav-links a:hover { color: var(--text); }

/* ── Hero ─────────────────────────────── */
.hero { max-width: 1140px; margin: 0 auto; padding: 48px 40px 88px; display: grid; grid-template-columns: 55fr 45fr; gap: 48px; align-items: center; }

/* Terminal ────────────────────────────── */
.term { background: var(--surface); border-radius: 10px; overflow: hidden; transition: box-shadow 0.4s; }
.term:hover { box-shadow: 0 24px 64px -20px rgba(138,190,183,0.12); }
.term-bar { display: flex; align-items: center; gap: 8px; padding: 10px 16px; background: var(--surface2); }
.term-dot { width: 10px; height: 10px; border-radius: 50%; }
.d-r { background: #ff5f57; }
.d-y { background: #ffbd2e; }
.d-g { background: #28c840; }
.term-title { margin-left: auto; font-family: var(--mono); font-size: var(--size-body); color: var(--dim); }
.term-rec { display: inline-block; width: 6px; height: 6px; border-radius: 50%; background: #f06030; margin-left: 6px; animation: pulse 1.5s ease-out infinite; }
@keyframes pulse { 0% { box-shadow: 0 0 0 0 rgba(240,96,48,0.6); } 70% { box-shadow: 0 0 0 6px rgba(240,96,48,0); } 100% { box-shadow: 0 0 0 0 rgba(240,96,48,0); } }
/* Reserve a landscape-ish slot before the player mounts to avoid layout
   shift. The player computes its own height from the 90x26 grid via
   fit:'width', so this is just a sensible placeholder min-height. */
.term-cast { min-height: 340px; background: var(--bg); }
.term-cast .ap-player { width: 100% !important; }
@media (max-width: 860px) { .term-cast { min-height: 260px; } }
.term-status { display: flex; justify-content: space-between; padding: 8px 22px; background: var(--surface2); font-family: var(--mono); font-size: var(--size-body); color: var(--dim); }
.term-status-model { animation: model-color 12s ease-in-out infinite; }
@keyframes model-color { 0%,15% { color:#875fff; } 20%,35% { color:#d08030; } 40%,55% { color:#0087ff; } 60%,70% { color:#ff69b4; } 75%,100% { color:#8abeb7; } }
.term-caption { padding: 12px 22px; background: var(--surface2); color: var(--dim); }
.hero-term { box-shadow: 0 0 0 1px rgba(135,95,255,0.3), 0 24px 64px -20px rgba(135,95,255,0.1); animation: border-morph 12s ease-in-out infinite; }
@keyframes border-morph { 0%,15% { box-shadow:0 0 0 1px rgba(135,95,255,0.3),0 24px 64px -20px rgba(135,95,255,0.1); } 20%,35% { box-shadow:0 0 0 1px rgba(208,128,48,0.3),0 24px 64px -20px rgba(208,128,48,0.1); } 40%,55% { box-shadow:0 0 0 1px rgba(0,135,255,0.3),0 24px 64px -20px rgba(0,135,255,0.1); } 60%,70% { box-shadow:0 0 0 1px rgba(255,105,180,0.3),0 24px 64px -20px rgba(255,105,180,0.1); } 75%,100% { box-shadow:0 0 0 1px rgba(138,190,183,0.3),0 24px 64px -20px rgba(138,190,183,0.1); } }

/* Hero text ───────────────────────────── */
.hero-text { max-width: 480px; }
.hero-label { display: inline-flex; align-items: center; gap: 8px; font-family: var(--mono); font-size: var(--size-body); font-weight: 700; color: var(--text2); margin-bottom: 20px; }
.hero-pill { display: inline-flex; align-items: center; gap: 6px; padding: 4px 10px; background: var(--surface); border-radius: 999px; color: var(--muted); }
.hero-pill-dot { width: 5px; height: 5px; border-radius: 50%; background: var(--accent); animation: pulse 2s ease-out infinite; }
.hero-tagline { font-family: var(--sans); font-size: clamp(var(--size-heading), 4.5vw, 42px); font-weight: 600; color: #fafafa; letter-spacing: -0.03em; line-height: 1.12; margin-bottom: 18px; }
.hero-tagline em { font-style: normal; background: linear-gradient(120deg, var(--accent), var(--accent-hot)); -webkit-background-clip: text; background-clip: text; -webkit-text-fill-color: transparent; }
.hero-desc { color: var(--muted); margin-bottom: 24px; }
.hero-desc a { color: var(--relay); }
.cta-row { display: flex; gap: 10px; flex-wrap: wrap; margin-bottom: 20px; }
.cta { display: inline-flex; align-items: center; gap: 8px; padding: 12px 22px; border-radius: 6px; font-family: var(--mono); font-size: var(--size-body); font-weight: 600; cursor: pointer; transition: all 0.2s; border: none; }
.cta-primary { background: var(--accent); color: #0a0a0e; }
.cta-primary:hover { background: var(--accent-hot); transform: translateY(-1px); box-shadow: 0 8px 24px -8px var(--accent); }
.cta-ghost { background: var(--surface); color: var(--text2); }
.cta-ghost:hover { background: var(--surface2); color: var(--accent); }
.hero-install { font-family: var(--mono); font-size: var(--size-body); color: var(--dim); padding: 12px 18px; background: var(--surface); border-radius: 6px; display: inline-flex; gap: 8px; }
.dollar { color: var(--accent); user-select: none; }

/* Sections ────────────────────────────── */
.section { max-width: 1140px; margin: 0 auto; padding: 88px 40px; display: grid; grid-template-columns: 1fr 1fr; gap: 56px; align-items: center; }
.section-reverse .section-text { order: -1; }
.section-wide { display: block; max-width: 900px; padding: 88px 40px; margin: 0 auto; }
.bg-alt { background: var(--surface); }
.section-kicker { font-family: var(--mono); font-size: var(--size-body); font-weight: 600; color: var(--accent); margin-bottom: 12px; text-transform: uppercase; letter-spacing: 0.08em; }
.section-text h3 { font-family: var(--sans); font-size: var(--size-heading); font-weight: 600; color: #fafafa; letter-spacing: -0.02em; line-height: 1.2; margin-bottom: 16px; }
.section-text p { color: var(--muted); margin-bottom: 14px; }
.section-text p:last-child { margin-bottom: 0; }
.section-link { display: inline-flex; align-items: center; gap: 6px; color: var(--accent); font-family: var(--mono); font-size: var(--size-body); font-weight: 600; transition: gap 0.2s; }
.section-link:hover { gap: 10px; color: var(--accent-hot); }
.relay-link { color: var(--relay); }
.relay-box { margin-top: 16px; padding: 16px 20px; background: rgba(86,141,208,0.08); border-radius: 8px; color: var(--text2); }
.relay-box strong { color: var(--relay); }
.relay-box a { color: var(--relay); }
.model-strip { display: flex; gap: 10px; margin-top: 14px; flex-wrap: wrap; }
.model-chip { display: inline-flex; align-items: center; gap: 6px; padding: 6px 14px; background: var(--surface2); border-radius: 999px; font-family: var(--mono); font-size: var(--size-body); color: var(--text2); }

/* Parser grid ─────────────────────────── */
.parser-grid { display: grid; grid-template-columns: repeat(4, 1fr); gap: 8px; }
.parser-card { background: var(--surface); border-radius: 8px; padding: 16px; text-align: center; transition: background 0.2s, transform 0.2s; }
.parser-card:hover { background: var(--surface2); transform: translateY(-2px); }
.parser-glyph { font-size: 22px; margin-bottom: 6px; display: block; }
.parser-name { font-family: var(--mono); font-size: var(--size-body); font-weight: 600; color: var(--text2); }
.parser-id { font-family: var(--mono); font-size: var(--size-body); color: var(--dim); margin-top: 2px; }

/* Feature cards ───────────────────────── */
.wide-heading { font-family: var(--sans); font-size: var(--size-heading); font-weight: 600; color: #fafafa; letter-spacing: -0.02em; margin-bottom: 24px; }
.feature-grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: 12px; margin-top: 24px; }
.feature-card { background: var(--surface); border-radius: 8px; padding: 22px; transition: background 0.2s, transform 0.2s; }
.feature-card:hover { background: var(--surface2); transform: translateY(-2px); }
.feature-icon { font-size: 22px; margin-bottom: 10px; display: block; }
.feature-card h4 { font-family: var(--mono); font-size: var(--size-body); font-weight: 600; color: var(--text); margin-bottom: 6px; }
.feature-card p { color: var(--muted); }

/* Code block ──────────────────────────── */
.code-block { font-family: var(--mono); font-size: var(--size-body); line-height: 1.7; background: var(--surface); border-radius: 8px; padding: 20px 22px; margin-top: 20px; white-space: pre; overflow-x: auto; color: var(--text2); }
.kw { color: #c594c5; }
.str { color: var(--accent); }
.cls { color: #e5c07b; }
.fn { color: #DCDCAA; }

/* Closer ──────────────────────────────── */
.closer { max-width: 1140px; margin: 0 auto; padding: 96px 40px; text-align: center; }
.closer h3 { font-family: var(--sans); font-size: 32px; font-weight: 600; color: #fafafa; margin-bottom: 16px; }
.closer p { color: var(--muted); max-width: 560px; margin: 0 auto 28px; }
.closer-row { display: flex; justify-content: center; gap: 28px; margin-top: 24px; }
.closer-item { font-family: var(--mono); font-size: var(--size-body); color: var(--dim); }

/* Footer ──────────────────────────────── */
.footer { text-align: center; padding: 48px 40px; font-family: var(--mono); font-size: var(--size-body); color: var(--dim); }
.footer a { color: var(--muted); transition: color 0.2s; }
.footer a:hover { color: var(--accent); }
.footer-line { display: flex; align-items: center; justify-content: center; gap: 10px; flex-wrap: wrap; }
.footer-line strong { color: var(--text2); }
.footer-sep { color: var(--dim); }
.footer-sub { margin-top: 8px; }

/* On bg-alt, parser/feature cards need different base */
.bg-alt .model-chip { background: var(--bg); }
.bg-alt .feature-card { background: var(--bg); }
.bg-alt .feature-card:hover { background: var(--surface2); }
.bg-alt .code-block { background: var(--bg); }

/* Responsive ──────────────────────────── */
@media (max-width: 860px) {
  .hero, .section { grid-template-columns: 1fr; gap: 32px; padding: 48px 20px; }
  .section-reverse .section-text { order: unset; }
  .parser-grid { grid-template-columns: repeat(3, 1fr); }
  .feature-grid { grid-template-columns: 1fr 1fr; }
  .nav { padding: 14px 20px; }
  .nav-links { gap: 16px; }
}
@media (max-width: 480px) {
  .parser-grid { grid-template-columns: repeat(2, 1fr); }
  .feature-grid { grid-template-columns: 1fr; }
  .nav-links a:last-child { display: none; }
}
@media (prefers-reduced-motion: reduce) {
  .reveal { opacity: 1; transform: none; transition: none; }
  .hero-term { animation: none; }
  .term-status-model { animation: none; }
  .term-rec { animation: none; }
}
</style>
