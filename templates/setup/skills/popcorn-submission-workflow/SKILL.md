---
name: {{SKILL_NAME}}
description: Helps prepare and submit popcorn-cli GPU Mode solutions. Use when users ask to set up a project, create a submission template, or run/register submissions.
compatibility: Intended for popcorn-cli repositories with README.md and shell access.
---

# Popcorn Submission Workflow

Use this skill when the user is working on Popcorn CLI submissions and needs a reliable flow from setup to submit.

## Recommended workflow
1. Ensure the project has a `submission.py` file with POPCORN directives.
2. Register once with `popcorn register discord` (or `github`) if `.popcorn.yaml` is missing.
3. Use `popcorn submit submission.py` for interactive mode, or `popcorn submit --no-tui ...` for scripts/CI.
4. Use `popcorn submissions list/show/delete` to inspect previous runs.

## Reference: Authentication (from README)

{{AUTHENTICATION_SECTION}}

## Reference: Commands (from README)

{{COMMANDS_SECTION}}

## Reference: Submission Format (from README)

{{SUBMISSION_FORMAT_SECTION}}

## Guardrails
- Keep submissions as a single Python file.
- Prefer POPCORN directives (`#!POPCORN leaderboard ...`, `#!POPCORN gpu ...`) so defaults are embedded.
- Use `test` or `benchmark` mode before `leaderboard` submissions when iterating.
