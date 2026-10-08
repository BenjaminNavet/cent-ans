class_name DataFile
extends RefCounted

## Accès unique aux fichiers JSON de `data/` : résolution du dossier (autoload `MapPaths`,
## sinon valeur par défaut dev/export) et lecture avec cache par chemin absolu.

const MAP_PATHS_SCRIPT := preload("res://scripts/map/map_paths.gd")

static var _cache: Dictionary = {}


## Dossier `data/` courant (celui de l'autoload `MapPaths` s'il existe, sinon le défaut).
static func data_dir() -> String:
	var tree := Engine.get_main_loop() as SceneTree
	if tree != null and tree.root != null:
		var map_paths := tree.root.get_node_or_null("MapPaths")
		if map_paths != null:
			return str(map_paths.get("data_dir"))
	return MAP_PATHS_SCRIPT.default_data_dir()


## Chemin absolu de `rel_path` (relatif à `data/`). Si le dossier courant (ex. fixtures de test)
## ne le contient pas, retombe sur le `data/` du dépôt.
static func path_of(rel_path: String) -> String:
	var path := data_dir().path_join(rel_path)
	if not FileAccess.file_exists(path):
		var repo_path: String = MAP_PATHS_SCRIPT.project_root().path_join("data").path_join(rel_path)
		if FileAccess.file_exists(repo_path):
			return repo_path
	return path


## Vrai si le fichier existe sous `data/`.
static func exists(rel_path: String) -> bool:
	return FileAccess.file_exists(path_of(rel_path))


## Lit et parse un JSON de `data/`. Retourne `null` (avec `push_error`) s'il est absent ou invalide.
static func read_json(rel_path: String) -> Variant:
	return parse_file(path_of(rel_path))


## Lit et parse un JSON à chemin absolu. Retourne `null` (avec `push_error`) s'il est absent ou invalide.
static func parse_file(path: String) -> Variant:
	if not FileAccess.file_exists(path):
		push_error("DataFile: fichier absent : %s" % path)
		return null
	var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
	if parsed == null:
		push_error("DataFile: JSON invalide : %s" % path)
	return parsed


## Comme `read_json`, mis en cache par chemin absolu (un échec n'est pas mémorisé).
static func load_cached(rel_path: String) -> Variant:
	var path := path_of(rel_path)
	if _cache.has(path):
		return _cache[path]
	var parsed: Variant = read_json(rel_path)
	if parsed != null:
		_cache[path] = parsed
	return parsed


## Vide le cache (nouvelle partie, tests, changement de `data_dir`).
static func clear_cache() -> void:
	_cache.clear()
