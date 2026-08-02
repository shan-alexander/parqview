---
tags: [rustbrain]
node_type: concept
---
# How to use rustbrain

Gotta install it. if already installed, upgrade.



## Upgrade the CLI

install latest published CLI (0.3.5): 
`cargo install rustbrain --version 0.3.5 --locked --force`

if cargo installs aren't on PATH in this shell: 
`export PATH="$HOME/.cargo/bin:$PATH"`

`rustbrain --version`   # should print: rustbrain 0.3.5

`--force` replaces the existing binary under `~/.cargo/bin/rustbrain`.

## refresh the brain with a project/repo

`cd /home/path/to/project/repo/`

re-index with the new binary
`rustbrain sync`

then: 
`rustbrain doctor`

## Create a new note 

A new note can be any type. Starting with a goal is best, a random note is probably a concept.

```
rustbrain note new --type "goal" --title "Rustbrain fluency" --body "Prefer rustbrain context/query before large refactors. Capture decisions with note new --type `adr`. Run sync after doc/code changes. Keep docs truthful — do not invent ADR history. When new helpful info is collected but unsure of which type, use the `concept` type."
```

This creates a new .md in the `docs/goals/` dir with the title `rustbrain-fluency.md`.
