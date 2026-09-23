extends Node

## Autoload `IconLibrary` (F2) : icônes SVG monochromes encre sépia de game-icons.net
## (CC BY 3.0, voir `CREDITS.md`), générées par `cent-ans assets icons` dans
## `res://assets/icons/` et décrites par `icons.json` (id → fichier, catégorie, auteur).
##
## `get_icon(id)` renvoie la texture de l'identifiant ; à défaut, celle de sa catégorie
## (`category` explicite, sinon déduite du préfixe : `unit_`, `bld_`, `res_`, `tech_`,
## `trait_`, `skill_`, `class_`, `gauge_`, `hud_`, `branch_`), puis l'icône par défaut.
## Godot importe les SVG nativement (64 px, taille déclarée dans le fichier).

const ICONS_DIR := "res://assets/icons"
const TABLE_PATH := "res://assets/icons/icons.json"
const PREFIX_CATEGORIES := {
	"unit_": "unit", "bld_": "building", "building_": "building", "res_": "resource",
	"tech_": "technology", "trait_": "trait", "skill_": "skill", "class_": "class",
	"gauge_": "gauge", "hud_": "hud", "branch_": "branch",
}

## id → {file, category, source, author}
var icons: Dictionary = {}
## catégorie → id de repli
var fallbacks: Dictionary = {}
var _textures: Dictionary = {}  # chemin → Texture2D


func _ready() -> void:
	load_table()


## Relit `icons.json` ; faux si la table manque (le jeu tourne alors sans icônes).
func load_table() -> bool:
	icons = {}
	fallbacks = {}
	_textures.clear()
	if not FileAccess.file_exists(TABLE_PATH):
		push_warning("IconLibrary: %s missing (run cent-ans assets icons)" % TABLE_PATH)
		return false
	var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(TABLE_PATH))
	if not (parsed is Dictionary):
		push_warning("IconLibrary: invalid %s" % TABLE_PATH)
		return false
	icons = parsed.get("icons", {})
	fallbacks = parsed.get("fallbacks", {})
	return true


func has_icon(id: String) -> bool:
	return icons.has(id)


## Catégorie déduite d'un identifiant (`unit_knights` → `unit`), "default" sinon.
func category_of(id: String) -> String:
	if icons.has(id):
		return str(icons[id].get("category", "default"))
	for prefix in PREFIX_CATEGORIES:
		if id.begins_with(prefix):
			return PREFIX_CATEGORIES[prefix]
	return "default"


## Identifiant effectivement affiché pour `id` (lui-même, ou le repli de sa catégorie).
func resolve(id: String, category: String = "") -> String:
	if icons.has(id):
		return id
	var fallback_category := category if category != "" else category_of(id)
	var fallback: String = str(fallbacks.get(fallback_category, fallbacks.get("default", "")))
	if icons.has(fallback):
		return fallback
	return ""


## Chemin `res://` du SVG (pour `[img]` en BBCode), "" si aucune icône.
func icon_path(id: String, category: String = "") -> String:
	var resolved := resolve(id, category)
	if resolved == "":
		return ""
	return ICONS_DIR.path_join(str(icons[resolved].get("file", "")))


## Texture de `id` avec repli par catégorie ; null seulement si la table est absente.
func get_icon(id: String, category: String = "") -> Texture2D:
	var path := icon_path(id, category)
	if path == "":
		return null
	if _textures.has(path):
		return _textures[path]
	var texture: Texture2D = null
	if ResourceLoader.exists(path):
		texture = load(path) as Texture2D
	_textures[path] = texture
	return texture


func author_of(id: String) -> String:
	return str(icons.get(resolve(id), {}).get("author", ""))


## `TextureRect` carré prêt à insérer dans une ligne d'interface.
func make_rect(id: String, size: float = 20.0, category: String = "") -> TextureRect:
	var rect := TextureRect.new()
	rect.texture = get_icon(id, category)
	rect.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	rect.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
	rect.custom_minimum_size = Vector2(size, size)
	rect.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	rect.mouse_filter = Control.MOUSE_FILTER_IGNORE
	rect.name = "Icon_%s" % id
	return rect


## Pose l'icône sur un bouton (taille bornée par `icon_max_width`).
func decorate_button(button: Button, id: String, size: int = 20, category: String = "") -> void:
	button.icon = get_icon(id, category)
	button.expand_icon = false
	button.add_theme_constant_override("icon_max_width", size)


## BBCode `[img]` de l'icône (infobulles riches), "" si aucune.
func bbcode(id: String, size: int = 18, category: String = "") -> String:
	var path := icon_path(id, category)
	if path == "":
		return ""
	return "[img=%dx%d]%s[/img]" % [size, size, path]


func clear_cache() -> void:
	_textures.clear()
