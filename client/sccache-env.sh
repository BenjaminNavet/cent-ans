# Opt-in shared compilation cache for the worktrees (ADR 0362). Usage: `source client/sccache-env.sh`
# before `cargo build`. Safe when sccache is missing (does nothing). Not a .cargo/config.toml
# `rustc-wrapper`: that would break the build of anyone without sccache.
if command -v sccache >/dev/null 2>&1; then
	export RUSTC_WRAPPER=sccache
	# One cache for every ../gp-* worktree (the default is already per user, ~/Library/Caches/Mozilla.sccache).
	export SCCACHE_DIR=${SCCACHE_DIR:-$HOME/.cache/sccache-cent-ans}
	export SCCACHE_CACHE_SIZE=${SCCACHE_CACHE_SIZE:-30G}
	# Worktrees live at different absolute paths (../gp-bm-<lot>): strip the common prefix from
	# the cache key so that they hit each other's entries.
	export SCCACHE_BASEDIRS=${SCCACHE_BASEDIRS:-$HOME/dev}
fi
