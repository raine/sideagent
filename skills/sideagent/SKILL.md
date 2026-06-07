# sideagent

Use `sideagent` when implementation work can be delegated to a configured
coding agent running in a nearby tmux pane.

## Delegate work

Omitting `--profile` uses the default profile from the config.

For short prompts:

```sh
sideagent "implement the requested change"
```

For long prompts or markdown plans:

```sh
cat path/to/plan.md | sideagent
```

The command blocks until the delegated agent writes its done file. When it
returns, read the short summary printed by `sideagent`, inspect the working
tree, and verify the result before reporting success.

## Prompt guidance

Include:

- The goal
- Exact files or plan path when known
- Constraints from `CLAUDE.md`
- Expected checks
- A request for a short final summary

Do not ask the delegated agent to commit unless the user explicitly asked for
that behavior.
