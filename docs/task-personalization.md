# Task Personalization

## Goal

Paws The Scroll should generate tasks that feel dynamic, personal, and funny without becoming unsafe or random. OpenAI should provide the cat's flavor, dialogue, portrait, and task wording; the app should own the constraints, safety checks, local history, and adaptation loop.

## Personalization Inputs

The app can tailor tasks from data it already collects locally:

- User goals: doomscroll less, move more, drink water, tidy space, start avoided tasks, or get unstuck.
- Stuck patterns: doomscrolling, paralysis, isolation, avoidance, or overwhelm.
- Cat tone: gentle, sassy, dramatic, chaotic, or unknown.
- Mobility constraints: seated-only through higher-movement tasks.
- Environment constraints: desk, bedroom, office, public space, or shared space.
- Task boundaries: no food, no loud movement, no leaving the room, no outside, or nothing socially embarrassing.
- Free-form onboarding notes: the user's own wording about goals, stuck patterns, tone, mobility, environment, and boundaries.
- Cat state: needs, mood, visible traits, hidden traits, skills, and independence level.
- Current context: foreground app category, time of day, reroll count, and whether fallback mode is needed.
- Recent behavior: completed categories, dismissed categories, rerolls, inaccessible tasks, and time away after interruptions.

All of this should remain local by default. The OpenAI request should receive only the compact context needed to generate the next task.

## How Dynamic Tasks Can Be

Tasks can adapt across several dimensions:

| Dimension | Example adaptation |
| --- | --- |
| Goal | "Doomscroll less" biases toward screen-breaking movement, grounding, and app-exit tasks. |
| Stuck pattern | Paralysis biases toward tiny task-init actions; overwhelm biases toward grounding and narrowing tasks. |
| Environment | Office and public settings avoid embarrassing movement; bedroom and desk settings can use small reset tasks. |
| Mobility | Low mobility keeps tasks seated; higher mobility can include short walks or larger stretches. |
| Boundaries | Food, outside, loud movement, and leaving-room tasks are excluded when the user opts out. |
| Cat state | Hungry, bored, lonely, dirty-litter, play, and attention needs map to different task categories. |
| Rerolls | Each reroll should become materially easier or different; after several rerolls, use fallback-safe tasks. |
| Completion history | Categories the user completes can appear more often; categories repeatedly dismissed should cool down. |
| Time of day | Late night should lower intensity; morning can bias toward gentle activation. |
| Active app category | Social/feed contexts can bias toward interrupting passive loops; work contexts can bias toward task initiation. |

## Task Categories

The task category should stay explicit and validated:

- `movement`: stand, walk briefly, change posture, or do a low-risk movement.
- `hydration`: take a sip of water if available and not boundary-conflicting.
- `environment`: clear one item, reset a small surface, or improve the immediate space.
- `food`: prepare or notice food only if allowed by boundaries.
- `stretching`: small, low-risk mobility action.
- `grounding`: sensory orientation or breath-based reset without clinical framing.
- `task_init`: open the avoided thing, write one word, name the next step, or start a short timer.

## Adaptation Loop

1. Build a compact `TaskContext` before every interruption.
2. Include profile goals, notes, boundaries, cat state, current app category, time of day, reroll count, and recent task history.
3. Ask OpenAI for one structured `GeneratedTaskBundle`.
4. Validate the generated task locally:
   - category is known
   - difficulty is within range
   - estimated duration is short enough
   - mobility level does not exceed the user's constraint
   - task does not violate food, movement, outside, room-leaving, or embarrassment boundaries
   - fallback mode produces no-item, no-embarrassment, low-mobility tasks
5. Show the task with cat dialogue.
6. Record what happened: completed, rerolled, dismissed, or marked inaccessible.
7. Update local preference stats.
8. Use those stats in the next `TaskContext`.

This gives the product dynamic AI behavior while preserving predictable safety and product judgment.

## Local Preference Stats

Add a lightweight derived profile that summarizes task fit over time:

```ts
interface TaskPreferenceStats {
  category: TaskCategory;
  offered_count: number;
  completed_count: number;
  dismissed_count: number;
  rerolled_count: number;
  inaccessible_count: number;
  average_completion_difficulty: number;
  last_offered_at: string | null;
  last_completed_at: string | null;
}
```

Suggested rules:

- Increase category weight when completion rate is high.
- Temporarily reduce category weight when reroll or dismissal rate is high.
- Strongly reduce similar tasks when the user marks one inaccessible.
- Cool down recently repeated categories so the cat does not feel stale.
- Lower difficulty after repeated dismissals or late-night interruptions.
- Keep successful tasks small; do not escalate into productivity pressure.

## Interests

