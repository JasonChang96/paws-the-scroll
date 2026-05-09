# PRD: Paws The Scroll

## 1. Product Summary

Paws The Scroll is a macOS desktop companion that interrupts prolonged active device use with a needy virtual cat. The cat asks the user to complete tiny behavioral activation tasks, then rewards both task completion and time spent away from the device.

The product is inspired by the experience of how caring for a real cat can interrupt isolation, paralysis, and emotional spirals. The app should feel like a cute pet game on the surface, with behavioral activation and emotional regulation principles underneath.

The hackathon goal is wow factor: a cat that appears over the whole desktop at the right moment, demands care, and turns a stuck device loop into a small physical action.

## 2. Product Positioning

### One-Liner

A desktop cat that interrupts doomscrolling and device paralysis with tiny acts of care, then grows more independent as you spend less time stuck on your computer.

### Product Frame

This is a behavioral activation companion, not a clinical mental health app. It should not claim to diagnose, treat, or monitor mental illness. It should help users notice avoidance loops and take small, concrete actions.

### Target Audience

The first audience is people who want to improve their mental health and emotional regulation but often get stuck in device use, doomscrolling, avoidance, isolation, or paralysis.

The user may be overwhelmed, not lazy. The tone should avoid shame-heavy productivity language.

## 3. Core Principles

1. **Action precedes motivation.**
   The app should ask for actions so small that the user can start before they feel ready.

2. **Care for the cat becomes a bridge to care for the self.**
   The cat's needs should map to tiny user actions: movement, hydration, environment reset, food prep, stretching, grounding, or task initiation.

3. **The app should reward leaving the device.**
   The cat should not only reward in-app engagement. Time away from the computer should create meaningful pet progress.

4. **No permanent punishment.**
   The cat may sulk, complain, or react dramatically, but it should never die, abandon the user, or create shame that makes returning harder.

5. **Local-first privacy.**
   User profile, cat state, task history, and app-usage aggregates should be stored locally by default.

6. **AI should be constrained.**
   OpenAI should generate personalized tasks, dialogue, and story moments, but outputs must follow a schema and pass safety filters.

## 4. Scientific Basis

### Behavioral Activation

Behavioral activation is based on the idea that small actions can precede and improve motivation or mood. The app applies this by interrupting passive device loops with low-friction micro-actions.

### Just-in-Time Adaptive Intervention

The app should support users close to the moment of need by using active device time, app context, recent rerolls, and user goals to choose an intervention.

### Human-Animal Bond

The product uses a simulated caregiving bond. Users may respond more readily to caring for a needy cat than to abstract self-improvement prompts.

### Safety Framing

The app should include clear language that it is not emergency support, not a replacement for care, and not a diagnostic tool. A small crisis resources link should be present in onboarding/settings.

## 5. Platform And Technical Direction

### Platform

macOS desktop app built with Tauri.

### Recommended Stack

- Tauri for desktop shell and native macOS integration.
- React or Next-style frontend inside Tauri.
- TypeScript for UI logic.
- Rust/Tauri commands for macOS activity tracking and window control.
- Local storage using SQLite or Tauri store.
- OpenAI API for live task and dialogue generation.

### API Key

Use `OPENAI_API_KEY` loaded from the local environment or a local settings flow. The prototype should never hard-code secrets.

### Offline Fallback

The app must work when OpenAI is unavailable. Fallback should use a local task library and canned cat dialogue.

## 6. Key User Experience

### Onboarding

Tone should be soft, non-clinical, and non-diagnostic.

Ask about:

- Habit goals.
- Desired life direction.
- Common stuck patterns.
- Emotional regulation tendencies using soft language.
- Mobility or accessibility constraints.
- Available environment: desk, bedroom, office, public place, shared space.
- Preferred interruption intensity.
- Cat tone comfort: gentle, sassy, dramatic, chaotic, or unknown.
- Sensitive task boundaries.

Avoid phrasing like "What mental illness do you have?" Prefer:

- "When overwhelmed, I tend to..."
- "I want tiny nudges that help me..."
- "Tasks I would rather avoid..."
- "My body can usually handle..."

