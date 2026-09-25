class_name SaveSlots
extends RefCounted

## F3 — emplacements de sauvegarde au-dessus de `SimFacade.save_game` / `load_game` :
## - fiche `user://saves/<nom>.meta.json` (faction, date de jeu, tour, difficulté, date réelle,
##   moteur, libellé) écrite à côté de chaque sauvegarde : la liste n'a pas à relire l'état complet ;
## - vignette `user://saves/<nom>.png` (320 × 180, capture de la carte) ;
## - sauvegarde automatique tous les N tours sur 3 emplacements tournants `auto_1..3` ;
## - `latest()` pour « Continuer ».
## Les autoloads sont obtenus par le nœud racine (script compilé tôt par le smoke test).

## T2 : redirigeable par `use_test_dir` (smoke test), voir `SimFacade.use_test_saves_dir`.
static var SAVES_DIR := "user://saves"
const AUTOSAVE_PREFIX := "auto_"
const AUTOSAVE_SLOTS := 3
const THUMBNAIL_SIZE := Vector2i(320, 180)


## T2 : isole les sauvegardes (smoke test) dans un dossier dédié à cette exécution.
static func use_test_dir(dir: String) -> void:
	SAVES_DIR = dir


static func _facade() -> Node:
	var tree := Engine.get_main_loop() as SceneTree
	return tree.root.get_node_or_null("/root/SimFacade") if tree != null else null


static func meta_path(save_name: String) -> String:
	return SAVES_DIR.path_join(save_name.validate_filename() + ".meta.json")


static func thumbnail_path(save_name: String) -> String:
	return SAVES_DIR.path_join(save_name.validate_filename() + ".png")


static func is_autosave(save_name: String) -> bool:
	return save_name.begins_with(AUTOSAVE_PREFIX)


static func display_name(save_name: String) -> String:
	if is_autosave(save_name):
		return "Sauvegarde automatique %s" % save_name.trim_prefix(AUTOSAVE_PREFIX)
	return save_name


## Sauvegarde nommée + fiche + vignette (facultative). Vrai si l'état a été écrit.
static func save(save_name: String, thumbnail: Image = null) -> bool:
	var facade := _facade()
	if facade == null or facade.get("sim") == null:
		return false
	if not facade.call("save_game", save_name):
		return false
	write_meta(save_name)
	write_thumbnail(save_name, thumbnail)
	return true


## Fiche d'une sauvegarde qui vient d'être écrite par `SimFacade.save_game`.
static func write_meta(save_name: String) -> bool:
	var facade := _facade()
	if facade == null or facade.get("sim") == null:
		return false
	var sim: Object = facade.get("sim")
	var meta := {
		"name": save_name,
		"label": display_name(save_name),
		"faction": str(sim.call("get_player_faction")),
		"date": str(sim.call("get_date_label")),
		"turn": int(sim.call("get_turn")),
		"difficulty": str(facade.call("current_difficulty")) if facade.has_method("current_difficulty") else "normal",
		"timestamp": Time.get_datetime_string_from_system(),
		"engine": "real" if facade.get("is_real") else "mock",
		"path": facade.call("save_path", save_name),
	}
	var file := FileAccess.open(meta_path(save_name), FileAccess.WRITE)
	if file == null:
		return false
	file.store_string(JSON.stringify(meta))
	file.close()
	return true


static func write_thumbnail(save_name: String, image: Image) -> bool:
	if image == null or image.is_empty():
		return false
	var copy := image.duplicate() as Image
	if copy.is_compressed():
		return false
	copy.convert(Image.FORMAT_RGB8)
	# Recadrage 16:9 centré puis réduction.
	var width := copy.get_width()
	var height := copy.get_height()
	var target_h := int(width * 9.0 / 16.0)
	if target_h <= height:
		copy = copy.get_region(Rect2i(0, (height - target_h) / 2, width, target_h))
	else:
		var target_w := int(height * 16.0 / 9.0)
		copy = copy.get_region(Rect2i((width - target_w) / 2, 0, target_w, height))
	copy.resize(THUMBNAIL_SIZE.x, THUMBNAIL_SIZE.y, Image.INTERPOLATE_BILINEAR)
	return copy.save_png(thumbnail_path(save_name)) == OK


