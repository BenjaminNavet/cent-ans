class_name ArmyMarker
extends Node3D

## Marqueur d'armée (lot V3, façon Total War) : groupe de figurines (chef monté, fantassins,
## arbalétriers selon la composition, teintés aux couleurs de la faction), étendard aux
## armoiries (`map_banner.gdshader`), anneau au sol projeté sur le relief (`Decal`, couleur de
## la faction, doré et plus vif quand l'armée est sélectionnée). La plaque d'effectif est un
## contrôle 2D tenu par `ArmyMarkers` (taille constante à l'écran), ancré sur `plate_anchor()`.
## Mis à l'échelle par `ArmyMarkers` selon la distance caméra pour rester lisible à tout zoom.

## Au-delà de cette échelle, pas d'ombre portée (au dézoom, les figurines agrandies
## projetteraient des ombres démesurées).
const SHADOW_MAX_SCALE := 2.6
const RING_SELECTED := Color(1.0, 0.82, 0.3, 1.0)

var army_id: String = ""
var province_id: String = ""
var faction_id: String = ""
var is_player: bool = false
## Effectif total (somme des `strength` des unités) et nombre d'unités.
var men: int = 0
var unit_count: int = 0
## "siege", "moving", "embarked" ou "".
var status: String = ""
var faction_color: Color = Color.WHITE
## Position de base (centroïde de la province) et décalage unitaire (armées empilées).
var base_position: Vector3 = Vector3.ZERO
var offset_dir: Vector2 = Vector2.ZERO
var marker_scale: float = 1.0

@onready var banner: MeshInstance3D = $Banner
@onready var pole: MeshInstance3D = $Pole
@onready var flag: MeshInstance3D = $Flag
@onready var selection: Decal = $Selection

var _flag_material: ShaderMaterial
var _selected := false
var _shadows_on := true

static var _ring_textures: Dictionary = {}  # prémultipliée ? → ImageTexture


func _ready() -> void:
	_flag_material = flag.material_override.duplicate() as ShaderMaterial
	flag.material_override = _flag_material
	selection.texture_albedo = ring_texture(false)
	selection.texture_emission = ring_texture(true)
	selection.albedo_mix = 1.0
	selection.upper_fade = 0.3
	selection.lower_fade = 0.3
	_update_ring()


func setup(id: String, army: Dictionary, color: Color, player: bool) -> void:
	army_id = id
	province_id = str(army.get("location", ""))
	faction_id = str(army.get("faction", ""))
	is_player = player
	faction_color = color
	unit_count = 0
	men = 0
	for unit in army.get("units", []):
		unit_count += 1
		men += int(unit.get("strength", 0))
	status = army_status(army)
	name = "Army_" + id
	_flag_material.set_shader_parameter("faction_color", color)
	var heraldry := _heraldry(faction_id)
	_flag_material.set_shader_parameter("has_heraldry", heraldry != null)
	if heraldry != null:
		_flag_material.set_shader_parameter("heraldry", heraldry)
	(banner.material_override as StandardMaterial3D).albedo_color = color
	# Figurines 3D (ou cogue, + camp de siège) si les modèles existent.
	ModelLibrary.dress_army_marker(self, army, color)
	if status == "embarked":
		# À bord : l'étendard flotte au mât de la cogue.
		pole.visible = false
		$Finial.visible = false
		flag.position = Vector3(0.1, 9.6, 0.0)
	_update_ring()


## "siege", "moving", "embarked" ou "" (lecture de l'état exposé par la simulation).
static func army_status(army: Dictionary) -> String:
	if bool(army.get("embarked", false)) or bool(army.get("at_sea", false)):
		return "embarked"
	if str(army.get("stance", "")) == "siege":
		return "siege"
	if not (army.get("path", []) as Array).is_empty():
		return "moving"
	return ""


static func _heraldry(faction: String) -> Texture2D:
	if faction == "":
		return null
	var path := "res://assets/heraldry/%s.png" % faction
	return load(path) as Texture2D if ResourceLoader.exists(path) else null


func set_selected(selected: bool) -> void:
	_selected = selected
	_update_ring()


func _update_ring() -> void:
	if selection == null:
		return
	if _selected:
		selection.modulate = RING_SELECTED
		selection.emission_energy = 1.1
	else:
		var ring := faction_color
		ring.a = 0.75
		selection.modulate = ring
		selection.emission_energy = 0.35


## Oriente les figurines (en marche : vers la province suivante), `direction` en coordonnées carte.
func face(direction: Vector2) -> void:
	var model := get_node_or_null(ModelLibrary.MODEL_NODE) as Node3D
	if model == null or direction.length() < 0.01:
		return
	# Les modèles regardent +X ; Basis(UP, a) envoie X sur (cos a, 0, −sin a).
	model.rotation.y = atan2(-direction.y, direction.x)


## Point écran de référence pour le picking (milieu de l'étendard).
func pick_position() -> Vector3:
	return flag.global_position + Vector3(0.0, -1.0, 0.0) * marker_scale


## Points de picking : étendard et figurines.
func pick_positions() -> PackedVector3Array:
	return PackedVector3Array([pick_position(), global_position + Vector3(0.0, 1.5, 0.0) * marker_scale])


## Sommet de l'étendard : ancre de la plaque d'effectif.
func plate_anchor() -> Vector3:
	return flag.global_position + Vector3(0.0, 0.9, 0.0) * marker_scale


func apply_scale(new_scale: float) -> void:
	marker_scale = new_scale
	scale = Vector3.ONE * new_scale
	position = base_position + Vector3(offset_dir.x, 0.0, offset_dir.y) * (6.0 * new_scale)
	var shadows := new_scale <= SHADOW_MAX_SCALE
	if shadows != _shadows_on:
		_shadows_on = shadows
		var setting := GeometryInstance3D.SHADOW_CASTING_SETTING_ON if shadows else GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		var model := get_node_or_null(ModelLibrary.MODEL_NODE)
		if model != null:
			for child in model.find_children("*", "GeometryInstance3D", true, false):
				(child as GeometryInstance3D).cast_shadow = setting


## Anneau (texture partagée) : couronne douce, bord extérieur plus marqué.
static func ring_texture(premultiplied: bool) -> ImageTexture:
	if _ring_textures.has(premultiplied):
		return _ring_textures[premultiplied]
	var size := 128
	var image := Image.create(size, size, false, Image.FORMAT_RGBA8)
	var center := Vector2(size - 1, size - 1) * 0.5
	for y in size:
		for x in size:
			var r := Vector2(x, y).distance_to(center) / (size * 0.5)
			var band := smoothstep(0.66, 0.78, r) * (1.0 - smoothstep(0.9, 0.98, r))
			var inner := smoothstep(0.3, 0.8, r) * 0.1 * (1.0 - smoothstep(0.9, 0.98, r))
			var a := clampf(band + inner, 0.0, 1.0)
			# Émission (additive) prémultipliée : rien hors de l’anneau ; albédo blanc (alpha seul).
			var c := a if premultiplied else 1.0
			image.set_pixel(x, y, Color(c, c, c, a))
	image.generate_mipmaps()
	_ring_textures[premultiplied] = ImageTexture.create_from_image(image)
	return _ring_textures[premultiplied]


static func clear_cache() -> void:
	_ring_textures.clear()
