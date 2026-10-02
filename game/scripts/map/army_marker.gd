class_name ArmyMarker
extends Node3D

## Marqueur d'armée (lot V3) : groupe de figurines (chef monté, fantassins,
## arbalétriers selon la composition, teintés aux couleurs de la faction), étendard aux
## armoiries (`map_banner.gdshader`), anneau au sol projeté sur le relief (`Decal`, couleur de
## la faction, doré et plus vif quand l'armée est sélectionnée). La plaque d'effectif est un
## contrôle 2D tenu par `ArmyMarkers` (taille constante à l'écran), ancré sur `plate_anchor()`.
## Mis à l'échelle par `ArmyMarkers` selon la distance caméra pour rester lisible à tout zoom.

## Au-delà de cette échelle, pas d'ombre portée (au dézoom, les figurines agrandies
## projetteraient des ombres démesurées).
const SHADOW_MAX_SCALE := 2.6
const RING_SELECTED := Color(1.0, 0.82, 0.3, 1.0)
## Bannières peintes (autre session) : `<id>_banner.png` (256×512) et spéciales ; `dragon.png`
## et les flammes `<id>_pennon.png` ne sont pas encore utilisées.
const BANNERS_DIR := "res://assets/heraldry/banners/"
const ROYAL_STANDARDS := {"fac_france": "oriflamme", "fac_england": "st_george"}
## Hauteur de la hampe de la scène, et hauteur portée pour une bannière verticale.
const POLE_HEIGHT := 8.4
const POLE_TALL := 11.0

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
## Lot EN : catégorie de relation avec le joueur (`StanceCues` : self, enemy, friend, other).
var cue: String = StanceCues.OTHER
## Position de base (centroïde de la province) et décalage unitaire (armées empilées).
var base_position: Vector3 = Vector3.ZERO
var offset_dir: Vector2 = Vector2.ZERO
var marker_scale: float = 1.0
## Mode de l'étendard (0 drapeau armorié, 1 flamme, 2 bannière verticale).
var standard_mode: int = 0

@onready var banner: MeshInstance3D = $Banner
@onready var pole: MeshInstance3D = $Pole
@onready var flag: MeshInstance3D = $Flag
@onready var selection: Decal = $Selection

var _flag_material: ShaderMaterial
## Lot CV2 : figurines animées (null en repli sur les figurines M10/V3).
var figures: ArmyFigures
## Hauteurs de repos de la hampe, du fleuron et du drapeau (avant décalage vers le porteur).
var _rest_y: Array = []
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
	province_id = str(army.get("location_province", army.get("location", "")))
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
	_apply_standard(standard_for(faction_id, army))
	(banner.material_override as StandardMaterial3D).albedo_color = color
	_rest_y = [pole.position.y, $Finial.position.y, flag.position.y]
	# Lot CV2 : figurines skinnées animées (général, porte-étendard, escorte), flotte, camp ;
	# repli sur les figurines M10/V3 (`--legacy-army-markers` ou modèles absents).
	var previous := get_node_or_null(ModelLibrary.MODEL_NODE)
	if previous != null:
		remove_child(previous)
		previous.queue_free()
	if ArmyFigures.enabled():
		figures = ArmyFigures.build(army, color, _heraldry(faction_id), id)
		add_child(figures)
		banner.visible = false
		face(Vector2(1.0, 0.45).normalized())
	else:
		figures = null
		ModelLibrary.dress_army_marker(self, army, color)
	if status == "embarked" and figures == null:
		# À bord : l'étendard flotte au mât de la cogue.
		pole.visible = false
		$Finial.visible = false
		flag.position = Vector3(0.1, 11.0 if standard_mode == 2 else 9.6, 0.0)
	_update_ring()
	# Lot CV3-4 : pastille de posture sur la hampe, fantôme en embuscade (armée du joueur).
	StanceBadge.apply(self, str(army.get("stance", "normal")), player)


## Étendard d'une armée : {texture, mode} (modes de `map_banner.gdshader`). Ordre de recherche :
## bannière spéciale de l'armée royale (`banners/oriflamme.png` pour le roi de France,
## `banners/st_george.png` pour celui d'Angleterre), bannière de faction
## (`banners/<id>_banner.png`), écu (`heraldry/<id>.png`, rogné), sinon couleur unie.
## Une texture plus large que haute est une flamme attachée à gauche (mode 1), sinon une
## bannière verticale (mode 2).
static func standard_for(faction: String, army: Dictionary) -> Dictionary:
	var candidates := PackedStringArray()
	var general := str(army.get("general", ""))
	if general != "" and general == ModelLibrary.faction_ruler(faction):
		var special: String = ROYAL_STANDARDS.get(faction, "")
		if special != "":
			candidates.append(BANNERS_DIR + special + ".png")
	if faction != "":
		candidates.append(BANNERS_DIR + faction + "_banner.png")
	for path in candidates:
		if ResourceLoader.exists(path):
			var texture := load(path) as Texture2D
			if texture != null:
				var wide := texture.get_width() > texture.get_height() * 1.2
				return {"texture": texture, "mode": 1 if wide else 2}
	return {"texture": _heraldry(faction), "mode": 0}


