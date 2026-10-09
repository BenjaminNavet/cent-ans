extends TestCase

## Test headless : résolution du dossier de relief (`MapPaths.relief_root_for`, ADR 0036).
## `data/map` s'il contient `pyramid/`, variable `CENT_ANS_RELIEF_DIR` sinon prioritaire.
## (L'avis de cache ReliefCacheNotice/ReliefCacheStatus a été remplacé par un push_warning, MB6.)
## Usage : godot --headless --path game --script res://tests/zg7b_cache_test.gd

const TEST_DIR := "user://zg7b_test"


func _init() -> void:
	await process_frame
	_run()
	finish()


func _remove_tree(path: String) -> void:
	if not DirAccess.dir_exists_absolute(path):
		return
	for file in DirAccess.get_files_at(path):
		DirAccess.remove_absolute(path.path_join(file))
	for sub in DirAccess.get_directories_at(path):
		_remove_tree(path.path_join(sub))
	DirAccess.remove_absolute(path)


func _run() -> void:
	var base := ProjectSettings.globalize_path(TEST_DIR)
	_remove_tree(base)
	var map_dir := base.path_join("map")
	var empty_dir := base.path_join("empty")
	var external := base.path_join("external")
	DirAccess.make_dir_recursive_absolute(map_dir.path_join("pyramid"))
	DirAccess.make_dir_recursive_absolute(empty_dir)
	DirAccess.make_dir_recursive_absolute(external.path_join("pyramid"))
	OS.unset_environment(MAP_PATHS.RELIEF_ENV_VAR)
	check(MAP_PATHS.relief_root_for(map_dir) == map_dir, "map_dir with pyramid/ is its own relief root")
	check(MAP_PATHS.relief_root_for(empty_dir) == empty_dir, "fallback: map_dir when nothing is found")
	OS.set_environment(MAP_PATHS.RELIEF_ENV_VAR, external)
	check(MAP_PATHS.relief_root_for(empty_dir) == external, "CENT_ANS_RELIEF_DIR wins")
	OS.unset_environment(MAP_PATHS.RELIEF_ENV_VAR)
	_remove_tree(base)
