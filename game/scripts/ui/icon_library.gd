extends Node

## Autoload `IconLibrary` (F2) : icônes SVG monochromes encre sépia de game-icons.net
## (CC BY 3.0, voir `CREDITS.md`), générées par `cent-ans assets icons` dans
## `res://assets/icons/` et décrites par `icons.json` (id → fichier, catégorie, auteur).
##
## `get_icon(id)` renvoie la texture de l'identifiant ; à défaut, celle de sa catégorie
## (`category` explicite, sinon déduite du préfixe : `unit_`, `bld_`, `res_`, `tech_`,
## `trait_`, `skill_`, `class_`, `gauge_`, `hud_`, `branch_`), puis l'icône par défaut.
## Godot importe les SVG nativement (64 px, taille déclarée dans le fichier).
##
## Lot DA5 : famille unique d'icônes d'action à l'encre (PNG 128 px, RVB = `HudStyle.INK`,
## alpha = encre ; `cent-ans assets ink-icons`, catalogue `data/ui/icons_ink.json`).
## `ink/index.json` (id → fichier) est lu en premier : un identifiant qu'il couvre prend
## l'icône à l'encre, sinon le SVG (repli si le PNG manque). Teinte au rendu : `tint(color)`
## donne la modulation qui change l'encre en `color` (or au survol, encre pâlie désactivé).
## Médaillons enluminés : `get_medallion(id)` (`res://assets/ui/medallions/`, même index).
##
## Lot DA5b : icônes d'entité en miniatures peintes (unités, bâtiments, techniques, compétences,
## ressources, régimes, catégories d'unité), PNG 128 px encadrés or et azur
## (`cent-ans assets entity-icons`, catalogue `data/ui/entity_icons.json`). `entity/index.json`
## est lu en premier : priorité miniature d'entité > encre > SVG. Une miniature n'est jamais
## teintée (elle porte ses couleurs), elle garde ses couleurs au survol.

const ICONS_DIR := "res://assets/icons"
const TABLE_PATH := "res://assets/icons/icons.json"
const ENTITY_DIR := "res://assets/icons/entity"
const ENTITY_INDEX_PATH := "res://assets/icons/entity/index.json"
const INK_DIR := "res://assets/icons/ink"
const INK_INDEX_PATH := "res://assets/icons/ink/index.json"
const MEDALLIONS_DIR := "res://assets/ui/medallions"
const MEDALLIONS_INDEX_PATH := "res://assets/ui/medallions/index.json"
## Couleur de l'encre cuite dans les PNG (== `HudStyle.INK`).
const INK_COLOR := Color(0.22, 0.14, 0.07, 1.0)
const GOLD_COLOR := Color(0.72, 0.56, 0.24, 1.0)
const FADED_COLOR := Color(0.50, 0.44, 0.36, 0.55)
const PREFIX_CATEGORIES := {
	"unit_": "unit", "bld_": "building", "building_": "building", "res_": "resource",
	"tech_": "technology", "trait_": "trait", "skill_": "skill", "class_": "class",
	"gauge_": "gauge", "hud_": "hud", "branch_": "branch",
}

## id → {file, category, source, author}
var icons: Dictionary = {}
## catégorie → id de repli
var fallbacks: Dictionary = {}
## DA5 : id → chemin `res://` du PNG à l'encre ; id → chemin du médaillon.
var ink: Dictionary = {}
var medallions: Dictionary = {}
## DA5b : id → chemin `res://` de la miniature peinte encadrée.
var entity: Dictionary = {}
var _textures: Dictionary = {}  # chemin → Texture2D (ou états d'un médaillon)


func _ready() -> void:
	load_table()


