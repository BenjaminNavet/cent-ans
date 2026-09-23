class_name MenuBackground
extends Control

## F3 — fond illustré des écrans hors carte (menu de départ, chargement, crédits) : la carte
## ancienne `res://assets/ui/menu_map.jpg` (`cent-ans assets menu-art`) couvre l'écran,
## calée en haut (le cartouche reste visible), avec un lent mouvement de zoom (« respiration »).
## `menu_map.json` donne le rectangle du cartouche : `cartouche_rect()` le convertit en
## coordonnées d'écran pour placer l'interface autour (signal `layout_changed`). Sans image :
## fond brun uni.

signal layout_changed

const ART_PATH := "res://assets/ui/menu_map.jpg"
const SIDECAR_PATH := "res://assets/ui/menu_map.json"
const FALLBACK_COLOR := Color(0.36, 0.27, 0.17)
const BREATH_SECONDS := 50.0
const BREATH_ZOOM := 0.03

## Voile sombre par-dessus l'image (0 = aucun).
@export var dim: float = 0.0
@export var animated: bool = true

var texture: Texture2D = null
var _sidecar: Dictionary = {}
var _time := 0.0


func _ready() -> void:
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	set_anchors_preset(Control.PRESET_FULL_RECT)
	texture = PortraitLoader.load_texture(ART_PATH)
	if FileAccess.file_exists(SIDECAR_PATH):
		var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(SIDECAR_PATH))
		if parsed is Dictionary:
			_sidecar = parsed
	resized.connect(func() -> void:
		queue_redraw()
		layout_changed.emit())


func has_art() -> bool:
	return texture != null


func _process(delta: float) -> void:
	if animated and texture != null:
		_time += delta
		queue_redraw()


func _zoom() -> float:
	return 1.0 + BREATH_ZOOM * (0.5 - 0.5 * cos(_time * TAU / BREATH_SECONDS))


## Rectangle d'affichage de l'image : couvre le contrôle, calé en haut et à droite (le
## cartouche, en haut à droite de l'illustration, reste entier quelle que soit la fenêtre).
func image_rect() -> Rect2:
	if texture == null:
		return Rect2(Vector2.ZERO, size)
	var texture_size := texture.get_size()
	var scale := maxf(size.x / texture_size.x, size.y / texture_size.y) * _zoom()
	var drawn := texture_size * scale
	return Rect2(Vector2(size.x - drawn.x, 0.0), drawn)


## Cartouche « Cent Ans » en coordonnées du contrôle (vide sans image ni fiche).
func cartouche_rect() -> Rect2:
	var box: Array = _sidecar.get("cartouche", [])
	var art_size: Array = _sidecar.get("size", [])
	if texture == null or box.size() != 4 or art_size.size() != 2:
		return Rect2()
	var rect := image_rect()
	var factor := rect.size.x / float(art_size[0])
	return Rect2(rect.position + Vector2(box[0], box[1]) * factor, Vector2(float(box[2]) - float(box[0]), float(box[3]) - float(box[1])) * factor)


func _draw() -> void:
	if texture == null:
		draw_rect(Rect2(Vector2.ZERO, size), FALLBACK_COLOR)
	else:
		draw_texture_rect(texture, image_rect(), false)
	if dim > 0.0:
		draw_rect(Rect2(Vector2.ZERO, size), Color(0.08, 0.05, 0.02, dim))