static func thumbnail_texture(save_name: String) -> Texture2D:
	var path := thumbnail_path(save_name)
	if not FileAccess.file_exists(path):
		return null
	var image := Image.load_from_file(ProjectSettings.globalize_path(path))
	if image == null or image.is_empty():
		return null
	return ImageTexture.create_from_image(image)


## Emplacement de la sauvegarde automatique du tour `turn` (vide si pas de sauvegarde ce tour) :
## tous les `interval` tours, `auto_1`, `auto_2`, `auto_3`, `auto_1`…
static func autosave_name_for(turn: int, interval: int) -> String:
	if interval <= 0 or turn <= 0 or turn % interval != 0:
		return ""
	return "%s%d" % [AUTOSAVE_PREFIX, (turn / interval - 1) % AUTOSAVE_SLOTS + 1]


static func autosave(turn: int, interval: int, thumbnail: Image = null) -> String:
	var save_name := autosave_name_for(turn, interval)
	if save_name == "":
		return ""
	return save_name if save(save_name, thumbnail) else ""


## Sauvegardes, plus récentes d'abord : `{path, name, label, faction, date, turn, difficulty,
## timestamp, engine, thumbnail}` (`difficulty` absente des sauvegardes d'avant DF1 : Normale) ; la fiche `.meta.json` évite de relire l'état, sinon repli sur
## `SimFacade.read_save`.
static func list() -> Array[Dictionary]:
	var result: Array[Dictionary] = []
	var dir := DirAccess.open(SAVES_DIR)
	if dir == null:
		return result
	var facade := _facade()
	for file_name in dir.get_files():
		if not file_name.ends_with(".json") or file_name.ends_with(".meta.json"):
			continue
		var save_name := file_name.get_basename()
		var path := SAVES_DIR.path_join(file_name)
		var entry: Dictionary = {}
		var meta_file := meta_path(save_name)
		if FileAccess.file_exists(meta_file):
			var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(meta_file))
			if parsed is Dictionary:
				entry = parsed
		if entry.is_empty() and facade != null:
			var wrapper: Dictionary = facade.call("read_save", path)
			if wrapper.is_empty():
				continue
			entry = {
				"faction": str(wrapper.get("faction", "")),
				"date": str(wrapper.get("date", "")),
				"turn": int(wrapper.get("turn", 0)),
				"difficulty": str(wrapper.get("difficulty", "normal")),
				"timestamp": str(wrapper.get("timestamp", "")),
				"engine": str(wrapper.get("engine", "mock")),
			}
		entry["path"] = path
		entry["name"] = save_name
		entry["label"] = display_name(save_name)
		entry["thumbnail"] = thumbnail_path(save_name) if FileAccess.file_exists(thumbnail_path(save_name)) else ""
		result.append(entry)
	result.sort_custom(func(a: Dictionary, b: Dictionary) -> bool: return str(a.get("timestamp", "")) > str(b.get("timestamp", "")))
	return result


## Sauvegarde chargeable par ce moteur (une partie « réelle » exige la GDExtension).
static func loadable(entry: Dictionary) -> bool:
	return str(entry.get("engine", "mock")) != "real" or ClassDB.class_exists("CampaignSim")


## Sauvegarde la plus récente chargeable (« Continuer »), vide s'il n'y en a pas.
static func latest() -> Dictionary:
	for entry in list():
		if loadable(entry):
			return entry
	return {}


static func delete(save_name: String) -> void:
	for path in [SAVES_DIR.path_join(save_name.validate_filename() + ".json"), meta_path(save_name), thumbnail_path(save_name)]:
		if FileAccess.file_exists(path):
			DirAccess.remove_absolute(path)
