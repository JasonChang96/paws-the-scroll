# Demo Plan

## Goal

Create a 2-minute recorded hackathon demo that presents Paws The Scroll as a personal, funny, AI-powered behavioral activation companion.

The video should be a direct screen demo with talking-head narration. Avoid b-roll. The centerpiece is the full-screen cat interruption, with enough time to explain what the app is built on and where OpenAI is used.

## Core Message

My cat changed my mental health by doing something software is usually bad at: interrupting me with care, not shame. Paws The Scroll recreates that moment on the desktop: a needy cat appears when device paralysis takes over and turns being stuck into one tiny act of care.

## Audience

Hackathon judges.

## Tone

Hackathon dramatic, personal, and funny. The cat should be memorable and a little demanding without sounding shaming.

## What The App Runs On

Paws The Scroll is a local-first macOS desktop app:

- **Tauri v2** provides the desktop shell and lets the app create native macOS overlay windows that sit above fullscreen apps and follow the user across Spaces.
- **React + TypeScript** renders onboarding, adoption, dashboard, and overlay UI.
- **Rust** owns activity tracking, foreground-app detection, scheduling, local storage, cat-state updates, and OpenAI calls.
- **OpenAI Responses API with `gpt-5.5`** generates cat dialogue, tiny behavioral-activation tasks, completion lines, and structured task bundles.
- **OpenAI Image API with `gpt-image-2`** generates and regenerates cat portraits, streamed with partial images during reveal.
- **`tauri-plugin-store`** stores user profile, cat state, task events, settings, and generated portrait paths locally.

The user's OpenAI key stays in the local app store. The frontend never directly calls OpenAI; Rust does.

## Where AI Appears In The Demo

### `gpt-5.5`

Use `gpt-5.5` for language and decision output:

- cat line shown in the interruption
- cat need
- one tiny task
- task category, difficulty, estimated seconds, and safety flags
- completion reaction

The model receives a compact local context packet: goals, stuck patterns, mobility, environment, boundaries, cat state, recent rerolls, recent task outcomes, foreground-app category, and time of day. It returns a strict structured task bundle that the app validates before showing.

### `gpt-image-2`

Use `gpt-image-2` for cat visuals:

- initial adoption portraits
- portrait reveal during cat selection
- lazy portrait regeneration when mood, independence tier, or skills change
- visual evolution cues like mood, self-feeding, independent play, or grooming

For the recording, show at least one visible portrait generation or regenerated cat image so the image model usage is obvious.

## Demo Arc

1. Personal hook: my cat changed my mental health by interrupting paralysis with care instead of shame.
2. Base app explanation: this app uses Tauri for native macOS overlays, React for the UI, Rust for activity tracking/local state/OpenAI calls, and local storage by default.
3. Onboarding/adoption glimpse: show that the user gives goals, boundaries, mobility, environment, tone, and an OpenAI key; then the app generates cat options with `gpt-image-2`.
4. Problem setup: show a normal stuck-at-computer state with tabs, notes, or an avoided task.
5. Product moment: trigger the full-screen cat interruption with `Cmd + Ctrl + Opt + P`.
6. `gpt-5.5` moment: explain that the cat line and tiny task are generated from a structured local context packet.
7. Payoff: click `I did it`; the cat reacts, needs update, and the app can reward time away.
8. Cat evolution: explain that task history and time away change mood, independence, skills, demeanor, and future portraits.
9. Closing line: the app does not ask the user to become disciplined; it gives them a cat needy enough to make one tiny action possible.

## 2-Minute Script

The biggest change in my mental health did not start with a productivity system.

It started with a cat.

Not because she fixed anything for me. Because when I was frozen, she interrupted me with a need I could actually answer.

When I am stuck in paralysis, I do not need another dashboard telling me I failed. I need something small, immediate, and hard to ignore, but not shameful.

So I built Paws The Scroll: a local-first macOS desktop cat that interrupts device paralysis.

The app uses Tauri for the native macOS shell and overlay behavior, React and TypeScript for the interface, and Rust for the parts that need to be close to the system: activity tracking, foreground-app detection, scheduling, local storage, cat state, and the OpenAI client.

