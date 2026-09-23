class_name PortraitLoader
extends RefCounted

## M10 assets : portraits (`res://assets/portraits/<character>.png`, OpenRouter) et écus
## (`res://assets/heraldry/<faction>.png`, procéduraux). Tout est facultatif : sans
## fichier, rien n'est ajouté et le placeholder (couleur + initiales) reste visible.
##
## Chargement : ressource importée si elle existe, sinon lecture directe du PNG
## (`Image.load_from_file`, utile juste après une génération sans réimport).

const PORTRAITS_DIR := "res://assets/portraits/"
const HERALDRY_DIR := "res://assets/heraldry/"
const OVERLAY_NAME := "M10Overlay"

static var _cache: Dictionary = {}  # chemin → Texture2D ou null


static func clear_cache() -> void:
	_cache.clear()


static func load_texture(path: String) -> Texture2D:
	if _cache.has(path):
		return _cache[path]
	var texture: Texture2D = null
	if ResourceLoader.exists(path):
		texture = load(path) as Texture2D
	if texture == null:
		var absolute := ProjectSettings.globalize_path(path)
		if FileAccess.file_exists(absolute):
			var image := Image.load_from_file(absolute)
			if image != null and not image.is_empty():
				texture = ImageTexture.create_from_image(image)
	_cache[path] = texture
	return texture


static func portrait_texture(character_id: String) -> Texture2D:
	if character_id == "":
		return null
	return load_texture(PORTRAITS_DIR + character_id + ".png")


static func heraldry_texture(faction_id: String) -> Texture2D:
	if faction_id == "":
		return null
	return load_texture(HERALDRY_DIR + faction_id + ".png")


static func has_portrait(character_id: String) -> bool:
	return portrait_texture(character_id) != null


## Pose `texture` par-dessus `target` (plein cadre, proportions gardées). Renvoie vrai si posé.
static func _overlay(target: Control, texture: Texture2D, min_size: Vector2) -> bool:
	var previous := target.get_node_or_null(OVERLAY_NAME)
	if previous != null:
		target.remove_child(previous)
		previous.queue_free()
	if texture == null:
		return false
	var rect := TextureRect.new()
	rect.name = OVERLAY_NAME
	rect.texture = texture
	rect.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	rect.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
	rect.mouse_filter = Control.MOUSE_FILTER_IGNORE
	rect.set_anchors_preset(Control.PRESET_FULL_RECT)
	target.add_child(rect)
	if min_size != Vector2.ZERO:
		target.custom_minimum_size = min_size
		var parent := target.get_parent() as Control
		if parent is PanelContainer:
			parent.custom_minimum_size = min_size
	return true


## Portrait peint du personnage ; à défaut, blason de sa faction (personnages générés).
static func overlay_portrait(target: Control, character_id: String, faction_id: String, min_size: Vector2 = Vector2.ZERO) -> bool:
	var texture := portrait_texture(character_id)
	if texture == null:
		texture = heraldry_texture(faction_id)
	var placed := _overlay(target, texture, min_size)
	# Les initiales du placeholder sont masquées sous une image.
	for child in target.get_children():
		if child is Label:
			(child as Label).visible = not placed
	return placed


## Écu de la faction par-dessus un carré de couleur (barre, diplomatie, menu).
static func overlay_heraldry(target: Control, faction_id: String, min_size: Vector2 = Vector2.ZERO) -> bool:
	var placed := _overlay(target, heraldry_texture(faction_id), min_size)
	if placed and target is ColorRect:
		(target as ColorRect).color = Color(0, 0, 0, 0)
	return placed
