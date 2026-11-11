<!--
Agents: fill in every section and tick only the checklist items that are true. Title the pull request `feat: …`, `fix: …`
or `docs: …`, as this repo's commits are. Keep these comments: GitHub hides them on the page, and they are the rules for
whoever edits this description later.
-->

## Summary

<!-- One or two sentences: what changes and why. Name the task id if there is one (for example hsw/134). -->


## Closes

<!-- Closing lines: one per GitHub issue this change finishes, the word Closes, then the issue's number after a hash sign.
The board reads them, and GitHub closes each issue when this merges. Write None when this closes no issue. -->

None

## Changes

<!-- One bullet per area, in this order: crates/core, crates/flow, crates/board, src/, scripts/, docs/, .github/. -->

-

## Verification

<!-- The commands you ran and what they printed: the last lines of `npm run check -- --all`, or of the areas you ran
(`npm run check board-panel`). "Tests pass" alone is not verification. -->

- Command:
- Result:

## Behaviour

<!-- What a user sees that is new or different. Write "No change for users" when that is true. -->


## Checklist

<!-- Each item is a fact about this change. Untick it and say why below it when it does not apply. -->

- [ ] Each change is in the file of its area, and no source file passes 300 lines (`npm run check` runs the repo's rules)
- [ ] A key's chord is only in `crates/flow/src/board/keys.rs`; a setting is only in `crates/core/src/store/settings_list.rs`
- [ ] `docs/keys.md` and `docs/settings.md` are regenerated and committed when a key or a setting changed
- [ ] A new test file has its `mod` line in `tests/main.rs`
- [ ] Board frames in `tests/frames/` were rewritten on purpose (`UPDATE_FRAMES=1 npm run check frames`) and looked at
- [ ] The words of this change are in the `docs/` page of its topic; `README.md` is not changed for a feature
- [ ] No report, review or research result is committed
