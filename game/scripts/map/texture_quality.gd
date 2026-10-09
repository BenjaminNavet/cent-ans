class_name TextureQuality
extends RefCounted
## TX (ADR 0236) : choix des textures régionales générées.
##
## `use_tx()` : faux avec `--legacy-textures` (anciennes textures d'avant TX, jusqu'à la partie
## pilote) pour le parcellaire HB, la couche par défaut des bâtiments, la végétation, l'eau et les
## écorces. Il n'affecte PLUS le sol de bataille ni le sol de campagne (ADR 0244 : plus de Poly
## Haven, les paquets TX régionaux sont la seule voie).
## `hi_path(path)` : variante 2k locale d'un paquet (`<dossier>/hi/<fichier>`, manifeste
## `<nom>_2048.json`) quand « Qualité des textures » vaut « haute » et qu'elle existe, sinon `path`.

const LEGACY_FLAG := "--legacy-textures"
const HI_DIR := "hi/"
const HI_SIZE := 2048


static func use_tx() -> bool:
	return not (OS.get_cmdline_user_args().has(LEGACY_FLAG) or OS.get_cmdline_args().has(LEGACY_FLAG))


static func is_high() -> bool:
	var tree := Engine.get_main_loop() as SceneTree
	var settings: Node = tree.root.get_node_or_null("Settings") if tree != null else null
	if settings == null:
		return true
	return str(settings.call("get_value", "video/texture_quality")) != "medium"


## Texture d'un paquet : `res://a/b.jpg` → `res://a/hi/b.jpg` si haute qualité et présent.
static func texture_path(path: String) -> String:
	if not is_high():
		return path
	var hi := path.get_base_dir().path_join(HI_DIR + path.get_file())
	return hi if ResourceLoader.exists(hi) else path


## Manifeste d'un paquet : `res://data/x.json` → `res://data/x_2048.json` si haute qualité et présent.
static func manifest_path(path: String) -> String:
	if not is_high():
		return path
	var hi := path.get_basename() + "_%d.%s" % [HI_SIZE, path.get_extension()]
	return hi if FileAccess.file_exists(hi) else path