## Relit `icons.json` (et les index DA5) ; faux si la table manque (le jeu tourne alors sans
## icônes SVG).
func load_table() -> bool:
	icons = {}
	fallbacks = {}
	_textures.clear()
	ink = _read_index(INK_INDEX_PATH, INK_DIR)
	entity = _read_index(ENTITY_INDEX_PATH, ENTITY_DIR)
	medallions = _read_index(MEDALLIONS_INDEX_PATH, MEDALLIONS_DIR)
	if not FileAccess.file_exists(TABLE_PATH):
		push_warning("IconLibrary: %s missing (run cent-ans assets icons)" % TABLE_PATH)
		return false
	var parsed: Variant = DataFile.parse_file(TABLE_PATH)
	if not (parsed is Dictionary):
		push_warning("IconLibrary: invalid %s" % TABLE_PATH)
		return false
	icons = parsed.get("icons", {})
	fallbacks = parsed.get("fallbacks", {})
	return true


## Index DA5 (`{"icons": {id: fichier}}`) → id → chemin `res://`, pour les seuls PNG présents.
func _read_index(path: String, directory: String) -> Dictionary:
	var result := {}
	if not FileAccess.file_exists(path):
		return result
	var parsed: Variant = DataFile.parse_file(path)
	if not (parsed is Dictionary):
		push_warning("IconLibrary: invalid %s" % path)
		return result
	var table: Variant = (parsed as Dictionary).get("icons", {})
	if table is Dictionary:
		for id in table:
			var file_path := directory.path_join(str(table[id]))
			if ResourceLoader.exists(file_path):
				result[str(id)] = file_path
	return result


func has_icon(id: String) -> bool:
	return icons.has(id) or ink.has(id) or entity.has(id)


## Vrai si `id` s'affiche avec une icône de la famille à l'encre (DA5).
func is_ink(id: String, category: String = "") -> bool:
	var resolved := resolve(id, category)
	return ink.has(resolved) and not entity.has(resolved)


## Vrai si `id` s'affiche avec une miniature d'entité peinte et encadrée (DA5b).
func is_entity(id: String, category: String = "") -> bool:
	return entity.has(resolve(id, category))


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
	if has_icon(id):
		return id
	var fallback_category := category if category != "" else category_of(id)
	var fallback: String = str(fallbacks.get(fallback_category, fallbacks.get("default", "")))
	if has_icon(fallback):
		return fallback
	return ""


## Chemin `res://` de l'icône (pour `[img]` en BBCode), "" si aucune : miniature d'entité (DA5b),
## sinon PNG à l'encre (DA5), sinon SVG.
func icon_path(id: String, category: String = "") -> String:
	var resolved := resolve(id, category)
	if resolved == "":
		return ""
	if entity.has(resolved):
		return str(entity[resolved])
	if ink.has(resolved):
		return str(ink[resolved])
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
	var resolved := resolve(id)
	if entity.has(resolved):
		return "Cent Ans (DA5b)"
	if ink.has(resolved):
		return "Cent Ans (DA5)"
	return str(icons.get(resolved, {}).get("author", ""))


## Modulation qui change l'encre cuite (`INK_COLOR`) en `color` : composantes > 1 permises
## (le canevas les applique telles quelles), alpha conservé.
static func tint(color: Color) -> Color:
	return Color(color.r / INK_COLOR.r, color.g / INK_COLOR.g, color.b / INK_COLOR.b, color.a)


## Couleurs d'icône d'un bouton selon l'état (bible § 8) : encre au repos, or au survol et
## enfoncé, encre pâlie désactivé.
static func apply_state_tints(button: Button) -> void:
	var gold := tint(GOLD_COLOR)
	button.add_theme_color_override("icon_normal_color", Color.WHITE)
	button.add_theme_color_override("icon_focus_color", Color.WHITE)
	button.add_theme_color_override("icon_hover_color", gold)
	button.add_theme_color_override("icon_pressed_color", gold)
	button.add_theme_color_override("icon_hover_pressed_color", gold)
	button.add_theme_color_override("icon_disabled_color", tint(FADED_COLOR))


## `TextureRect` carré prêt à insérer dans une ligne d'interface.
func make_rect(id: String, size: float = 20.0, category: String = "") -> TextureRect:
	var rect := TextureRect.new()
	rect.texture = get_icon(id, category)
	rect.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	rect.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
	rect.custom_minimum_size = Vector2(size, size)
	rect.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	rect.mouse_filter = Control.MOUSE_FILTER_IGNORE
	rect.texture_filter = CanvasItem.TEXTURE_FILTER_LINEAR_WITH_MIPMAPS
	rect.name = "Icon_%s" % id
	return rect


