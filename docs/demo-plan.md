# Demo Plan

## Goal

Create a 2-minute recorded hackathon demo that presents Paws The Scroll as a personal, funny, AI-powered behavioral activation companion.

The video should be a direct screen demo with talking-head narration. Avoid b-roll. The centerpiece is the full-screen cat interruption, with enough time to show how OpenAI makes the cat feel personal.

## Core Message

My cat changed my mental health by lovingly bullying me out of paralysis. Paws The Scroll recreates that moment on the desktop: an annoying but endearing cat interrupts a stuck loop and turns it into one tiny act of care.

## Audience

Hackathon judges.

## Tone

Hackathon dramatic, personal, and funny. The cat should be memorable and a little demanding without sounding shaming.

## What The User Experiences

Paws The Scroll should feel like a pet interrupting a stuck loop, not another productivity app:

- The user tells the app what they are trying to move toward, what they get stuck in, what tasks are off-limits, and what tone they want from the cat.
- The app gives them a cat with a visible personality and a generated portrait.
- When the user has been actively stuck on the computer, the cat appears over the screen.
- The cat asks for one tiny action: stand up, drink water, clear one object, ground, or start one step.
- If the user completes, rerolls, dismisses, or marks a task inaccessible, the cat learns from that.
- Over time, the cat's mood, demeanor, independence, and appearance become more specific to the user.

The technical base is still visible if judges ask: Tauri powers the desktop app and overlay, React renders the interface, Rust handles activity tracking and app state, `gpt-5.5` generates structured dialogue/tasks, and `gpt-image-2` generates cat portraits.

## Where AI Appears In The Demo

### `gpt-5.5`

Use `gpt-5.5` for language and decision output:

- cat line shown in the interruption
- cat need
- one tiny task
- task category, difficulty, estimated seconds, and safety flags
- completion reaction

The model receives a compact context packet: goals, stuck patterns, mobility, environment, boundaries, cat state, recent rerolls, recent task outcomes, foreground-app category, and time of day. It returns a strict structured task bundle that the app validates before showing.

### `gpt-image-2`

Use `gpt-image-2` for cat visuals:

- initial adoption portraits
- portrait reveal during cat selection
- lazy portrait regeneration when mood, independence tier, or skills change
- visual evolution cues like mood, self-feeding, independent play, or grooming

For the recording, show at least one visible portrait generation or regenerated cat image so the image model usage is obvious.

## Demo Arc

1. Personal hook: my cat changed my mental health by lovingly bullying me out of paralysis.
2. User setup: show that the user gives goals, boundaries, mobility, environment, and tone so the cat knows what kinds of help fit.
3. Adoption glimpse: show the user choosing a cat generated with `gpt-image-2`.
4. Problem setup: show a normal stuck-at-computer state with tabs, notes, or an avoided task.
5. Product moment: trigger the full-screen cat interruption with `Cmd + Ctrl + Opt + P`.
6. `gpt-5.5` moment: explain that the cat line and tiny task are generated from a structured context packet.
7. Payoff: click `I did it`; the cat reacts, needs update, and the app can reward time away.
8. Cat evolution: explain that task history and time away change mood, independence, skills, demeanor, and future portraits.
9. Closing line: the app does not ask the user to become disciplined; it gives them a cat needy enough to make one tiny action possible.

## 2-Minute Script

The biggest change in my mental health did not start with a productivity system.

It started with getting bullied by a cat.

My cat is annoying, demanding, and somehow exactly what I need. She does not fix anything for me. But when I am frozen, she interrupts me with a need I can actually answer.

When I am stuck in paralysis, I do not need another dashboard telling me I failed. I need something small, immediate, and hard to ignore, but not shameful.

So I built Paws The Scroll: a desktop cat that interrupts device paralysis.

On first launch, I tell the app what I am trying to move toward, what tends to stop me, and what kinds of tasks are off-limits.

