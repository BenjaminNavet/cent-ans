class_name FaUi
extends RefCounted

## Lot FA5 — ornements réels de l'interface : initiales enluminées, rinceaux, sceaux de cire et
## matières découpés dans des œuvres du domaine public (`game/assets/ui/fa/`, sources dans
## `SOURCE.md`). Le catalogue des découpes est `data/ui/fa_ui_assets.json` (schéma
## `data/schemas/ui_fa_assets.schema.json`), lu ici pour savoir quelle initiale existe.
## Rendu seulement. `enabled = false` (option `--no-fa` des scripts de capture) rend l'ancien
## habillage pour les comparaisons avant/après.

const DATA_PATH := "ui/fa_ui_assets.json"
const ASSET_DIR := "res://assets/ui/fa/"
const MAP_PATHS_SCRIPT := preload("res://scripts/map/map_paths.gd")

static var enabled := true
static var _data: Dictionary = {}
static var _loaded := false
static var _textures: Dictionary = {}


static func data() -> Dictionary:
	if not _loaded:
		_loaded = true
		# Toujours le catalogue du dépôt : les jeux de données réduits des tests n'en ont pas.
		var path := MAP_PATHS_SCRIPT.project_root().path_join("data").path_join(DATA_PATH)
		if FileAccess.file_exists(path):
			var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
			if parsed is Dictionary:
				_data = parsed
		if _data.is_empty():
			push_warning("FaUi: %s missing or invalid" % path)
	return _data


## Texture `game/assets/ui/fa/<kind>/<id>.png`, ou null (absente, ou FA désactivé).
static func texture(kind: String, id: String) -> Texture2D:
	if not enabled or id.is_empty():
		return null
	var path := ASSET_DIR + kind + "/" + id + ".png"
	if not _textures.has(path):
		_textures[path] = load(path) as Texture2D if ResourceLoader.exists(path) else null
	return _textures[path]


## Réglage de pose (`display` du catalogue), `fallback` si absent.
static func display(key: String, fallback: float) -> float:
	return float((data().get("display", {}) as Dictionary).get(key, fallback))


## Initiale enluminée réelle pour la première lettre de `title` (sans accent) :
## `{"texture": Texture2D, "framed": bool}` (`framed` : champ peint opaque), ou `{}` — le titre
## garde alors la lettrine dessinée. Le jeu de la culture du joueur passe d'abord (feuillets
## anglais pour une faction anglaise, français sinon) ; entre plusieurs initiales de la même
## lettre, le choix est stable pour un titre donné.
static func initial_for(title: String) -> Dictionary:
	if not enabled or title.is_empty():
		return {}
	var letter := title.substr(0, 1).to_upper()
	letter = str(Lettrine.UNACCENTED.get(letter, letter))
	var preferred := preferred_set()
	var own: Array[Dictionary] = []
	var other: Array[Dictionary] = []
	for entry: Dictionary in data().get("initials", []):
		if str(entry.get("letter", "")) != letter:
			continue
		if str(entry.get("set", "")) == preferred:
			own.append(entry)
		else:
			other.append(entry)
	var pool := own if not own.is_empty() else other
	if pool.is_empty():
		return {}
	var chosen := pool[absi(hash(title)) % pool.size()]
	var image := texture("initials", str(chosen.get("id", "")))
	if image == null:
		return {}
	return {"texture": image, "framed": str(chosen.get("mode", "")) == "framed"}


## Jeu d'initiales de la culture du joueur : `english` ou `french` (`display.english_cultures`).
static func preferred_set() -> String:
	var tree := Engine.get_main_loop() as SceneTree
	var facade: Node = tree.root.get_node_or_null("SimFacade") if tree != null and tree.root != null else null
	if facade == null:
		return "french"
	var faction := str(facade.get("pending_faction"))
	var culture := str((facade.call("faction_info", faction) as Dictionary).get("culture", ""))
	var english: Array = (data().get("display", {}) as Dictionary).get("english_cultures", [])
	return "english" if english.has(culture) else "french"
