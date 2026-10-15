MERGE AGENT (one lot, fresh context) — Bevy migration of "Cent Ans".
You merge a BATCH of 2-4 lot branches `bm/<lot>` into `bevy-migration` (checkout /Users/jean_hubert/dev/gp-bevy), one after the other (steps 1-7 per lot, the lock may be held for the whole batch), then stop. Full checks (clippy --all-targets, cargo test --features dev) per lot; smoke may be run once after the last lot of the batch with the union of scenarios, bisecting only if it fails.
Scratchpad: S=/private/tmp/claude-501/-Users-jean-hubert-dev-game-project/bf386972-2fd1-410e-a891-68fddfc05b7a/scratchpad

1. Lock (only one merge at a time): `until mkdir $S/merge.lock 2>/dev/null; do sleep 20; done` (run it with Bash; if it takes > 9 min, re-run it). ALWAYS `rmdir $S/merge.lock` at the end, success or failure.
2. Check: the worktree /Users/jean_hubert/dev/gp-bm-<lot> exists with `.bm-ready`, `git -C <wt> status --porcelain --untracked-files=all client/src core` empty (else: create `<wt>/.bm-failed` with the reason, unlock, report). gp-bevy must be clean (`git status --porcelain --untracked-files=no`); if a previous merge was left half-done, `git merge --abort` only if MERGE_HEAD exists.
3. `cd gp-bevy && git merge --no-edit bm/<lot>`. Conflicts: resolve them properly (both sides' intent; read both lots' code; README/wip notes: keep both sections). Commit the merge. Never `git stash`, never `-a`/`add -A` (add explicit paths), no Co-Authored-By, never push, never touch main.
4. Checks with REAL exit codes (no `| grep | head` masking: redirect to a file, test `$?`, then grep the file):
   `cd client && cargo clippy --features dev --all-targets -- -D warnings`, `cargo test --features dev` ; if the lot touched core/: `cd core && cargo clippy -p game-api -- -D warnings && cargo test -p game-api`.
   Fix breakages caused by the merge yourself (small integration fixes, committed as `fix(client): … after merging <lot>`). If the lot itself is broken beyond a small fix: `git reset --hard ORIG_HEAD` (only your merge), `.bm-failed` with the reason.
5. Targeted smoke: `client/tests/smoke.sh <scenario…>` for the scenarios touching the lot (see client/tests/smoke/*.txt; campaign lots → campaign campaign_turns; battle lots → battle crecy siege; menus → main_menu faction_select; codex → codex). Machine is heavily loaded: a timeout once is retried once.
6. Clean up: `zsh $S/bm_rm.sh <lot>` (checks ancestor + clean status, removes worktree and branch).
7. Append one line to $S/merge_log.md: `<time> <lot> <merge hash> OK|FAILED clippy/tests/smoke + fixes`.
8. Unlock, then final report ≤6 lines (merge hash, checks, fixes, problems).
   NOTE: run `cargo test` WITHOUT CENT_ANS_MUTE in the env (audio_playback::plays_and_crossfades_without_device fails under it: "one music voice in the menu" 0≠1). Keep CENT_ANS_MUTE=1 for smoke only.
