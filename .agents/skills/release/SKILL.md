---
name: release
description: Release the Shekere application. Use when asked to prepare, verify, version, tag, commit, or publish a Shekere release.
---

# Role

Act as the Release Manager for the Shekere project. Follow this procedure to safely update versions, verify builds, and publish releases.

# Constraints

- Never execute `git push`, including tags, until the user explicitly replies `Approve` to the final confirmation prompt.
- If the workflow pauses to ask a question, fix a warning, or perform another operation, ask for and receive `Approve` again before tagging or pushing. Do not infer approval from an earlier message.
- Edit carefully and inspect diffs to prevent unintended file changes.
- If an error occurs, report it immediately and pause. Do not continue based on guesswork.

# Release Workflow

## 1. Pre-flight Checks

1. Run `git status` to ensure the repository has no uncommitted changes.
2. Confirm that `docs/` reflects new application features, API changes, and bug fixes.
3. If uncommitted changes exist, stop and ask the user to commit or stash them.

## 2. Determine Version Number

Ask the user to decide the new version number, for example `0.5.3`.

## 3. Build and Integrity Verification

Run at least the following checks:

- `npx tsc --noEmit`
- `cargo check`
- `npm run docs:build`

Alternatively, run `npm run build` and `npm run docs:build`. If any command fails, stop the release, report the error, and resolve it before continuing.

## 4. Update Version Numbers

Update the decided version in all three files:

1. `package.json` `version`
2. `tauri.conf.json` `version`
3. `Cargo.toml` `version`

Inspect the edited files to verify the replacement.

## 5. Wait for Approval before Commit and Push

Show the diffs for the three version files and the planned tag, such as `v0.5.3`. Ask: "Is it okay to commit these, create the tag, and push to the main branch?" Then wait for the explicit reply `Approve`.

## 6. Commit, Tag, and Push

Only after explicit approval, run the following commands in order:

```bash
git add package.json tauri.conf.json Cargo.toml
git commit -m "Release v<version_number>"
git tag v<version_number>
git push origin main v<version_number>
```

## 7. Completion Report

Report the successful push and mention whether GitHub Releases are automatically triggered.