This is also where the first big AI moment happens: `gpt-image-2` generates the cat portraits. The cat starts as a broad personality, but over time its mood, appearance, and demeanor become more specific to me.

Now imagine I am frozen at my laptop. I have been actively using the computer too long, and I am not really choosing the next thing anymore.

Paws The Scroll watches for active computer use. For the demo, I am using the built-in trigger, `Cmd + Ctrl + Opt + P`, which summons the same full-screen interruption without waiting for the scheduler.

Here, `gpt-5.5` is generating the cat's line and the task. But it is not just writing random wellness advice. It gets a compact context packet about me, the cat, and what has or has not worked before.

Then it returns a structured task bundle with the cat's need, the task, and safety metadata. The app validates that before showing it.

So instead of saying "be productive," the cat can say: "Human. You have become furniture. Stand up for ten seconds so I know you are alive."

The task is tiny on purpose. The cat does not ask me to fix my life. It asks for one act of care small enough to do now.

When I click `I did it`, the cat calms down, its needs update, and the app records what worked. If I spend time away afterward, the cat becomes more independent.

Over time, the cat becomes more owner-like-pet. It learns what helps me, changes its mood and demeanor, and `gpt-image-2` can regenerate portraits as it becomes more independent. The cat starts to look and act like a relationship, not a mascot.

Paws The Scroll is evidence-informed behavioral support, not treatment. It is the experience of getting bullied by a cat, gently, until the moment where I disappear into my screen becomes one tiny act of care.

It does not try to make me a more optimized person. It gives me a cat that can reach me when I cannot quite reach myself.

Paws The Scroll: get bullied by a cat into caring for yourself, gently.

## Recording Beats

1. Start on talking head: "The biggest change in my mental health did not start with a productivity system. It started with getting bullied by a cat."
2. Show onboarding/profile fields quickly: goals, stuck patterns, mobility, environment, boundaries, and tone.
3. Briefly mention the technical base only as needed: Tauri overlay, React UI, Rust activity tracking, `gpt-5.5`, and `gpt-image-2`.
4. Show cat adoption or portrait generation with `gpt-image-2`.
5. Show a normal stuck-at-computer setup: browser, notes, cluttered tabs, or an avoided task.
6. Trigger the demo interruption with `Cmd + Ctrl + Opt + P`.
7. Show the full-screen cat overlay above the desktop.
8. Call out `gpt-5.5`: generated cat line, task, category, difficulty, and safety flags from user context.
9. Click `I did it`.
10. Show the cat reaction in the small bottom-right overlay using `gpt-image-2`, the needs update, or the dashboard for only a few seconds.
11. Mention that time away and task history change cat mood, independence, skills, demeanor, and future portraits.
12. End on the app or talking head with the tagline: "Paws The Scroll: get bullied by a cat into caring for yourself, gently."

## Demo Build Priorities

1. Reliable full-screen overlay trigger.
2. Polished interruption state with a strong cat portrait, funny dialogue, one tiny task, and clear actions.
3. Visible `gpt-image-2` usage through adoption portrait generation or portrait regeneration.
4. Visible `gpt-5.5` usage through generated cat dialogue and a structured task.
5. Immediate completion payoff after clicking `I did it`.
6. Minimal dashboard proof if time allows: mood, needs, tiny actions completed, time-away reward, or independence.

## AI Personalization Talking Point

The AI is not just generating random wellness prompts. `gpt-5.5` receives a compact context packet: my goals, stuck patterns, mobility, environment, boundaries, cat state, recent rerolls, foreground-app category, time of day, and what kinds of tasks I actually complete. Then it generates one tiny task in a strict schema, and the app validates it before showing it.

`gpt-image-2` handles the visual side: initial cat portraits and later portrait regeneration as the cat's mood, independence, and skills evolve.

See [Task Personalization](./task-personalization.md) for the longer product plan.

## Recommended Final Line

Paws The Scroll: get bullied by a cat into caring for yourself, gently.
