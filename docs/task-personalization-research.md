# Task Personalization Research for Paws The Scroll

This note translates behavior-change research into a practical task-selection model for Paws The Scroll. The goal is to help the app choose tiny, safe tasks that fit a user's goals, interests, current constraints, and preferred cat personality.

The evidence is strongest for matching tasks to goals, context, autonomy, and ability. Evidence for matching to broad personality types is useful but more indirect. Personality should shape tone, framing, novelty, and task flavor; it should not override safety, boundaries, or user-stated goals.

## Design Principle

The task selector should answer five questions in order:

1. **What is the user trying to move toward?** Use onboarding goals and notes.
2. **What stuck pattern is happening?** Doomscrolling, paralysis, isolation, avoidance, or overwhelm.
3. **What can the user safely do right now?** Use mobility, environment, time, and hard boundaries.
4. **What style will make the task feel acceptable?** Use tone, cat type, and interest/personality hints.
5. **What is the smallest complete action?** Prefer a task the user can finish in 10-90 seconds.

## Research Anchors

### Behavior Change Techniques

The Behaviour Change Technique Taxonomy is useful because it names the intervention ingredients Paws is already using: prompts/cues, action planning, graded tasks, feedback, reward, behavior substitution, self-monitoring, and restructuring the physical environment.

Relevant sources:

- [Behaviour Change Technique labels and definitions, NCBI Bookshelf](https://www.ncbi.nlm.nih.gov/books/NBK567039/)
- [Knittle et al., 2018, physical activity motivation meta-analysis](https://pubmed.ncbi.nlm.nih.gov/29385950/)
- [Spring et al., 2021, self-regulatory behavior-change meta-review](https://pmc.ncbi.nlm.nih.gov/articles/PMC7429262/)

Product implication:

- Most tasks should combine **prompt/cue + tiny graded task + immediate reward**.
- Avoid vague advice. "Take care of yourself" is not a task. "Put one cup by the sink" is a task.

### Autonomy, Competence, and Relatedness

Self-determination theory suggests behavior change is more durable when people feel choice, capability, and connection. Paws can support this by offering rerolls, easy tasks, cat companionship, and non-shaming feedback.

Relevant sources:

- [Patrick and Williams, 2012, self-determination theory and health behavior](https://pubmed.ncbi.nlm.nih.gov/22385676/)
- [Gillison et al., 2019, meta-analysis of SDT techniques for health behavior](https://pubmed.ncbi.nlm.nih.gov/30295176/)

Product implication:

- Rerolls are a feature, not a loophole.
- Tasks should make the user feel capable quickly.
- The cat can provide relatedness: "I am here with you," not "you failed again."

### Goal Concordance and Values

People persist more when goals feel self-concordant: aligned with their interests, values, and own reasons, not just guilt or external pressure.

Relevant sources:

- [Sheldon and Elliot, 1999, self-concordance model](https://cir.nii.ac.jp/crid/1363951794098173312)
- [Sheldon, 2014, self-concordant goal selection](https://pubmed.ncbi.nlm.nih.gov/24981515/)
- [Kanter et al., 2006, behavioral activation and ACT comparison](https://pmc.ncbi.nlm.nih.gov/articles/PMC2223147/)
- [Bramwell and Richardson, 2018, values-based action in ACT](https://pubmed.ncbi.nlm.nih.gov/29515268/)

Product implication:

- Use interest notes to change task flavor. A creative user and a systems user may both need "task initiation," but the task should look different.
- Ask "what would feel good to do more of?" rather than "what should you fix?"

### Computer Tailoring

Computer-tailored interventions can improve fit by adapting messages to user characteristics and context. The evidence is better for health behavior than for this exact mental-health-adjacent use case, but it supports using onboarding data rather than one generic task list.

Relevant sources:

- [Neville et al., 2009, computer-tailored physical activity interventions](https://link.springer.com/article/10.1186/1479-5868-6-30)
- [Neville et al., 2009, computer-tailored dietary behavior interventions](https://pmc.ncbi.nlm.nih.gov/articles/PMC2706490/)
- [Latimer et al., 2010, physical activity message construction review](https://link.springer.com/article/10.1186/1479-5868-7-36)

Product implication:

- Tailoring should be concrete: task category, difficulty, wording, environment, and reward.
- Avoid over-personalizing from weak signals. User-stated boundaries beat inferred personality every time.

### Personality and Behavior

Personality traits are associated with health behaviors and activity patterns, but using them for deterministic task assignment would be too strong. For Paws, personality is best treated as a light personalization layer.

Relevant sources:

- [Rhodes and Smith, 2006, personality correlates of physical activity meta-analysis](https://pubmed.ncbi.nlm.nih.gov/16606453/)
- [Sutin et al., 2016, Big Five personality and health behaviors](https://pmc.ncbi.nlm.nih.gov/articles/PMC6266523/)
- [Hudson et al., 2024, volitional personality change systematic review](https://pmc.ncbi.nlm.nih.gov/articles/PMC11608366/)

Product implication:

- Do not say "you are a neurotic user, so..."
- Use softer operational labels: novelty-seeking, structure-seeking, socially energized, privacy-seeking, sensory-friendly, comfort-seeking, achievement-oriented, play-oriented.

## Goal-to-Task Map

### Goal: Get Unstuck More Easily

Best task types:

- **Task initiation:** open the file, write one bad sentence, put the cursor where the next action begins.
- **Grounding:** name three objects, press feet into the floor, breathe out longer than in.
- **Environment reset:** remove one blocker from the desk.

Why it fits:

- Behavioral activation works by lowering the activation energy of the first step.
- Graded tasks and action planning are well-supported behavior-change ingredients.

Example tasks:

- "Open the thing you are avoiding. Do not work on it yet. Just let it exist on screen."
- "Write one intentionally messy sentence. The cat accepts terrible drafts."
- "Move one object that is physically between you and the next task."

Avoid:

- Multi-step planning when the user is in paralysis.
- Big productivity language.
- "Finish" tasks.

### Goal: Doomscroll Less

Best task types:

- **Behavior substitution:** replace scrolling with a tiny offline action.
- **Movement break:** stand, stretch, walk to the door and back.
- **Attention reset:** look away from the screen and identify a non-screen object.
- **Exit ritual:** close one tab, lock the screen, place phone/computer aside for 60 seconds.

Why it fits:

- Passive or problematic social media use is associated with worse well-being for some users.
- JITAI-style timing is plausible when the app detects prolonged active use or social app context.

Example tasks:

- "Look at the farthest corner of the room and name one real object. The rectangle can wait."
- "Close one feed tab. Leave the useful tab open if there is one."
- "Stand up, touch the nearest wall, and come back."

Avoid:

- Moralizing app use.
- "Never use social media" framing.
- Long reflection prompts while the feed is still open.

### Goal: Move My Body During The Day

Best task types:

- **Movement:** stand, step, shoulder roll, short walk.
- **Stretching:** wrist, neck, shoulder, back.
- **Play:** cat toy metaphor, tiny chase, wall tap.

Why it fits:

- Physical activity interventions often benefit from behavioral goal setting, self-monitoring, rehearsal, and graded tasks.
- Brief movement breaks are low-cost and can interrupt sedentary loops.

Example tasks:

- Low mobility: "Roll your shoulders three times, slowly."
- Light mobility: "Stand up and sit back down once."
- Moderate/high mobility: "Walk to the nearest doorway and back."

Avoid:

- Exercise language if the user opted out in notes.
- Tasks that require stairs, going outside, equipment, sweating, or social visibility.

### Goal: Drink More Water

Best task types:

- **Hydration:** sip, refill, place water nearby.
- **Environment setup:** put cup within reach.
- **Sensory reset:** notice temperature, swallow, pause.

Why it fits:

- Hydration can affect mood and cognition, but tasks should remain optional and boundary-aware.
- The main behavior-change ingredient is making the desired behavior easier in the current environment.

Example tasks:

- "Take one sip if water is already nearby."
- "Put a cup where future-you can reach it."
- "If you have no drink nearby, just notice whether getting one would be possible. That counts as data."

Avoid:

- Hydration pressure in public, during illness, or when the user says no food/drink tasks.
- Claims that water will fix mood.

### Goal: Tidy My Space

Best task types:

- **Environment reset:** one object, one dish, one piece of trash.
- **Boundary clearing:** make the smallest visible area better.
- **Completion cue:** before/after cat approval.

Why it fits:

- Restructuring the physical environment is a recognized behavior-change technique.
- A visible environmental win creates immediate reinforcement.

Example tasks:

- "Move one object to a better place. Not the whole room. One object."
- "Throw away one obvious piece of trash."
- "Clear a cat-sized square of desk."

Avoid:

- Full-room cleaning.
- Shame language about mess.
- Tasks that require leaving the room if the user opted out.

### Goal: Start Tasks I Have Been Avoiding

Best task types:

- **Task initiation:** open, name, outline, first mark.
- **Friction reduction:** find the document, plug in charger, place needed object nearby.
- **Two-minute bridge:** start a timer, do the first visible step.

Why it fits:

- Avoidance is a central target of behavioral activation.
- Implementation-intention style prompts help turn vague intentions into concrete behavior.

Example tasks:

- "Open the avoided task and rename it something boring."
- "Write the next action as a fragment, not a plan."
- "Set a two-minute timer and touch only the first step."

Avoid:

- Asking for emotional analysis before action.
- "Just do it" framing.
- Anything that requires completion.

### Goal: Feel Less Isolated

Best task types:

- **Low-stakes connection:** react to a message, send a neutral check-in, choose one person.
- **Parallel presence:** sit near another person, move to shared space, open a co-working room.
- **Cat-mediated relatedness:** care for cat first, then optional human contact.

Why it fits:

- Relatedness is a core need in self-determination theory.
- Isolation tasks must be consent-based and embarrassment-safe.

Example tasks:

- "Think of one person who would not mind a tiny message. You do not have to send it yet."
- "Send a dot, heart, or 'thinking of you' to someone safe."
- "Move one step closer to where people exist, if that feels okay."

Avoid:

- Forced calls.
- Contacting unsafe people.
- Social tasks in public/shared settings if the user marked no social embarrassment.

### Goal: Calm Overwhelm

Best task types:

- **Grounding:** sensory naming, feet on floor, unclench jaw.
- **Breath-adjacent but not breath-dependent:** slow exhale, pause, look around.
- **Choice reduction:** pick one of two tiny actions.

Why it fits:

- Mindfulness and grounding-style exercises can reduce acute distress for some users.
- Overwhelm often needs lower cognitive load, not more options.

Example tasks:

- "Find three blue things. The cat will wait."
- "Press both feet into the floor for five seconds."
- "Unclench your jaw and drop your shoulders once."

Avoid:

- Complex meditation instructions.
- Trauma-heavy body scans.
- Long journaling prompts unless the user asks for them.

## Interest-to-Task Flavor Map

Interests should change task flavor, not safety rules.

| Interest signal | Use these task flavors | Example |
| --- | --- | --- |
| Creative / art / writing | Doodle, color, one sentence, visual noticing | "Draw one terrible circle on paper or in the air." |
| Analytical / systems | Sort, label, classify, pick next step | "Name the next action as a verb plus noun." |
| Music / audio | One song cue, volume reset, listen for a sound | "Play or imagine the first five seconds of a song that changes the room." |
| Cozy / sensory | Texture, warmth, lighting, comfort object | "Touch the nearest soft thing for five seconds." |
| Games / playful | Tiny quest, speedrun, score one point | "Speedrun: move one object before the cat blinks." |
| Nature | Look outside, plant care, sky/weather noticing | "Find one non-screen natural thing: plant, sky, wood, water, light." |
| Social / community | Send a small signal, shared-room action | "React to one message without starting a full conversation." |
| Learning | Open one source, define one term, bookmark one useful thing | "Write one question you want the answer to." |
| Caregiving | Water plant, care for pet, prep future self | "Do one tiny thing that makes future-you easier to care for." |
| Order / aesthetics | Align, clear, arrange, reset | "Straighten one object until the cat grudgingly approves." |

## Personality-Style Task Matching

These are product-facing styles, not diagnoses or fixed traits.

### Novelty-Seeking

Likely preference:

- Variety, weird prompts, playful language, creative substitutions.

Good tasks:

- "Find the strangest harmless object within reach."
- "Do a tiny side quest: touch something red, then return."
- "Invent a one-word title for the current mess."

Avoid:

- Repeating the exact same hydration or standing task too often.

### Structure-Seeking

Likely preference:

- Clear steps, predictable categories, visible progress, low ambiguity.

Good tasks:

- "Step 1: open the document. Step 2: type one word. Stop."
- "Move one item from desk to its home."
- "Pick the next task category: movement or environment."

Avoid:

- Random surreal prompts.
- Ambiguous tasks like "reset your vibe."

### Socially Energized

Likely preference:

- Connection, shared presence, messages, accountability, cat commentary with warmth.

Good tasks:

- "Send one low-stakes emoji to someone safe."
- "Move to a room where another human exists, if available."
- "Tell the cat, out loud or silently, what you are doing next."

Avoid:

- Social tasks when in public/shared environments and no-embarrassment boundary is active.

### Privacy-Seeking

Likely preference:

- Solo, quiet, invisible, non-performative tasks.

Good tasks:

- "Press your feet into the floor. No one can tell."
- "Close one tab silently."
- "Move one private object into place."

Avoid:

- Calling, texting, speaking aloud, visible movement in public.

### Sensory-Sensitive

Likely preference:

- Low-noise, low-light, low-intensity, predictable body tasks.

Good tasks:

- "Lower screen brightness one notch if that feels better."
- "Unclench your hands."
- "Look at a still object for five seconds."

Avoid:

- Loud movement, jumping, abrupt breathing instructions, rapid sensory switching.

### Achievement-Oriented

Likely preference:

- Clear wins, streaks, progress language, "one point" framing.

Good tasks:

- "Score one point: write the next action."
- "Clear one item and claim the smallest possible win."
- "Finish a 30-second sprint, then stop."

Avoid:

- Turning every task into productivity pressure.
- Punitive streak loss.

### Comfort-Seeking

Likely preference:

- Gentle warmth, reassurance, co-regulation, low demand.

Good tasks:

- "Put one hand somewhere comfortable and exhale once."
- "Make the room 1 percent kinder: blanket, light, water, posture."
- "Let the cat ask for the easiest possible version."

Avoid:

- Sarcasm, urgency, competitive framing.

### Play-Oriented

Likely preference:

- Quests, randomness, cat drama, silly but safe movement.

Good tasks:

- "The cat demands a wall boop. Touch the nearest wall."
- "Find a cursed object and move it one inch."
- "Do one tiny victory pose if no one can see, or imagine it if they can."

Avoid:

- Tasks that become socially embarrassing.

## Cat Tone as Task Framing

The same task can be rendered differently without changing the safe behavior.

Task: take one sip of water.

- **Gentle:** "Take one sip if water is nearby. The cat will wait."
- **Sassy:** "The cat has inspected your hydration strategy and filed a complaint. One sip."
- **Dramatic:** "The cat is perishing theatrically beside the cup. One sip may save the realm."
- **Chaotic:** "Tiny quest: sip the water before the cat invents a new problem."
- **Unknown:** Rotate gently, avoiding extremes until the user shows preference.

Tone rules:

- Gentle should minimize pressure.
- Sassy should tease the situation, not the user.
- Dramatic should exaggerate cat stakes, not user failure.
- Chaotic should create novelty without adding complexity.

## Environment and Mobility Constraints

### Environment

| Environment | Best tasks | Avoid |
| --- | --- | --- |
| Desk at home | movement, hydration, environment, task initiation | unsafe equipment, long chores |
| Bedroom | grounding, comfort, gentle tidying, water if nearby | shame around bed, loud tasks |
| Office | subtle stretches, screen breaks, desk reset | embarrassing movement, speaking aloud |
| Public | invisible grounding, posture, quiet attention shifts | food, calls, visible exercises |
| Shared space | quiet movement, small object reset, optional connection | social pressure, loud tasks |

### Mobility

| Mobility setting | Allowed baseline |
| --- | --- |
| Low | seated only, no standing required |
| Light | short stand-up or one-step movement |
| Moderate | short walk inside current room/building |
| High | reasonable movement, still no risky tasks |

Hard boundaries always win:

- `no_food`: no eating or food-prep tasks; hydration should also be cautious unless explicitly allowed.
- `no_loud_movement`: no jumping, stomping, dancing, loud object movement.
- `no_leaving_room`: all tasks must fit the current room.
- `no_outside`: no outdoor tasks or fresh-air errands.
- `no_social_embarrassment`: no visible, verbal, or socially risky tasks.

## Suggested Task-Archetype Library

Each archetype can have variants by goal, interest, tone, environment, and mobility.

| Archetype | Category | Best for | Safe default |
| --- | --- | --- | --- |
| Wall boop | movement | doomscroll, play, paralysis | Touch nearest wall and return. |
| One-object reset | environment | tidy, overwhelm, avoidance | Move one object to a better place. |
| Cursor landing | task_init | avoidance, task start | Open the work and place cursor. |
| Bad first sentence | task_init | writing, avoidance | Write one intentionally bad sentence. |
| Three-color scan | grounding | overwhelm, public, low mobility | Name three colors nearby. |
| Feet press | grounding | overwhelm, low mobility | Press feet into floor for five seconds. |
| Cup within reach | hydration | water, desk/home | Put water within reach or take one sip. |
| Shoulder unlock | stretching | movement, desk, low mobility | Roll shoulders three times. |
| Tab release | environment/task_init | doomscroll, browser loops | Close one nonessential tab. |
| One-message signal | grounding/social | isolation | Send or draft one safe tiny message. |
| Cat-sized square | environment | tidy, achievement | Clear a small square of surface. |
| Doorway lap | movement | movement, doomscroll | Walk to doorway and back. |
| Object naming | grounding | anxiety, overwhelm | Name one object and one property. |
| Future-self setup | environment/task_init | avoidance, caregiving | Place one needed item where future-you can use it. |
| Tiny sensory comfort | grounding | overwhelm, comfort | Touch a soft/cool/stable object for five seconds. |

## Selection Heuristic

Use a weighted selector rather than a rigid decision tree.

Suggested weights:

- +4 if task category directly matches a selected goal.
- +3 if task directly counters current stuck pattern.
- +3 if task fits current environment.
- +3 if task fits mobility setting.
- +4 if task avoids all hard boundaries.
- +2 if task matches interest notes.
- +1 if task matches preferred tone or cat personality.
- -5 if task resembles a recently dismissed category.
- -3 if same archetype appeared recently.
- +3 if reroll index is 5 or higher and task is fallback-safe.

Fallback rule:

- After 5 rerolls, only choose tasks that are no-item, no-embarrassment, low-mobility, indoors, and under 60 seconds.

## Example Personalization Profiles

### User A: Doomscroll Less + Sassy + Office + No Social Embarrassment

Best tasks:

- Close one feed tab.
- Look away and name three non-screen objects.
- Subtle wrist stretch.

Sample cat line:

- "The cat has audited your feed and found zero mice. Close one tab."

### User B: Start Avoided Tasks + Gentle + Bedroom + Low Mobility

Best tasks:

- Open the file.
- Write one bad sentence.
- Press feet into floor before choosing one next action.

Sample cat line:

- "No finishing. Just open the thing and let the cat sit next to it."

### User C: Move Body + Chaotic + Home Desk + Play-Oriented

Best tasks:

- Wall boop.
- Doorway lap.
- Tiny victory pose.

Sample cat line:

- "Emergency side quest: boop the wall and return before the cat becomes furniture."

### User D: Tidy Space + Dramatic + Shared Space + No Loud Movement

Best tasks:

- Move one object silently.
- Clear a cat-sized square.
- Align one item.

Sample cat line:

- "A single object is ruining the kingdom. Relocate it quietly."

## Implementation Notes

- Keep the user's explicit notes available to both AI prompting and local fallback selection.
- Store failed tasks by archetype as well as category; "movement" may fail because the specific task was visible or loud, not because movement is always wrong.
- Treat "This does not work for me" as a stronger negative signal than "Not right now."
- Prefer task templates with parameter slots over fully handwritten one-offs.
- Add a `fallback_safe` flag to every local task template.
- Add tags for `quiet`, `public_safe`, `seated`, `no_items`, `social`, `creative`, `sensory`, `task_start`, and `screen_exit`.

## Product Guardrails

1. Safety and boundaries beat personalization.
2. The smallest useful action is usually better than the most relevant ambitious action.
3. The app should not infer clinical traits or label users by personality.
4. Personality matching should change framing and variety more than core task selection.
5. Interests should make tasks feel more self-concordant, not create extra effort.
6. Rerolls should converge toward easier, safer, less socially visible tasks.
7. The task library should make offline mode useful without pretending to be therapy.

## Citation List

- [Behaviour Change Technique labels and definitions, NCBI Bookshelf](https://www.ncbi.nlm.nih.gov/books/NBK567039/)
- [Knittle et al., 2018, How Can Interventions Increase Motivation for Physical Activity?](https://pubmed.ncbi.nlm.nih.gov/29385950/)
- [Spring et al., 2021, Self-Regulatory Behaviour Change Techniques Meta-Review](https://pmc.ncbi.nlm.nih.gov/articles/PMC7429262/)
- [Patrick and Williams, 2012, Self-Determination Theory and Health Behavior](https://pubmed.ncbi.nlm.nih.gov/22385676/)
- [Gillison et al., 2019, SDT Techniques Meta-Analysis](https://pubmed.ncbi.nlm.nih.gov/30295176/)
- [Sheldon and Elliot, 1999, The Self-Concordance Model](https://cir.nii.ac.jp/crid/1363951794098173312)
- [Sheldon, 2014, Self-Concordant Goal Selection](https://pubmed.ncbi.nlm.nih.gov/24981515/)
- [Kanter et al., 2006, ACT and Behavioral Activation](https://pmc.ncbi.nlm.nih.gov/articles/PMC2223147/)
- [Bramwell and Richardson, 2018, Values-Based Action](https://pubmed.ncbi.nlm.nih.gov/29515268/)
- [Neville et al., 2009, Computer-Tailored Physical Activity Interventions](https://link.springer.com/article/10.1186/1479-5868-6-30)
- [Neville et al., 2009, Computer-Tailored Dietary Behaviour Change Interventions](https://pmc.ncbi.nlm.nih.gov/articles/PMC2706490/)
- [Latimer et al., 2010, Physical Activity Message Construction Review](https://link.springer.com/article/10.1186/1479-5868-7-36)
- [Rhodes and Smith, 2006, Personality Correlates of Physical Activity](https://pubmed.ncbi.nlm.nih.gov/16606453/)
- [Sutin et al., 2016, Big Five Personality and Health Behaviors](https://pmc.ncbi.nlm.nih.gov/articles/PMC6266523/)
- [Hudson et al., 2024, Volitional Personality Change Systematic Review](https://pmc.ncbi.nlm.nih.gov/articles/PMC11608366/)