func _apply_standard(standard: Dictionary) -> void:
	var texture: Texture2D = standard.get("texture")
	standard_mode = int(standard.get("mode", 0))
	_flag_material.set_shader_parameter("mode", standard_mode)
	_flag_material.set_shader_parameter("has_heraldry", texture != null)
	if texture != null:
		_flag_material.set_shader_parameter("heraldry", texture)
	var quad := QuadMesh.new()
	if standard_mode == 2:
		# Bannière suspendue, un peu surdimensionnée, hampe plus haute.
		quad.size = Vector2(3.0, 6.0)
		quad.subdivide_width = 3
		quad.subdivide_depth = 10
		quad.center_offset = Vector3(0.0, -3.0, 0.0)
		flag.mesh = quad
		flag.position = Vector3(pole.position.x, POLE_TALL - 0.35, pole.position.z)
		pole.scale = Vector3(1.0, POLE_TALL / POLE_HEIGHT, 1.0)
		pole.position.y = POLE_TALL * 0.5
		$Finial.position.y = POLE_TALL + 0.15
	elif standard_mode == 1:
		var aspect := float(texture.get_height()) / maxf(texture.get_width(), 1.0)
		quad.size = Vector2(4.2, 4.2 * aspect)
		quad.subdivide_width = 12
		quad.subdivide_depth = 1
		quad.center_offset = Vector3(2.1, -2.1 * aspect, 0.0)
		flag.mesh = quad


## "siege", "moving", "embarked" ou "" (lecture de l'état exposé par la simulation).
static func army_status(army: Dictionary) -> String:
	if bool(army.get("embarked", false)) or bool(army.get("at_sea", false)):
		return "embarked"
	if str(army.get("stance", "")) == "siege":
		return "siege"
	if not (army.get("path", []) as Array).is_empty() or not (army.get("planned_path", PackedVector2Array()) as PackedVector2Array).is_empty():
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


## Lot EN : catégorie de relation avec le joueur ; l'anneau au sol des ennemis passe au rouge.
func set_cue(value: String) -> void:
	if value == cue:
		return
	cue = value
	_update_ring()


func _update_ring() -> void:
	if selection == null:
		return
	var stance_ring := StanceCues.army_ring(cue)
	if _selected:
		selection.modulate = RING_SELECTED
		selection.emission_energy = 1.1
	elif not stance_ring.is_empty():  # EN
		selection.modulate = stance_ring["color"]
		selection.emission_energy = float(stance_ring["emission"])
	else:
		var ring := faction_color
		ring.a = 0.5
		selection.modulate = ring
		selection.emission_energy = 0.08


## Oriente les figurines (en marche : vers la province suivante), `direction` en coordonnées carte.
func face(direction: Vector2) -> void:
	var model := get_node_or_null(ModelLibrary.MODEL_NODE) as Node3D
	if model == null or direction.length() < 0.01:
		return
	# Les modèles regardent +X ; Basis(UP, a) envoie X sur (cos a, 0, −sin a).
	model.rotation.y = atan2(-direction.y, direction.x)
	if figures != null:
		_follow_bearer()


## Lot CV2 : la hampe suit le porte-étendard (ou la hampe de poupe du navire amiral).
func _follow_bearer() -> void:
	var anchor := figures.bearer_anchor()
	if _rest_y.size() < 3:
		return
	pole.position = Vector3(anchor.x, float(_rest_y[0]) + anchor.y, anchor.z)
	$Finial.position = Vector3(anchor.x, float(_rest_y[1]) + anchor.y, anchor.z)
	flag.position = Vector3(anchor.x, float(_rest_y[2]) + anchor.y, anchor.z)


## Lot CV2 : marche (animation du déplacement) ou repos des figurines.
func set_walking(value: bool) -> void:
	if figures != null:
		figures.set_walking(value)


## Lot CV2 : niveau de détail et présence des figurines selon la distance caméra.
func set_view(camera_distance: float, weight: float) -> void:
	if figures != null:
		figures.set_view(camera_distance, weight)
		if figures.is_lord():  # CV3-5 : la hampe suit la main du général (fondu au loin)
			_follow_bearer()


## Point écran de référence pour le picking (milieu de l'étendard).
func pick_position() -> Vector3:
	return flag.global_position + Vector3(0.0, -2.1 if standard_mode == 2 else -1.0, 0.0) * marker_scale


## Points de picking : étendard et figurines.
func pick_positions() -> PackedVector3Array:
	return PackedVector3Array([pick_position(), global_position + Vector3(0.0, 1.5, 0.0) * marker_scale])


## Ancre de la plaque d'effectif : sous une bannière verticale (`plate_below()`), sinon
## au-dessus du drapeau.
func plate_anchor() -> Vector3:
	if standard_mode == 2:
		# Le tissu occupe les 352 px du haut (sur 512) : plaque sous le bord ondulé.
		return flag.global_position + Vector3(0.0, -4.3, 0.0) * marker_scale
	return flag.global_position + Vector3(0.0, 0.9, 0.0) * marker_scale


func plate_below() -> bool:
	return standard_mode == 2


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