## Pose l'icône sur un bouton (taille bornée par `icon_max_width`) ; une icône à l'encre (DA5)
## passe à l'or au survol et enfoncée.
func decorate_button(button: Button, id: String, size: int = 20, category: String = "") -> void:
	button.icon = get_icon(id, category)
	button.expand_icon = false
	button.texture_filter = CanvasItem.TEXTURE_FILTER_LINEAR_WITH_MIPMAPS
	button.add_theme_constant_override("icon_max_width", size)
	if is_ink(id, category):
		apply_state_tints(button)
	elif is_entity(id, category):
		# DA5b : miniature 128 px ; `expand_icon` la cadre dans la hauteur du bouton (bornée par
		# `icon_max_width`) sans que sa taille native gonfle la taille minimale du bouton.
		button.expand_icon = true


## BBCode `[img]` de l'icône (infobulles riches), "" si aucune.
func bbcode(id: String, size: int = 18, category: String = "") -> String:
	var path := icon_path(id, category)
	if path == "":
		return ""
	return "[img=%dx%d]%s[/img]" % [size, size, path]


func clear_cache() -> void:
	_textures.clear()


# --- Médaillons enluminés (DA5) --------------------------------------------------------


func has_medallion(id: String) -> bool:
	return medallions.has(id)


## Texture du médaillon `id` (état normal), null si absent (l'appelant garde son repli).
func get_medallion(id: String) -> Texture2D:
	if not medallions.has(id):
		return null
	var path := str(medallions[id])
	if not _textures.has(path):
		_textures[path] = load(path) as Texture2D
	return _textures[path]


## États dérivés par code d'une seule image : `{normal, hover, pressed, disabled}` (éclairci,
## assombri, désaturé ; `Image.adjust_bcs`). Vide si le médaillon manque.
func medallion_states(id: String) -> Dictionary:
	var key := "states:" + id
	if _textures.has(key):
		return _textures[key]
	var base := get_medallion(id)
	if base == null:
		return {}
	var image := base.get_image()
	if image == null:
		return {}
	if image.is_compressed():
		image.decompress()
	image.clear_mipmaps()
	var states := {"normal": base}
	# [luminosité, contraste, saturation]
	var variants := {"hover": [1.12, 1.05, 1.08], "pressed": [0.82, 1.05, 1.0], "disabled": [0.95, 0.85, 0.0]}
	for state in variants:
		var copy := image.duplicate() as Image
		var bcs: Array = variants[state]
		copy.adjust_bcs(bcs[0], bcs[1], bcs[2])
		copy.generate_mipmaps()
		states[state] = ImageTexture.create_from_image(copy)
	_textures[key] = states
	return states


## Pose le médaillon `id` en icône d'un bouton existant (taille `size`) : états par modulation
## (éclairci au survol, assombri enfoncé, grisé et pâli désactivé). Faux si le médaillon
## manque : le bouton garde alors son icône.
func decorate_medallion(button: Button, id: String, size: int = 30) -> bool:
	var texture := get_medallion(id)
	if texture == null:
		return false
	button.icon = texture
	button.expand_icon = false
	button.texture_filter = CanvasItem.TEXTURE_FILTER_LINEAR_WITH_MIPMAPS
	button.add_theme_constant_override("icon_max_width", size)
	button.add_theme_color_override("icon_normal_color", Color.WHITE)
	button.add_theme_color_override("icon_focus_color", Color.WHITE)
	button.add_theme_color_override("icon_hover_color", Color(1.18, 1.14, 1.06))
	button.add_theme_color_override("icon_pressed_color", Color(0.80, 0.78, 0.76))
	button.add_theme_color_override("icon_hover_pressed_color", Color(0.90, 0.88, 0.85))
	button.add_theme_color_override("icon_disabled_color", Color(0.70, 0.68, 0.64, 0.6))
	return true
