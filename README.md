# Paws The Scroll

A macOS desktop companion that interrupts prolonged active device use with a needy virtual cat. The cat asks you to complete tiny behavioral-activation tasks, then rewards both task completion and time spent away from the device.

See [`PRD.md`](./PRD.md) for the full product brief.

## Stack

- **Tauri v2** desktop shell with macOS NSPanel overlay (floats above fullscreen apps, follows you across Spaces)
- **React + TypeScript** frontend (Vite, Biome)
- **Rust** activity tracking, OpenAI client, store, scheduler
- **OpenAI Responses API** (`gpt-5.5`, `reasoning_effort: none`, `verbosity: low`) for cat dialogue and tasks
- **OpenAI Image API** (`gpt-image-2`, `quality: low`, streamed via `partial_images: 3`) for cat portraits
- **`tauri-plugin-store`** for local JSON state (no cloud, no telemetry)

## What's wired

| Feature | Where |
|---|---|
| Onboarding wizard with soft-language steps | `src/views/Onboarding.tsx` |
| Three cat picks (Mango / Pluto / Bean) with streamed `gpt-image-2` portrait reveal | `src/views/CatAdoption.tsx` |
| Idle companion overlay (top-right, all-Spaces, above fullscreen) | `src-tauri/src/overlay.rs` |
| Activity scheduler — 5s tick, idle via `CGEventSourceSecondsSinceLastEventType`, foreground app via `NSWorkspace.frontmostApplication` | `src-tauri/src/activity/` |
| Social-app classifier (bundle-ID based) | `src-tauri/src/activity/classifier.rs` |
| Hidden demo trigger (`Cmd+Ctrl+Opt+P`) | `src-tauri/src/demo_trigger.rs` |
| Full-screen interruption with multi-monitor mirroring, 5s lockout, reroll, fallback after 5 rerolls | `src/OverlayApp.tsx`, `src-tauri/src/overlay.rs` |
| Cat-state evolution: per-task need decrements, derived mood, streak skills (day 7/14/30), autonomous decay | `src-tauri/src/cat_state.rs` |
| Lazy portrait regen when mood/tier/skills change | `src/OverlayApp.tsx`, `src-tauri/src/openai.rs` |
| Dashboard with non-shaming framing | `src/views/Dashboard.tsx` |

## Cat evolution

The cat is generated and re-generated based on what you do.

- **Per-task rewards.** Completing a *movement* task lowers boredom and play_drive. *Food* lowers hunger. *Grounding* lowers loneliness and attention. The mood is derived from the resulting need state — a cat with all-low needs and a fresh win looks `affectionate`; a cat with one screaming need still looks `dramatic`.
- **Streak skills.** Distinct days with at least one completion are counted from the task-event log. Crossing thresholds unlocks tier skills:
  - Day 7: **occasional self-feeding** — hunger decays passively
  - Day 14: **independent play** — boredom and play_drive decay passively
  - Day 30: **self-grooming** — litter decays passively
- **Visual evolution.** Skills feed the portrait prompt as visual cues ("with a tiny self-caught morsel nearby", "immaculately groomed"). The cache key includes mood, independence tier, and a hash of the skill set, so each new combination produces a fresh portrait that stays cached after.
- **Time-away rewards.** Lock the screen after an interruption and the cat credits time-away to today's aggregate, bumps `independence_level`, and adds a story moment.

## Run it

```bash
pnpm install
pnpm tauri dev
```

First launch: the onboarding wizard will ask for your OpenAI key. The key is stored locally at `~/Library/Application Support/com.paws-the-scroll.app/paws-the-scroll.json` and never leaves your machine — all OpenAI calls are made from Rust.

> **Important:** `gpt-image-2` requires API Organization Verification in your OpenAI developer console before it'll generate. The first cat-portrait call will 403 if you haven't verified.

Demo trigger: `Cmd + Ctrl + Opt + P` from any app summons the cat over your screen, bypassing the grace period.

For verbose Rust logs:

```bash
RUST_LOG=paws_the_scroll_lib=info pnpm tauri dev
```

## Build a release bundle

```bash
pnpm tauri build
```

Produces a `.app` and a signed `.dmg` (if codesigning is set up) under `src-tauri/target/release/bundle/`.

## Development

See [`CONTRIBUTING.md`](./CONTRIBUTING.md) for code style, type-design conventions, and the static-analysis commands. The short version:

```bash
pnpm check                                                   # biome + tsc + cargo clippy + cargo fmt
cargo clippy --all-targets --all-features --manifest-path src-tauri/Cargo.toml
```

Both should be clean before pushing.

## Privacy

This is a local-first app. All state — user profile, cat record, task history, daily aggregates, OpenAI key, generated cat sprites — stays on disk under `~/Library/Application Support/com.paws-the-scroll.app/`. The only network traffic is direct calls from your machine to `api.openai.com` using your key.

## Safety

This app is not emergency support or a substitute for professional care. If you might hurt yourself or someone else, contact local emergency services or a crisis line now. The app surfaces this notice in onboarding and settings.
