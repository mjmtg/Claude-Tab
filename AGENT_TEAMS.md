# Agent Teams Best Practices

## Task Design

- Size tasks as self-contained units with a clear deliverable (a function, test file, or review)
- Aim for 5-6 tasks per teammate to keep everyone productive
- Break work so each teammate owns a different set of files — avoid two teammates editing the same file

## Spawning Teammates

- Include task-specific details in spawn prompts — teammates don't inherit the lead's conversation history
- Specify file paths, tech stack details, and focus areas explicitly
- Use `require plan approval` for risky or complex tasks so teammates plan before implementing
- Pre-approve common permissions before spawning to reduce prompt interruptions

## Coordination

- Use **delegate mode** (Shift+Tab) to keep the lead focused on orchestration, not implementation
- Tell the lead to wait for teammates before proceeding if it starts doing work itself
- Monitor progress and redirect approaches that aren't working — don't let teams run unattended too long
- Use hooks (`TeammateIdle`, `TaskCompleted`) to enforce quality gates

## When to Use Teams vs Subagents

- **Teams**: work requiring discussion, collaboration, or cross-layer coordination
- **Subagents**: focused tasks where only the result matters, lower token cost

## Cleanup

- Always clean up via the lead, never from a teammate
- Shut down all teammates before running cleanup