### Cat Adoption

The cat chooses the user, but the user receives three cat options.

Initial suggested choices:

1. **Orange Fat Cat**
   Food-motivated, theatrical, demanding, emotionally obvious.

2. **Void Cat**
   Quiet, mysterious, intense, observant, secretly affectionate.

3. **Scrungly Street Cat**
   Chaotic, scrappy, weirdly loyal, dramatic, unpredictable.

Personality traits should be hinted at, not fully revealed. The cat's personality should develop through interaction history.

### Idle Companion Mode

Before interruptions, the cat should be visibly present but not disruptive.

For V1:

- Cat sits around the menu bar or in a small floating companion window.
- First 15 minutes of active device use should have no interruptions.
- Focus mode can show a happy, quiet cat that keeps the user company without blocking.

### Interruption Mode

After the 15-minute grace period, active device use enters a randomized interruption window.

The app should:

- Detect active use, not idle time.
- Increase interruption likelihood when social media apps or domains are active.
- Show a full-screen blocking overlay on all monitors.
- Show both cat and task on every monitor.
- Require the task to be visible for at least 5 seconds before dismiss/snooze.
- Give the cat a concrete need every time it appears.

Cat needs may include:

- Hungry.
- Bored.
- Lonely.
- Dirty litter.
- Wants play.
- Wants attention.
- Is suspiciously dramatic.
- Has found something cursed-looking and needs help.

### Task Flow

The task should be tiny, personalized, and safe.

User actions:

- "I did it."
- "Reroll."
- "Not right now."
- "This does not work for me."

Rerolls should be mostly unlimited. After 5 rerolls, the app should bias heavily toward easy fallback tasks that require no items, no unusual movement, and no embarrassment.

The user should not get stuck in the interruption for too long.

### Completion Reward

After task completion:

- Cat reacts with satisfaction, purring, smugness, or relief.
- User receives a small pet-care or story reward.
- App suggests stepping away from the computer when appropriate.

### Non-Use Rewards

The app should reward time away from the device.

Short-term rewards:

- Cat brings back a small item.
- Cat naps happily.
- Cat becomes less needy for a while.
- Cat leaves a note or visual change.

Long-term rewards, earned over days or weeks:

- Cat learns to hunt for food.
- Cat learns independent play.
- Cat grooms itself.
- Cat unlocks better food, toys, litter, rooms, or story events.
- Cat becomes more secure and expressive.

## 7. Activity Detection

### Grace Period

The first 15 minutes of active device use should not trigger interruptions.

### Active Use

Active use should be based on signs like:

- Keyboard input.
- Mouse movement.
- Foreground app changes.
- Non-idle device state.

### Idle Time

Idle time should not count toward interruption pressure. It may count toward non-use rewards if the user stays away long enough.

### App Tracking

The app may track foreground app names locally.

For V1, store aggregate stats only:

- Category totals.
- Social media minutes.
- Interruption count.
- Reroll count.
- Completion count.
- Time away after interruption.

Avoid storing raw detailed history by default.

### Social Media Detection

The app should auto-detect social media from both:

- App names.
- Browser domains, where feasible.

Initial social categories may include:

- Instagram.
- TikTok.
- YouTube.
- Reddit.
- X/Twitter.
- Facebook.
- Discord.
- Twitch.
- Browser tabs/domains matching social or feed-based websites.

Browser URL detection may be fragile across Safari, Chrome, Arc, and Firefox. For the hackathon, app-name tracking is the priority, with browser URL support as stretch.

## 8. Focus Mode

Focus mode should be gentle.

For V1, it should:

- Keep a happy cat visible.
- Avoid blocking the screen.
- Avoid complex coaching unless requested.
- Optionally offer tiny next-step suggestions.
- Reward sustained focus without turning into another productivity dashboard.

Focus mode should feel like company, not surveillance.

## 9. AI Features

### Required V1 AI Usage

Use OpenAI for live generation of:

- Cat dialogue.
- Personalized behavioral activation task.
- Reroll adaptation.
- Completion reaction.
- Short story reward.

