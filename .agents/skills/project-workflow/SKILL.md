---
name: project-workflow
description: "Use before making changes in a repository that has adopted this skill, or when the user explicitly requests it. Follow the ticket, gwt worktree, verification, pull request, merge, and cleanup workflow. Skip read-only tasks."
---

# Project workflow

Use this workflow when a repository adopts the skill or the user explicitly requests it. Read the repository's `AGENTS.md` and `CLAUDE.md`; repository-specific instructions and the user's explicit directions take precedence. Skip the workflow only for read-only tasks or when the owner says so.

## Every change: ticket → worktree → verify → PR → merge → clean up

1. **Ticket.** Find the repository's Linear project, create an issue there, and move it to In Progress. If the repository has no Linear project, skip the ticket. Do not create a project or put the issue in an unrelated project.

2. **Worktree.** Before editing, run `gwt add <branch>` and work in the resulting worktree. Never edit the main checkout. If the task already has a suitable worktree, use it. Keep branch names simple and descriptive: the issue ID, if there is one, and a few words describing the change, such as `abc-12-fix-login-redirect`.

3. **Verify.** Run the repository's required checks and the tests appropriate to the change. If the change affects visible behavior, run it and inspect the result. Record what passed and any unresolved limitations.

4. **PR.** Review the diff and fix findings before opening the pull request. In Claude Code, use `code-review` at `medium` when available; otherwise review the diff directly. Run `gh pr create`. If there is a ticket, end the title with its issue ID, such as `(ABC-12)`. Explain the resulting behavior and how it was verified.

5. **Merge.** Once verification and required CI checks pass, run `gh pr merge <N> --squash`. Do not merge if the owner asked to review first, or if the change touches production data, releases, or deploys. Never use `--delete-branch`: it can cause gh to remove the worktree without running gwt's cleanup hooks.

6. **Clean up.** After a confirmed merge, return to the main checkout and run `gwt remove -f <branch>`. This runs the repository's cleanup hooks and removes the worktree and local branch. Then run `git pull --ff-only` and move the Linear issue to Done, if one was created. Keep the worktree available while a PR is awaiting review or merge.

A merge never means release, deploy, or version bump. Perform those actions only when the user separately authorizes them.

If a required tool or service is unavailable, explain which step is blocked and continue any independent work that remains possible. Do not bypass the worktree rule or claim an unperformed verification, merge, or cleanup succeeded.
