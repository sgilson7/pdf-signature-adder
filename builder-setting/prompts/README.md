# Prompts

| File | Stage | Use |
| --- | --- | --- |
| [`start-from-description.md`](start-from-description.md) | 1–9 | a person who does not write code describes the tool in prose; the agent writes the brief, builds, and publishes if allowed |
| [`start-from-brief.md`](start-from-brief.md) | 1–2 | turn a written brief into a plan, and stop |
| [`review-plan.md`](review-plan.md) | 3 | read a plan against the brief before any code |
| [`audit-boundary.md`](audit-boundary.md) | before 8 | find a rule that leaked out of the core, and hard-coded strings |
| [`prompt-set-template.md`](prompt-set-template.md) | 1–9 | write a complete prompt set for a fresh session |
| [`sets/C-feedback-drafts.md`](sets/C-feedback-drafts.md) | 1–9 | a prompt set from a teacher request, run once from `template/` |