### AI Context Inputs

The AI should receive:

- User goals.
- Soft stuck-pattern tags.
- Mobility/accessibility constraints.
- Available environment.
- Cat type.
- Known cat personality hints.
- Current cat need.
- Active app/category.
- Time of day.
- Number of rerolls.
- Recent completed/skipped task categories.

### Structured Output

AI output must use a constrained schema.

Example:

```json
{
  "cat_line": "Absolutely tragic. You have been absorbed into the rectangle again.",
  "need": "bored",
  "task": {
    "title": "Wall boop",
    "instruction": "Stand up, touch the nearest wall, and come back.",
    "category": "movement",
    "difficulty": 1,
    "estimated_seconds": 20,
    "requires_items": false,
    "requires_leaving_room": false,
    "mobility_level": "light",
    "fallback_safe": true
  },
  "completion_line": "Fine. Acceptable. I have survived another minute.",
  "safety_notes": []
}
```

### AI Guardrails

Reject or regenerate tasks that:

- Require unsafe movement.
- Involve sharp objects, hot objects, stairs, roads, or going outside unexpectedly.
- Require eating if the user has not opted into food-related tasks.
- Involve body-shaming or calorie-shaming.
- Require social embarrassment.
- Use clinical diagnosis language.
- Increase guilt in a harmful way.
- Are too long or too vague.

### Additional AI Stretch Uses

- Interpret onboarding into goal tags.
- Evolve hidden cat personality.
- Summarize weekly patterns without shame.
- Generate story events after non-use.
- Generate repair moments after repeated dismissal.
- Infer task categories that work best for the user.

## 10. Dashboard

The app should include a dashboard, but it should not feel like a punitive screen-time report.

Dashboard sections:

- Cat state: hunger, boredom, affection, litter, independence.
- Story events.
- Items brought back.
- Learned cat skills.
- Recent task wins.
- Aggregate usage insights.
- Habit direction progress.
- Settings and privacy.

Suggested dashboard framing:

- "Spirals interrupted."
- "Tiny actions completed."
- "Time your cat spent thriving without you."
- "Tasks that helped most."
- "Apps that summoned the cat."

Avoid shame-heavy metrics like "failed days" or "wasted time."

## 11. Safety And Accessibility

### Task Safety

All tasks should be:

- Short.
- Low stakes.
- Easy to reroll.
- Adapted to user constraints.
- Possible in common indoor environments.

### Accessibility

Support:

- Seated alternatives.
- Quiet tasks.
- No-item fallback tasks.
- Low-mobility task mode.
- Public/shared-space-safe tasks.
- "This does not work for me" feedback.

### Crisis Language

The app should include a small note:

"This app is not emergency support or a substitute for professional care. If you might hurt yourself or someone else, contact local emergency services or a crisis line now."

Include crisis resources in settings/onboarding. Keep this accessible but not alarmist.

## 12. Cat Personality And Failure States

### Personality

The cat can be sassy, mean, dramatic, or demanding, but it should still feel fundamentally attached to the user.

The cat may:

- Complain.
- Sulk.
- Flop dramatically.
- Refuse to make eye contact.
- Leave a passive-aggressive note.
- Make a mess.
- Demand easier terms.

The cat should not:

- Die.
- Leave forever.
- Tell the user they are broken.
- Shame the user for symptoms or overwhelm.
- Punish the user in a way that makes return feel hard.

### Dismissal

Dismissals should have random emotional effects.

Sometimes the cat barely cares. Sometimes it sulks. Sometimes it demands a tiny repair task later. This variability creates character, but should not trap the user.

## 13. Hackathon Demo Script

### Story Setup

The presenter explains:

"I built this from a personal experience. When I was overwhelmed and stuck, caring for a real cat changed my life. Feeding it, playing with it, and responding to its needs gave me tiny actions when I could not start anything for myself. This app tries to recreate that bridge: a needy cat that interrupts device paralysis and gets you moving again."

### Demo Flow