Goals are already captured, but interests can make tasks feel more personal. Add an optional free-form field:

```ts
interests_notes: string;
```

Use interests as flavor, not as the core intervention logic.

Examples:

- Music: "Play one song and stand up for the first chorus."
- Plants: "Check on one plant like it is the cat's legal witness."
- Cooking: "Put one cup or plate where it belongs."
- Cozy games: "Do one tiny inventory-management action on your desk."
- Absurd humor: "Human. You have become furniture. Stand up for ten seconds so I know you are alive."

## Cat Individualization

The cat should not stay generic. As the user progresses, the cat should become more individualized: an "owner like pet" loop where the cat's mood, appearance, and demeanor reflect the user's actual behavior patterns.

The goal is not to punish the user visually. The cat should mirror rhythms and preferences in a playful, affectionate way.

### Individualization Inputs

Use local history to evolve the cat:

- Most completed task categories.
- Most rerolled or dismissed task categories.
- Stuck patterns the user selected.
- User goals and interests.
- Preferred cat tone.
- Time-away streaks and completion streaks.
- Typical active-use windows, such as late night or afternoon.
- Boundaries and accessibility constraints.
- Cat needs that are most often addressed or neglected.

### Demeanor Evolution

The cat's personality should shift subtly over time:

| User pattern | Cat evolution |
| --- | --- |
| Completes grounding tasks often | Cat becomes calmer, more observant, and quietly smug. |
| Completes movement tasks often | Cat becomes more playful, springy, and physically expressive. |
| Completes environment-reset tasks often | Cat becomes tidier, fussier, and more domestic. |
| Often rerolls hard tasks | Cat learns to offer easier tasks faster and becomes less pushy. |
| Often dismisses interruptions late at night | Cat becomes sleepier and asks for softer, lower-intensity tasks at night. |
| Strong time-away streaks | Cat becomes more independent, secure, and self-entertaining. |
| Isolation pattern plus completed connection tasks | Cat becomes more affectionate and socially brave. |
| Chaotic tone plus frequent completions | Cat becomes more theatrically loyal and weirdly proud. |

### Appearance Evolution

Portrait generation should include a compact cat-evolution profile. The profile can influence visual details without making the cat look harmed or neglected.

Examples:

- A task-init user might get a cat with little desk-side trophies, paper scraps, or a focused stare.
- A movement user might get a stretchier pose, brighter eyes, or toy-like accessories.
- A grounding user might get softer lighting, calmer posture, or cozy resting poses.
- A tidy-space user might get a neater blanket, arranged toys, or a smugly organized perch.
- A late-night user might get moonlit colors, sleepy eyes, or a blanket-nest look.
- A high time-away user might get self-sufficient cues: a tiny self-caught morsel, a groomed coat, or independent-play toys.

Avoid visual shame states. The cat can be dramatic, sulky, rumpled, or demanding, but should never look abandoned, sick, injured, or permanently worse because the user struggled.

### Cat Evolution Profile

Add a derived local profile that summarizes how this specific cat has adapted:

```ts
interface CatEvolutionProfile {
  dominant_task_affinity: TaskCategory | null;
  avoided_task_affinities: TaskCategory[];
  demeanor_traits: string[];
  appearance_cues: string[];
  favorite_task_flavors: string[];
  learned_user_rhythms: string[];
  last_updated_at: string;
}
```

This profile should be derived from task history, aggregates, and user profile data. It should be sent to OpenAI for dialogue and portrait generation as compact context, not as raw behavioral logs.

### Owner-Like-Pet Loop

The product should make the cat feel shaped by the relationship:

1. The user picks goals, boundaries, tone, and a cat.
2. The cat starts with a broad personality.
3. The user completes, rerolls, dismisses, and marks tasks inaccessible.
4. The app derives task preferences and cat evolution cues.
5. The cat's future tasks, dialogue, mood, and portraits reflect those cues.
6. The user sees a pet that feels increasingly specific to them.

This should be presented as affection and adaptation, not surveillance. The cat learns what helps; it does not judge what the user failed to do.

## Demo Explanation

For judges, describe the AI loop this way:

> The AI is not just generating random wellness prompts. It gets a local context packet: my goals, stuck patterns, mobility, environment, boundaries, cat state, recent rerolls, and what kinds of tasks I actually complete. Then it generates one tiny task in a strict schema, and the app validates it before showing it.

## Safety Notes

- Never generate diagnosis, treatment claims, crisis handling, or shame-heavy language.
- Never make the cat die, abandon the user, or create permanent punishment.
- Treat rerolls and dismissals as personalization data, not failure.
- Keep raw app history out of the personalization loop unless the user explicitly opts in.
- Store task history and aggregate stats locally by default.
