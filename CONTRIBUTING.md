# Contributing

Keep it clean, keep it human. A few hard rules, not suggestions.

## 1. The build has to be clean
- `cargo build` must pass. If cargo throws an **error**, it does not get pushed. Full stop.
- **Warnings count too.** A warning is a fix, not a vibe. Clear them before you push. Don't `#[allow(...)]` your way out of one unless there's a real reason and you say why in a comment.
- Actually run it. If it doesn't launch, it's not done.

## 2. AI is fine. Unreviewed AI is not.
Use whatever tools you want (Copilot, Claude, whatever). But you own every single line you push.
- Read it. Understand it. If you can't explain what a function does, you don't push it.
- We can tell when nobody reviewed the AI output. If we catch un-reviewed slop, it's *ggs*, the PR gets closed.
- AI writes the draft, you ship it. Act like it's your name on it, because it is.

## 3. Style
- **No em dashes. Anywhere.** (Yes,I am serious.)
- Comments should read like a person wrote them. Casual is fine, robotic is not.
- Public items get rustdoc (`///`), every module gets a `//!` header.
- Keep files focused, roughly 400 lines max. If one blows past that, split it into modules.

## 4. Don't leak secrets
- `tokens.json` is git-ignored and it stays that way. Never commit keys.
- Adding a new secret-shaped file? Put it in `.gitignore` before your first commit, not after.

## Before you open a PR
- [ ] `cargo build` = 0 errors, 0 warnings
- [ ] it actually runs
- [ ] you read every line you're pushing (not just skimmed it)
- [ ] no em dashes, no secrets