1. User completes onboarding with soft answers.
2. Three cats appear. The app says the cat chooses the user.
3. The selected cat sits near the menu bar.
4. Presenter opens a simulated or real social media context.
5. Hidden demo trigger activates the full-screen interruption.
6. Cat blocks all monitors and demands care.
7. AI-generated task appears.
8. User rerolls several times.
9. After 5 rerolls, the app offers a tiny fallback task.
10. User clicks "I did it."
11. Cat reacts and rewards the user.
12. Presenter simulates time away.
13. Cat brings back an item or unlocks a small story event.
14. Dashboard shows aggregate, non-shaming progress.

### Wow Moment

The key wow moment is the cat taking over the desktop and turning a stuck social media loop into a tiny real-world action.

## 14. MVP Scope

### Must Have

- macOS Tauri app.
- Local onboarding.
- Three cat choices.
- Local user profile.
- Local cat state.
- OpenAI live task/dialogue generation.
- Offline fallback task library.
- Menu bar or small floating cat presence.
- 15-minute grace-period logic.
- Active-use timer.
- Social media app-name detection.
- Full-screen overlay interruption.
- Same cat/task on all monitors where feasible.
- 5-second minimum task display.
- Reroll flow.
- Easy fallback after 5 rerolls.
- Completion reward.
- Non-use reward simulation.
- Dashboard with cat state and aggregate stats.
- Settings/privacy page.
- Hidden demo trigger.

### Should Have

- Browser domain detection for major browsers.
- Focus mode with happy quiet cat.
- Story event log.
- Cat item collection.
- Basic personality evolution.
- Crisis resources link.
- Accessibility settings.

### Could Have

- Weekly reflection.
- More cat breeds/types.
- AI-generated cat art variants.
- Multi-day skill learning.
- Notification-style guilt-trip messages.
- Configurable interruption windows.
- User-editable social app list.

### Not In V1

- Cloud accounts.
- Mobile app.
- Clinical diagnosis.
- Emergency intervention.
- Hard OS-level app blocking guarantees.
- Punitive failure states.
- Public leaderboard.

## 15. Data Model Draft

### User Profile

- `id`
- `created_at`
- `goals`
- `stuck_patterns`
- `preferred_tone`
- `mobility_constraints`
- `environment_constraints`
- `task_boundaries`
- `ai_enabled`

### Cat

- `id`
- `type`
- `name`
- `visible_traits`
- `hidden_traits`
- `needs`
- `mood`
- `independence_level`
- `skills`
- `items`
- `story_events`

### Activity Aggregate

- `date`
- `active_minutes`
- `idle_minutes`
- `social_minutes`
- `focus_minutes`
- `interruptions`
- `tasks_completed`
- `rerolls`
- `dismissals`
- `time_away_after_interruptions`

### Task Event

- `id`
- `created_at`
- `source`
- `category`
- `difficulty`
- `app_category`
- `reroll_index`
- `completed`
- `dismissed`
- `marked_inaccessible`

## 16. Open Questions

1. Should V1 feel more like a cute pet game or a mental-health companion? Current recommendation: game-first surface, behavioral activation underneath.

2. Should the cat art be generated by AI for the prototype or use simple placeholder illustrations first?

3. How much should users be allowed to tune interruption intensity?

4. Should browser URL detection be included in the hackathon demo or treated as stretch?

5. Should dashboard insights mention specific apps by name or only categories?

6. Should AI story events be generated live or from a constrained local template set?

7. Should the app ask users to name the cat, or should the cat arrive with a name?

8. Should the cat's sass level be editable after onboarding?

## 17. Build Recommendation

Build a thin vertical slice first:

1. Onboarding.
2. Cat selection.
3. Menu bar/floating companion.
4. Hidden demo trigger.
5. Full-screen overlay.
6. OpenAI-generated task and cat line.
7. Rerolls with fallback after 5.
8. Completion reward.
9. Simulated non-use reward.
10. Dashboard with cat state and aggregate stats.

After that, add real active-use tracking, social app detection, browser domain detection, and multi-monitor polish.

This sequence protects the hackathon demo while still moving toward the real macOS product.
