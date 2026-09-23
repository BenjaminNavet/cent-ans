class_name ArmyMarker
extends Node3D

## Marqueur d'armée : hampe + bannière (billboard, couleur de faction) + nombre d'unités,
## halo au sol quand l'armée est sélectionnée. Mis à l'échelle par `ArmyMarkers` selon
## la distance caméra pour rester lisible à tout zoom.

var army_id: String = ""
var province_id: String = ""
## Position de base (centroïde de la province) et décalage unitaire (armées empilées).
var base_position: Vector3 = Vector3.ZERO
var offset_dir: Vector2 = Vector2.ZERO

@onready var banner: MeshInstance3D = $Banner
@onready var pole: MeshInstance3D = $Pole
@onready var count_label: Label3D = $CountLabel
@onready var halo: MeshInstance3D = $Halo

var _banner_material: StandardMaterial3D


func _ready() -> void:
	_banner_material = banner.material_override.duplicate() as StandardMaterial3D
	banner.material_override = _banner_material
	halo.visible = false


func setup(id: String, army: Dictionary, color: Color, is_player: bool) -> void:
	army_id = id
	province_id = str(army.get("location", ""))
	_banner_material.albedo_color = color
	count_label.text = str(army.get("units", []).size())
	# Le joueur voit ses armées avec un liseré doré, les autres en sombre.
	count_label.outline_modulate = Color(0.95, 0.80, 0.30) if is_player else Color(0.10, 0.08, 0.05)
	name = "Army_" + id
	# M10 assets : porte-étendard 3D (ou cogue, + camp de siège) si le modèle existe.
	ModelLibrary.dress_army_marker(self, army, color)


func set_selected(selected: bool) -> void:
	halo.visible = selected


## Point écran de référence pour le picking (centre de la bannière).
func pick_position() -> Vector3:
	return banner.global_position


func apply_scale(marker_scale: float) -> void:
	scale = Vector3.ONE * marker_scale
	position = base_position + Vector3(offset_dir.x, 0.0, offset_dir.y) * (6.0 * marker_scale)
	# Le Label3D `fixed_size` ne doit pas hériter de l'échelle du marqueur.
	count_label.scale = Vector3.ONE / marker_scale