On first launch, I tell the app what I am trying to move toward, what I tend to get stuck in, what my body can handle, where I usually am, and what tasks are off-limits. The OpenAI key is stored locally, and all OpenAI calls go through Rust.

This is also where the first big AI moment happens: `gpt-image-2` generates the cat portraits. The cat starts as a broad personality, but over time its mood, appearance, and demeanor become more specific to me.

Now imagine I am frozen at my laptop. I have been actively using the computer too long, and I am not really choosing the next thing anymore.

Paws The Scroll tracks active use locally. For the demo, I am using the built-in trigger, `Cmd + Ctrl + Opt + P`, which summons the same full-screen interruption without waiting for the scheduler.

Here, `gpt-5.5` is generating the cat's line and the task. But it is not just writing random wellness advice. It gets a compact context packet: my goals, stuck patterns, mobility, environment, boundaries, the cat's current state, recent rerolls, and what kinds of tasks I actually complete.

Then it returns a structured task bundle: the cat's need, the dialogue, the task category, difficulty, estimated time, and safety flags. The app validates that before showing it.

So instead of saying "be productive," the cat can say: "Human. You have become furniture. Stand up for ten seconds so I know you are alive."

The task is tiny on purpose. Stand up. Drink water. Clear one object. Start one step. The cat does not ask me to fix my life. It asks for one act of care small enough to do now.

When I click `I did it`, the cat calms down, its needs update, and the app records what worked. If I spend time away afterward, the cat becomes more independent.

Over time, the cat becomes more owner-like-pet: if I respond well to grounding tasks, it gets calmer; if movement helps, it gets more playful; if I keep working late, it learns to be sleepier and gentler at night. `gpt-image-2` can regenerate portraits when mood, independence, or skills change, so the cat starts to look and act like a relationship, not a mascot.

Paws The Scroll is evidence-informed behavioral support, not treatment. It is a needy desktop cat that turns the moment where I disappear into my screen into one tiny act of care.

It does not try to make me a more optimized person. It gives me a cat that can reach me when I cannot quite reach myself.

## Recording Beats

1. Start on talking head: "The biggest change in my mental health did not start with a productivity system. It started with a cat."
2. Show the app launch or main window and briefly say it uses Tauri for native macOS overlays, React for UI, and Rust for tracking/storage/OpenAI calls.
3. Show onboarding/profile fields quickly: goals, stuck patterns, mobility, environment, boundaries, tone, and OpenAI key.
4. Show cat adoption or portrait generation with `gpt-image-2`.
5. Show a normal stuck-at-computer setup: browser, notes, cluttered tabs, or an avoided task.
6. Trigger the demo interruption with `Cmd + Ctrl + Opt + P`.
7. Show the full-screen cat overlay above the desktop.
8. Call out `gpt-5.5`: generated cat line, task, category, difficulty, and safety flags from local context.
9. Click `I did it`.
10. Show the cat reaction in the small bottom-right overlay using `gpt-image-2`, the needs update, or the dashboard for only a few seconds.
11. Mention that time away and task history change cat mood, independence, skills, demeanor, and future portraits.
12. End on the app or talking head with the punchline.

## Demo Build Priorities

1. Reliable full-screen overlay trigger.
2. Polished interruption state with a strong cat portrait, funny dialogue, one tiny task, and clear actions.
3. Visible `gpt-image-2` usage through adoption portrait generation or portrait regeneration.
4. Visible `gpt-5.5` usage through generated cat dialogue and a structured task.
5. Immediate completion payoff after clicking `I did it`.
6. Minimal dashboard proof if time allows: mood, needs, tiny actions completed, time-away reward, or independence.

## AI Personalization Talking Point

The AI is not just generating random wellness prompts. `gpt-5.5` receives a compact local context packet: my goals, stuck patterns, mobility, environment, boundaries, cat state, recent rerolls, foreground-app category, time of day, and what kinds of tasks I actually complete. Then it generates one tiny task in a strict schema, and the app validates it before showing it.

`gpt-image-2` handles the visual side: initial cat portraits and later portrait regeneration as the cat's mood, independence, and skills evolve.

See [Task Personalization](./task-personalization.md) for the longer product plan.

## Recommended Final Line

Paws The Scroll: for the moments when you cannot care for yourself directly, so the cat asks first.
