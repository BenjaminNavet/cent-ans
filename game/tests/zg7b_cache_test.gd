extends SceneTree

## Test headless du lot ZG7b (cache de relief absent ou partiel, ADR 0036), sans le vrai cache :
## un faux dossier `map/` (manifeste de pyramide à deux étages, index des fleuves et routes fins,
## tuiles vides) est écrit dans `user://zg7b_test/`.
##  1. `ReliefCacheStatus` : sans manifeste → DISABLED ; tuiles listées sans `pyramid/` → MISSING ;
##     un étage absent → PARTIAL (étage nommé) ; tout présent → COMPLETE ; fleuves fins absents →
##     PARTIAL (couche nommée) ; échantillonnage borné sur un grand étage ;
##  2. `ReliefCacheNotice.report` : avis ajouté une seule fois par session, avec la commande de
##     régénération, fermable ; rien sans manifeste ;
##  3. `MapPaths.relief_root_for` : `data/map` s'il contient `pyramid/`, variable
##     `CENT_ANS_RELIEF_DIR` sinon prioritaire.
## Usage : godot --headless --path game --script res://tests/zg7b_cache_test.gd

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
const TEST_DIR := "user://zg7b_test"

var _failures := 0


func _init() -> void:
	await process_frame
	_run()
	print("zg7b_cache_test: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("zg7b_cache_test: " + message)
	return condition


func _touch(path: String) -> void:
	DirAccess.make_dir_recursive_absolute(path.get_base_dir())
	var f := FileAccess.open(path, FileAccess.WRITE)
	f.store_8(0)
	f.close()


func _write_json(path: String, value: Variant) -> void:
	DirAccess.make_dir_recursive_absolute(path.get_base_dir())
	var f := FileAccess.open(path, FileAccess.WRITE)
	f.store_string(JSON.stringify(value))
	f.close()


func _remove_tree(path: String) -> void:
	if not DirAccess.dir_exists_absolute(path):
		return
	for file in DirAccess.get_files_at(path):
		DirAccess.remove_absolute(path.path_join(file))
	for sub in DirAccess.get_directories_at(path):
		_remove_tree(path.path_join(sub))
	DirAccess.remove_absolute(path)


## Manifeste : E1 = 2 × 2 tuiles, E2 = une ligne de 50 tuiles (échantillonnée).
func _write_map(map_dir: String) -> void:
	_write_json(map_dir.path_join("relief_pyramid.json"), {
		"version": 1, "tile_px": 512, "root_tile_units": 256, "dir": "pyramid",
		"pattern": "E{level}/{col}_{row}.png",
		"levels": [
			{"level": 1, "tiles_rle": [{"row": 14, "runs": [[16, 2]]}, {"row": 15, "runs": [[16, 2]]}]},
			{"level": 2, "tiles_rle": [{"row": 30, "runs": [[5, 50]]}]},
			{"level": 5, "tiles_rle": []},
		],
	})
	_write_json(map_dir.path_join("rivers_fine.json"), {
		"dir": "pyramid/hydro_fine", "pattern": "E2/{col}_{row}.bin",
		"tiles": [{"col": 3, "row": 4}, {"col": 5, "row": 6}],
	})
	_write_json(map_dir.path_join("fine_anchors.json"), {
		"roads": {"dir": "pyramid/roads_fine", "pattern": "E2/{col}_{row}.bin", "tiles": [{"col": 7, "row": 8}]},
	})


func _write_level(root: String, level: int) -> void:
	if level == 1:
		for row in range(14, 16):
			for col in range(16, 18):
				_touch(root.path_join("pyramid/E1/%d_%d.png" % [col, row]))
	else:
		for col in range(5, 55):
			_touch(root.path_join("pyramid/E2/%d_30.png" % col))


func _write_fine(root: String, rivers: bool) -> void:
	if rivers:
		_touch(root.path_join("pyramid/hydro_fine/E2/3_4.bin"))
		_touch(root.path_join("pyramid/hydro_fine/E2/5_6.bin"))
	_touch(root.path_join("pyramid/roads_fine/E2/7_8.bin"))


func _run() -> void:
	var base := ProjectSettings.globalize_path(TEST_DIR)
	_remove_tree(base)
	var map_dir := base.path_join("map")
	DirAccess.make_dir_recursive_absolute(map_dir)
	OS.unset_environment(MAP_PATHS.RELIEF_ENV_VAR)

	# 1. États.
	var status := ReliefCacheStatus.check(map_dir)
	_check(status.state == ReliefCacheStatus.State.DISABLED, "no manifest → DISABLED")
	_check(not status.needs_notice(), "no notice without manifest")

	_write_map(map_dir)
	status = ReliefCacheStatus.check(map_dir)
	_check(status.state == ReliefCacheStatus.State.MISSING, "listed tiles, no pyramid dir → MISSING (got %d)" % status.state)
	_check(status.needs_notice(), "MISSING needs the notice")
	_check(status.expected.get(1, 0) == 4 and status.expected.get(2, 0) == 50, "RLE expanded (E1 4, E2 50)")
	_check(not status.expected.has(5), "empty level ignored")
	_check(int(status.sampled.get(2, 0)) == ReliefCacheStatus.SAMPLES_PER_LEVEL, "large level sampled, not scanned")
	_check(status.notice_text().contains("introuvable"), "MISSING text says the cache is missing")
	_check(status.summary().begins_with("ReliefCache: MISSING"), "log summary: %s" % status.summary())

	_write_level(map_dir, 1)
	_write_fine(map_dir, true)
	status = ReliefCacheStatus.check(map_dir)
	_check(status.state == ReliefCacheStatus.State.PARTIAL, "E2 missing → PARTIAL (got %d)" % status.state)
	_check(status.incomplete_levels() == [2], "incomplete levels = [2] (got %s)" % [status.incomplete_levels()])
	_check(status.notice_text().contains("E2"), "PARTIAL text names E2")

	_write_level(map_dir, 2)
	status = ReliefCacheStatus.check(map_dir)
	_check(status.state == ReliefCacheStatus.State.COMPLETE, "all tiles → COMPLETE (got %d) %s" % [status.state, status.summary()])
	_check(not status.needs_notice(), "COMPLETE: no notice")

	DirAccess.remove_absolute(map_dir.path_join("pyramid/hydro_fine/E2/5_6.bin"))
	status = ReliefCacheStatus.check(map_dir)
	_check(status.state == ReliefCacheStatus.State.PARTIAL, "one fine river tile missing → PARTIAL")
	_check(status.incomplete_fine_layers() == ["rivers"], "incomplete fine layers = [rivers] (got %s)" % [status.incomplete_fine_layers()])
	_check(status.notice_text().contains("fleuves fins"), "PARTIAL text names the fine rivers")

	# 2. Avis une seule fois par session.
	var ui := CanvasLayer.new()
	root.add_child(ui)
	ReliefCacheNotice.shown_this_session = false
	var empty_dir := base.path_join("empty")
	DirAccess.make_dir_recursive_absolute(empty_dir)
	_check(ReliefCacheNotice.report(ui, empty_dir, empty_dir).state == ReliefCacheStatus.State.DISABLED, "report: DISABLED without manifest")
	_check(ui.get_child_count() == 0, "no notice without manifest")
	var reported := ReliefCacheNotice.report(ui, map_dir, map_dir)
	_check(reported != null and reported.state == ReliefCacheStatus.State.PARTIAL, "report returns the checked status")
	_check(ui.get_child_count() == 1, "notice added once")
	var notice := ui.get_child(0) as ReliefCacheNotice
	if _check(notice != null, "child is a ReliefCacheNotice"):
		_check(notice.command_field != null and notice.command_field.text == ReliefCacheStatus.FETCH_COMMAND, "notice shows the fetch command first (SZ7, ADR 0077)")
		_check(notice.mouse_filter == Control.MOUSE_FILTER_STOP and notice.anchor_left == 0.5, "notice: top-centred panel, clicks kept to itself")
	ReliefCacheNotice.report(ui, map_dir, map_dir)
	_check(ui.get_child_count() == 1, "second report in the same session: no second notice")
	if notice != null:
		notice.dismiss()
		_check(not notice.visible, "dismiss hides the notice")

	# 3. Résolution du dossier de relief.
	_check(MAP_PATHS.relief_root_for(map_dir) == map_dir, "map_dir with pyramid/ is its own relief root")
	_check(MAP_PATHS.relief_root_for(empty_dir) == empty_dir, "fallback: map_dir when nothing is found")
	var external := base.path_join("external")
	_write_level(external, 1)
	OS.set_environment(MAP_PATHS.RELIEF_ENV_VAR, external)
	_check(MAP_PATHS.relief_root_for(empty_dir) == external, "CENT_ANS_RELIEF_DIR wins")
	status = ReliefCacheStatus.check(map_dir, MAP_PATHS.relief_root_for(map_dir))
	_check(status.incomplete_levels() == [2], "external root: E1 found there, E2 missing")
	OS.unset_environment(MAP_PATHS.RELIEF_ENV_VAR)
	ui.queue_free()
	_remove_tree(base)
