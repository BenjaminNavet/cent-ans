class_name NavalShipView
extends Node3D

## Un navire de bataille navale (lot NV1), rendu seulement : modèle métrique
## (`assets/models/naval/*.glb`), voiles aux couleurs du camp gonflées par le vent, parapets et
## pavois teints, avirons qui rament, flamme au mât et bannière de poupe, équipage en figurines
## V2 sur le pont et dans les châteaux (archers et arbalétriers aux châteaux, hommes d'armes au
## milieu, massés contre le bord de l'abordage), cadavres sur le pont, feu et fumée, navire qui
## sombre. Toutes les valeurs viennent de `NavalBattleSim.get_ships()` ; la houle de `NavalWaves`.

const MODEL_DIR := "res://assets/models/naval/"
const SAIL_SHADER := preload("res://shaders/campaign_sail.gdshader")
const OAR_SHADER := preload("res://shaders/naval_oar.gdshader")
const BANNER_SHADER := preload("res://shaders/battle_banner.gdshader")
const HULL_SHADER := preload("res://shaders/naval_hull.gdshader")
const PAINT_SHADER := preload("res://shaders/naval_paint.gdshader")
const CANVAS := Color(0.78, 0.72, 0.6)
const TRIM_GOLD := Color(0.85, 0.7, 0.25)
const TRIM_SILVER := Color(0.82, 0.82, 0.8)
## Figurines dessinées au plus par groupe d'équipage, cadavres au plus par navire.
const MAX_FIGURES := 90
const MAX_CORPSES := 36
const SLOT_SPACING := 0.85
## Disposition du pont de chaque modèle (repère local, +X vers l'avant, mètres) :
## zones [centre x, longueur, largeur, hauteur du plancher], tête de mât, poupe.
const LAYOUT := {
	"cog": {"aft": [-8.6, 5.0, 5.0, 5.2], "fore": [9.0, 3.4, 3.6, 4.8], "waist": [-0.2, 10.8, 5.4, 2.8], "mast": [0.6, 23.6], "stern": [-11.4, 5.2], "draft": 2.2, "oars": [0.0, 0.0]},
	"nef": {"aft": [-10.6, 6.6, 6.6, 6.4], "fore": [11.2, 4.4, 4.8, 5.8], "waist": [-0.4, 13.6, 6.6, 3.4], "mast": [1.2, 28.2], "stern": [-14.2, 6.4], "draft": 2.6, "oars": [0.0, 0.0]},
	"galley": {"aft": [-16.0, 4.6, 3.8, 2.0], "fore": [16.4, 2.8, 4.0, 2.2], "waist": [0.0, 28.0, 1.0, 1.72], "mast": [5.4, 16.0], "stern": [-18.6, 2.0], "draft": 1.2, "oars": [1.8, 3.8]},
	"barge": {"aft": [-7.2, 3.4, 3.4, 2.8], "fore": [], "waist": [1.0, 11.0, 3.6, 1.6], "mast": [1.0, 16.4], "stern": [-9.2, 2.8], "draft": 1.4, "oars": [2.3, 2.2]},
}

var id: int = -1
var side: String = ""
var model_key: String = "cog"
var ship: Dictionary = {}
var color: Color = Color.WHITE
var heraldry: Texture2D = null
var captor_color: Color = Color.WHITE
var captor_heraldry: Texture2D = null
var length: float = 24.0
var beam: float = 7.5
## Pilonnement et assiette lissés (inertie de la coque).
var _heave: float = 0.0
var _pitch: float = 0.0
var _roll: float = 0.0
var _sink: float = 0.0  # 0 à flot, 1 coque sous l'eau
var _escaped_fade: float = 0.0
var _pivot: Node3D
var _model: Node3D
var _sail_mats: Array[ShaderMaterial] = []
var _oar_mat: ShaderMaterial = null
var _surface_mats: Array[ShaderMaterial] = []  # bois et peinture (brûlures)
var _charred: float = 0.0
var _oar_phase: float = 0.0
var _flag_pivot: Node3D
var _flag_mat: ShaderMaterial
var _ensign_mat: ShaderMaterial
var _ring: MeshInstance3D
var _groups: Array = []  # [{unit_type, kind, variant, mm, mat, men, state, ranged, corpses}]
var _visitors: Dictionary = {}  # clé -> {mm, mat, count}
var _corpse_layers: Dictionary = {}  # clé kind/variant -> {mm, data, count, mat}
var _slots: Dictionary = {}  # zone -> Array[Vector3]
var _layout_key: String = ""
var _fire: Node3D = null
var _flames: GPUParticles3D = null
var _smoke: GPUParticles3D = null
var _fire_light: OmniLight3D = null
var _rng := RandomNumberGenerator.new()
var _last_shot_time: float = -1000.0
var _selected: bool = false


## `units` : les régiments du camp (setup), pour le type d'unité de chaque groupe d'équipage.
func setup(p_ship: Dictionary, units: Array, p_color: Color, p_heraldry: Texture2D) -> void:
	ship = p_ship
	id = int(ship["id"])
	side = str(ship["side"])
	name = "Ship%d" % id
	color = p_color
	heraldry = p_heraldry
	model_key = str(ship.get("model", "cog"))
	if not LAYOUT.has(model_key):
		model_key = "cog"
	length = float(ship.get("length", 24.0))
	beam = float(ship.get("beam", 7.5))
	_rng.seed = 7919 * (id + 1)
	_pivot = Node3D.new()
	_pivot.name = "Pivot"
	add_child(_pivot)
	_build_model()
	_build_flags()
	_build_ring()
	_build_slots()
	for crew in ship.get("crew", []):
		var unit_index := int(crew["unit"])
		var unit_type := str((units[unit_index] as Dictionary).get("unit_type", "")) if unit_index < units.size() else ""
		_add_group(unit_type, crew)
	# Marins : figurines de milice, peu nombreuses (manœuvre, pas de combat).
	_add_group("unit_urban_militia", {"unit": -1, "men": ship.get("sailors", 0.0), "initial": ship.get("sailors_initial", 0.0), "ranged": false, "sailors": true})
	_apply_transform(0.0, null, 0.0, true)


func _build_model() -> void:
	var path := MODEL_DIR + model_key + ".glb"
	if not ResourceLoader.exists(path):
		push_warning("NavalShipView: missing model %s" % path)
		return
	_model = (load(path) as PackedScene).instantiate() as Node3D
	_model.name = "Model"
	_pivot.add_child(_model)
	_dress(_model, color, heraldry)


## Voiles (toile teinte, armes au centre), parapets et pavois, avirons ; coques en bordé à
## clin goudronné et châteaux peints patinés (NV2, `naval_hull` et `naval_paint`).
func _dress(root: Node, p_color: Color, arms: Texture2D) -> void:
	_sail_mats.clear()
	var layout: Dictionary = LAYOUT[model_key]
	var oars: Array = layout["oars"]
	var wood_mats := _wood_materials()
	var paint_mats := _paint_materials(p_color)
	for child in root.find_children("*", "MeshInstance3D", true, false):
		var mesh_instance := child as MeshInstance3D
		if mesh_instance.mesh == null:
			continue
		var dressed: Mesh = mesh_instance.mesh.duplicate() as Mesh
		for surface in dressed.get_surface_count():
			var material := dressed.surface_get_material(surface)
			if material == null:
				continue
			match material.resource_name:
				"Sail":
					var sail := ShaderMaterial.new()
					sail.shader = SAIL_SHADER
					sail.set_shader_parameter("faction_color", CANVAS.lerp(p_color, 0.55))
					sail.set_shader_parameter("heraldry", arms)
					sail.set_shader_parameter("has_heraldry", arms != null)
					sail.set_shader_parameter("belly", 1.2)
					sail.set_shader_parameter("breathe", 0.12)
					dressed.surface_set_material(surface, sail)
					_sail_mats.append(sail)
				"Banner", "Shield":
					dressed.surface_set_material(surface, paint_mats[material.resource_name])
				"Hull", "Wood", "Deck":
					dressed.surface_set_material(surface, wood_mats[material.resource_name])
				"Oar":
					if _oar_mat == null:
						_oar_mat = ShaderMaterial.new()
						_oar_mat.shader = OAR_SHADER
						_oar_mat.set_shader_parameter("pivot_y", float(oars[0]))
						_oar_mat.set_shader_parameter("half_width", float(oars[1]))
					dressed.surface_set_material(surface, _oar_mat)
		mesh_instance.mesh = dressed


## Bois du navire : coque (bordé à clin, goudron sous la flottaison, bande mouillée), bordages
## et châteaux (même chêne, un ton plus clair), pont (planches calfatées). Graine : le navire.
func _wood_materials() -> Dictionary:
	var out := {}
	var variation := float(id % 17) / 17.0
	for key in ["Hull", "Wood", "Deck"]:
		var mat := ShaderMaterial.new()
		mat.shader = HULL_SHADER
		mat.set_shader_parameter("seed", float(id) + 0.37)
		mat.set_shader_parameter("deck", key == "Deck")
		mat.set_shader_parameter("tone", (0.92 if key == "Hull" else 1.05) + 0.12 * variation)
		mat.set_shader_parameter("wear", 0.35 + 0.4 * variation)
		# Les galères, basses, ont la flottaison plus près du plat-bord.
		mat.set_shader_parameter("waterline", 0.2 if model_key == "galley" else 0.3)
		mat.set_shader_parameter("wet_band", 0.45 if model_key == "galley" else 0.7)
		out[key] = mat
		_surface_mats.append(mat)
	return out


## Parapets des châteaux (panneaux alternés, liseré) et pavois, peints aux couleurs du camp.
func _paint_materials(p_color: Color) -> Dictionary:
	var out := {}
	var accent := TRIM_GOLD if p_color.get_luminance() < 0.45 else p_color.darkened(0.45)
	for key in ["Banner", "Shield"]:
		var mat := ShaderMaterial.new()
		mat.shader = PAINT_SHADER
		mat.set_shader_parameter("paint", p_color if key == "Banner" else p_color.darkened(0.15))
		mat.set_shader_parameter("accent", accent)
		mat.set_shader_parameter("panels", key == "Banner")
		mat.set_shader_parameter("seed", float(id) * 1.3 + (0.0 if key == "Banner" else 7.0))
		mat.set_shader_parameter("fade", 0.18 + 0.14 * float(id % 5) / 4.0)
		mat.set_shader_parameter("chipping", 0.35 + 0.25 * float(id % 3) / 2.0)
		out[key] = mat
		_surface_mats.append(mat)
	return out


## Flamme au mât (pointe dans le vent) et grande bannière de poupe aux armes du camp.
func _build_flags() -> void:
	var layout: Dictionary = LAYOUT[model_key]
	var mast: Array = layout["mast"]
	_flag_pivot = Node3D.new()
	_flag_pivot.name = "MastFlag"
	_flag_pivot.position = Vector3(float(mast[0]), float(mast[1]), 0.0)
	_pivot.add_child(_flag_pivot)
	var pennant := MeshInstance3D.new()
	pennant.mesh = BattleMeshes.flag(7.0, 1.1)
	_flag_mat = _flag_material(color, null, 7.0)
	pennant.material_override = _flag_mat
	pennant.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	_flag_pivot.add_child(pennant)
	var stern: Array = layout["stern"]
	var ensign_pole := MeshInstance3D.new()
	ensign_pole.mesh = BattleMeshes.pole()
	ensign_pole.scale = Vector3(1.2, 5.5, 1.2)
	ensign_pole.position = Vector3(float(stern[0]), float(stern[1]), 0.0)
	_pivot.add_child(ensign_pole)
	var ensign := MeshInstance3D.new()
	ensign.name = "Ensign"
	ensign.mesh = BattleMeshes.flag(3.6, 2.4)
	_ensign_mat = _flag_material(color, heraldry, 3.6)
	ensign.material_override = _ensign_mat
	ensign.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	ensign.position = Vector3(float(stern[0]), float(stern[1]) + 5.4, 0.0)
	ensign.rotation.y = PI  # flotte vers l'arrière
	_pivot.add_child(ensign)


func _flag_material(p_color: Color, arms: Texture2D, flag_length: float) -> ShaderMaterial:
	var mat := ShaderMaterial.new()
	mat.shader = BANNER_SHADER
	mat.set_shader_parameter("livery", p_color)
	mat.set_shader_parameter("heraldry", arms)
	mat.set_shader_parameter("has_heraldry", arms != null)
	mat.set_shader_parameter("flag_length", flag_length)
	mat.set_shader_parameter("phase", float(id) * 1.3)
	return mat


func _build_ring() -> void:
	_ring = MeshInstance3D.new()
	_ring.name = "SelectionRing"
	var torus := TorusMesh.new()
	torus.inner_radius = 0.94
	torus.outer_radius = 1.0
	torus.rings = 48
	torus.ring_segments = 4
	_ring.mesh = torus
	_ring.scale = Vector3(length * 0.62, 0.4, beam * 0.95)
	_ring.position.y = 0.3
	var mat := StandardMaterial3D.new()
	mat.albedo_color = Color(1.0, 0.85, 0.25)
	mat.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	mat.no_depth_test = true
	_ring.material_override = mat
	_ring.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	_ring.visible = false
	add_child(_ring)


func set_selected(value: bool) -> void:
	_selected = value
	_ring.visible = value and visible


func set_ring_color(value: Color) -> void:
	(_ring.material_override as StandardMaterial3D).albedo_color = value


# --- Équipage -----------------------------------------------------------------------------


## Places du pont par zone (aft, fore, waist), en quinconce légèrement bruité.
func _build_slots() -> void:
	var layout: Dictionary = LAYOUT[model_key]
	for zone in ["aft", "fore", "waist"]:
		var rect: Array = layout[zone]
		var slots: Array[Vector3] = []
		if rect.size() == 4:
			var cx := float(rect[0])
			var len_x := float(rect[1])
			var width := float(rect[2])
			var y := float(rect[3])
			var cols := maxi(int(len_x / SLOT_SPACING), 1)
			var rows := maxi(int(width / SLOT_SPACING), 1)
			for r in rows:
				for c in cols:
					var x := cx - len_x * 0.5 + (float(c) + 0.5 + (0.25 if r % 2 == 1 else 0.0)) * len_x / cols
					var z := -width * 0.5 + (float(r) + 0.5) * width / rows
					x += _rng.randf_range(-0.12, 0.12)
					z += _rng.randf_range(-0.12, 0.12)
					# Pas de figurine dans le mât.
					if absf(x - float(layout["mast"][0])) < 0.5 and absf(z) < 0.5:
						continue
					slots.append(Vector3(x, y, z))
		_slots[zone] = slots


func _add_group(unit_type: String, crew: Dictionary) -> void:
	var kind := BattleMeshes.figure_kind_of(unit_type)
	var variant := BattleMeshes.variant_of(unit_type)
	if kind == "" or kind == "cavalry" or kind == "siege":
		kind = "archer" if bool(crew.get("ranged", false)) else "infantry"
		variant = 0 if kind == "archer" else 2
	if not BattleSkinned.has_figure(kind, variant):
		variant = 0
	if not BattleSkinned.has_figure(kind, variant):
		return
	var mat := _figure_material(kind, variant, color, heraldry)
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.mesh = BattleSkinned.mesh(kind, variant, 1)
	mm.instance_count = MAX_FIGURES
	mm.visible_instance_count = 0
	var instance := MultiMeshInstance3D.new()
	instance.name = "Crew_%s" % unit_type
	instance.multimesh = mm
	instance.material_override = mat
	instance.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	_pivot.add_child(instance)
	var sailors := bool(crew.get("sailors", false))
	_groups.append({
		"unit_type": unit_type, "kind": kind, "variant": variant, "mm": mm, "mat": mat,
		"men": float(crew.get("men", 0.0)), "shown": -1, "state": "", "ranged": bool(crew.get("ranged", false)),
		"sailors": sailors, "reload": 9.0 if str(crew.get("missile", "")) == "bolt" else 6.0,
	})


func _figure_material(kind: String, variant: int, p_color: Color, arms: Texture2D) -> ShaderMaterial:
	var mat := ShaderMaterial.new()
	mat.shader = BattleSkinned.SHADER
	mat.set_shader_parameter("livery", p_color)
	mat.set_shader_parameter("trim", TRIM_SILVER if p_color.get_luminance() > 0.55 else TRIM_GOLD)
	mat.set_shader_parameter("heraldry", arms)
	mat.set_shader_parameter("has_heraldry", arms != null)
	BattleSkinned.setup_material(mat, kind, variant)
	mat.set_shader_parameter("livery_share", 0.85 if BattleSkinned.is_noble(kind, variant) else 0.6)
	mat.set_shader_parameter("reload_time", 6.0)
	BattleSkinned.apply_config(mat, BattleSkinned.state_config(kind, variant, "idle", false), 0.0)
	return mat


## Direction locale (plan x-z du navire) vers un point du monde.
func _local_dir(world: Vector2) -> Vector2:
	var h := float(ship.get("heading", 0.0))
	var d := world - Vector2(float(ship["x"]), float(ship["z"]))
	var fwd := d.x * cos(h) + d.y * sin(h)
	var across := -d.x * sin(h) + d.y * cos(h)
	return Vector2(fwd, across).normalized() if d.length() > 0.5 else Vector2(1.0, 0.0)


## Replace les figurines : nombre d'hommes, état (tir, mêlée, repos), bord de l'abordage.
## `focus` : point du monde vers lequel l'équipage regarde (cible, navire grappiné) ou null.
func _layout_crew(anim_time: float, focus: Variant, melee: bool) -> void:
	var dir := _local_dir(focus) if focus != null else Vector2(1.0, 0.0)
	# Tirage fixe : une nouvelle disposition ne fait pas sauter les figurines restées en place.
	var rng := RandomNumberGenerator.new()
	rng.seed = 104729 * (id + 1)
	var waist: Array[Vector3] = (_slots["waist"] as Array[Vector3]).duplicate()
	if melee:
		# Mêlée : le plus près possible du bord de l'ennemi.
		waist.sort_custom(func(a: Vector3, b: Vector3) -> bool: return a.x * dir.x + a.z * dir.y > b.x * dir.x + b.z * dir.y)
	var castles: Array[Vector3] = []
	castles.append_array(_slots["aft"])
	castles.append_array(_slots["fore"])
	castles.sort_custom(func(a: Vector3, b: Vector3) -> bool: return a.x * dir.x + a.z * dir.y > b.x * dir.x + b.z * dir.y)
	var castle_used := 0
	var waist_used := 0
	var sinking := str(ship.get("status", "")) in ["sinking", "sunk"]
	var shooting := anim_time - _last_shot_time < 10.0
	# Tireurs d'abord (châteaux), puis combattants, puis marins.
	var order: Array = _groups.duplicate()
	order.sort_custom(func(a: Dictionary, b: Dictionary) -> bool: return _group_rank(a) < _group_rank(b))
	for group in order:
		var n := mini(int(ceil(float(group["men"]) * _figure_share(group))), MAX_FIGURES)
		var state := "idle"
		if sinking:
			state = "routing"
		elif bool(group["ranged"]) and shooting:
			state = "shooting"
		elif melee and not bool(group["sailors"]):
			state = "melee"
		var buffer := PackedFloat32Array()
		buffer.resize(MAX_FIGURES * 12)
		var placed := 0
		for i in n:
			var pos: Vector3
			if bool(group["ranged"]) and castle_used < castles.size():
				pos = castles[castle_used]
				castle_used += 1
			elif waist_used < waist.size():
				pos = waist[waist_used]
				waist_used += 1
			else:
				break
			var face := dir
			if state == "idle" and not bool(group["ranged"]):
				face = Vector2(dir.x + rng.randf_range(-0.6, 0.6), dir.y + rng.randf_range(-0.6, 0.6))
			var yaw := atan2(face.x, face.y) + rng.randf_range(-0.2, 0.2)
			var basis := Basis(Vector3.UP, yaw)
			var o := placed * 12
			buffer[o + 0] = basis.x.x
			buffer[o + 1] = basis.y.x
			buffer[o + 2] = basis.z.x
			buffer[o + 3] = pos.x
			buffer[o + 4] = basis.x.y
			buffer[o + 5] = basis.y.y
			buffer[o + 6] = basis.z.y
			buffer[o + 7] = pos.y
			buffer[o + 8] = basis.x.z
			buffer[o + 9] = basis.y.z
			buffer[o + 10] = basis.z.z
			buffer[o + 11] = pos.z
			placed += 1
		var mm: MultiMesh = group["mm"]
		mm.buffer = buffer
		mm.visible_instance_count = placed
		group["shown"] = placed
		if state != str(group["state"]):
			BattleSkinned.apply_config(group["mat"], BattleSkinned.state_config(group["kind"], group["variant"], state, false), anim_time)
			group["state"] = state


static func _group_rank(group: Dictionary) -> int:
	if bool(group["sailors"]):
		return 2
	return 0 if bool(group["ranged"]) else 1


## Marins : un sur trois dessiné ; soldats : tous (plafond MAX_FIGURES).
static func _figure_share(group: Dictionary) -> float:
	return 0.35 if bool(group["sailors"]) else 1.0


## Groupe d'un autre camp à bord (abordeurs, équipage de prise).
func set_visitors(key: String, unit_type: String, p_color: Color, arms: Texture2D, count: int, anim_time: float, focus: Variant, state: String) -> void:
	if count <= 0:
		if _visitors.has(key):
			(_visitors[key]["mm"] as MultiMesh).visible_instance_count = 0
		return
	if not _visitors.has(key):
		var kind := BattleMeshes.figure_kind_of(unit_type)
		var variant := BattleMeshes.variant_of(unit_type)
		if kind == "" or not BattleSkinned.has_figure(kind, variant):
			kind = "infantry"
			variant = 0
		var mat := _figure_material(kind, variant, p_color, arms)
		var mm := MultiMesh.new()
		mm.transform_format = MultiMesh.TRANSFORM_3D
		mm.mesh = BattleSkinned.mesh(kind, variant, 1)
		mm.instance_count = 40
		var instance := MultiMeshInstance3D.new()
		instance.name = "Visitors_%s" % key
		instance.multimesh = mm
		instance.material_override = mat
		instance.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		_pivot.add_child(instance)
		_visitors[key] = {"mm": mm, "mat": mat, "kind": kind, "variant": variant, "count": -1, "state": ""}
	var visitor: Dictionary = _visitors[key]
	count = mini(count, 40)
	if str(visitor["state"]) != state:
		BattleSkinned.apply_config(visitor["mat"], BattleSkinned.state_config(visitor["kind"], visitor["variant"], state, false), anim_time)
		visitor["state"] = state
	if int(visitor["count"]) == count:
		return
	visitor["count"] = count
	var dir := _local_dir(focus) if focus != null else Vector2(1.0, 0.0)
	var waist: Array[Vector3] = (_slots["waist"] as Array[Vector3]).duplicate()
	# Les abordeurs arrivent par le bord de leur navire et font face à l'intérieur.
	waist.sort_custom(func(a: Vector3, b: Vector3) -> bool: return a.x * dir.x + a.z * dir.y > b.x * dir.x + b.z * dir.y)
	var mm: MultiMesh = visitor["mm"]
	var shown := mini(count, waist.size())
	for i in shown:
		var pos := waist[i] + Vector3(_rng.randf_range(-0.25, 0.25), 0.0, _rng.randf_range(-0.25, 0.25))
		var yaw := atan2(-dir.x, -dir.y) + _rng.randf_range(-0.4, 0.4)
		mm.set_instance_transform(i, Transform3D(Basis(Vector3.UP, yaw), pos))
	mm.visible_instance_count = shown


## Morts sur le pont : cadavres (clip de mort) aux places que l'équipage vient de quitter.
func _add_corpses(group: Dictionary, count: int, anim_time: float) -> void:
	var key := "%s/%d" % [group["kind"], group["variant"]]
	if not _corpse_layers.has(key):
		var mat := _figure_material(group["kind"], group["variant"], color, heraldry)
		BattleSkinned.apply_config(mat, BattleSkinned.death_config(group["kind"], group["variant"]), anim_time)
		var mm := MultiMesh.new()
		mm.transform_format = MultiMesh.TRANSFORM_3D
		mm.use_custom_data = true
		mm.mesh = BattleSkinned.mesh(group["kind"], group["variant"], 1)
		mm.instance_count = MAX_CORPSES
		mm.visible_instance_count = 0
		var instance := MultiMeshInstance3D.new()
		instance.name = "Corpses_%s" % key.replace("/", "_")
		instance.multimesh = mm
		instance.material_override = mat
		instance.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		_pivot.add_child(instance)
		_corpse_layers[key] = {"mm": mm, "mat": mat, "count": 0, "death_count": (BattleSkinned.death_config(group["kind"], group["variant"])["set"] as Array).size()}
	var layer: Dictionary = _corpse_layers[key]
	var mm: MultiMesh = layer["mm"]
	var all_slots: Array[Vector3] = []
	all_slots.append_array(_slots["waist"])
	all_slots.append_array(_slots["aft"])
	all_slots.append_array(_slots["fore"])
	if all_slots.is_empty():
		return
	for i in count:
		var slot := int(layer["count"])
		if slot >= MAX_CORPSES:
			slot = _rng.randi_range(0, MAX_CORPSES - 1)
		else:
			layer["count"] = slot + 1
		var pos := all_slots[_rng.randi_range(0, all_slots.size() - 1)]
		var yaw := _rng.randf_range(-PI, PI)
		mm.set_instance_transform(slot, Transform3D(Basis(Vector3.UP, yaw), pos))
		mm.set_instance_custom_data(slot, Color(anim_time + _rng.randf_range(0.0, 0.8), float(_rng.randi_range(0, int(layer["death_count"]) - 1)), 0.0, 0.0))
	mm.visible_instance_count = int(layer["count"])
	(layer["mat"] as ShaderMaterial).set_shader_parameter("anim_time", anim_time)


## Point de départ des traits tirés par ce navire (châteaux) : pour les volées.
func deck_height() -> float:
	var layout: Dictionary = LAYOUT[model_key]
	var aft: Array = layout["aft"]
	return (float(aft[3]) if aft.size() == 4 else float(layout["waist"][3])) + _heave - _sink * 12.0


## Point du bord (monde) tourné vers `toward`, à la fraction `t` (-0,5 à 0,5) de la longueur,
## `lift` mètres au-dessus du pont principal.
func rail_point(t: float, toward: Vector3, lift: float) -> Vector3:
	var dir := _local_dir(Vector2(toward.x, toward.z))
	var side := 1.0 if dir.y >= 0.0 else -1.0
	var waist: Array = LAYOUT[model_key]["waist"]
	var half := maxf(float(waist[2]) * 0.5, beam * 0.42)
	var local := Vector3(float(waist[0]) + t * float(waist[1]), float(waist[3]) + lift, side * half)
	return _pivot.global_transform * local


func on_shot(anim_time: float) -> void:
	_last_shot_time = anim_time
	for group in _groups:
		if bool(group["ranged"]):
			(group["mat"] as ShaderMaterial).set_shader_parameter("volley_time", anim_time)


# --- Mise à jour --------------------------------------------------------------------------


## `anim_time` : horloge de rendu (temps de bataille lissé) ; `sea` : la mer (houle) ;
## `focus` : point du monde visé (Vector2) ou null ; `melee` : abordage en cours.
func update_view(p_ship: Dictionary, anim_time: float, dt: float, sea: NavalSea, wind_to: float, wind_strength: float, focus: Variant, melee: bool) -> void:
	var previous := ship
	ship = p_ship
	var status := str(ship.get("status", "afloat"))
	# Hommes tombés : cadavres (et le reste à l'eau), puis nouvelle disposition.
	var relayout := false
	var crew: Array = ship.get("crew", [])
	for i in _groups.size():
		var group: Dictionary = _groups[i]
		var men := float(ship.get("sailors", 0.0)) if bool(group["sailors"]) else (float((crew[i] as Dictionary).get("men", 0.0)) if i < crew.size() else 0.0)
		var lost := int(floor(float(group["men"]))) - int(floor(men))
		if lost > 0 and status in ["afloat", "captured", "abandoned"] and not bool(group["sailors"]):
			_add_corpses(group, mini(int(ceil(lost * 0.6)), 6), anim_time)
		if int(ceil(men)) != int(ceil(float(group["men"]))):
			relayout = true
		group["men"] = men
		(group["mat"] as ShaderMaterial).set_shader_parameter("anim_time", anim_time)
	for key in _visitors:
		(_visitors[key]["mat"] as ShaderMaterial).set_shader_parameter("anim_time", anim_time)
	for key in _corpse_layers:
		(_corpse_layers[key]["mat"] as ShaderMaterial).set_shader_parameter("anim_time", anim_time)
	var focus_key := "" if focus == null else str(Vector2i((focus as Vector2) / 15.0))
	var key := "%s|%s|%s|%s" % [focus_key, melee, status, anim_time - _last_shot_time < 10.0]
	if relayout or key != _layout_key or previous.is_empty():
		_layout_key = key
		_layout_crew(anim_time, focus, melee)
	# Prise : couleurs du vainqueur au mât et à la poupe.
	if status == "captured" and str(previous.get("status", "")) != "captured":
		_flag_mat.set_shader_parameter("livery", captor_color)
		_ensign_mat.set_shader_parameter("livery", captor_color)
		_ensign_mat.set_shader_parameter("heraldry", captor_heraldry)
		_ensign_mat.set_shader_parameter("has_heraldry", captor_heraldry != null)
	# Voiles : gonflées si le vent vient de l'arrière, faseyantes sinon ; carguées à l'arrêt
	# (navires enchaînés, pris).
	var heading := float(ship.get("heading", 0.0))
	var align := cos(wind_to - heading)
	var belly := (0.25 + 1.35 * clampf(align * 0.5 + 0.5, 0.0, 1.0)) * (0.4 + wind_strength)
	if int(ship.get("chain", -1)) >= 0 or status in ["captured", "abandoned"]:
		belly *= 0.25
	for mat in _sail_mats:
		mat.set_shader_parameter("belly", belly)
	# Avirons : cadence selon la vitesse ; rentrés si le navire est pris ou abandonné.
	if _oar_mat != null:
		var rowing := status == "afloat" and float(ship.get("speed", 0.0)) > 0.3
		var rate := clampf(float(ship.get("speed", 0.0)) / 3.0, 0.0, 1.0) * 0.45 if rowing else 0.0
		_oar_phase += dt * TAU * rate
		_oar_mat.set_shader_parameter("stroke_rate", rate)
		_oar_mat.set_shader_parameter("stroke_phase", _oar_phase)
		_oar_mat.set_shader_parameter("shipped", 0.0 if status == "afloat" else 1.0)
	_update_fire(float(ship.get("fire", 0.0)), wind_to, wind_strength)
	_apply_transform(dt, sea, wind_to, false)
	# Flamme du mât : pointe vers où souffle le vent.
	_flag_pivot.global_basis = Basis(Vector3.UP, -wind_to)
	_flag_mat.set_shader_parameter("routing", bool(ship.get("fleeing", false)))


func _apply_transform(dt: float, sea: NavalSea, _wind_to: float, snap: bool) -> void:
	var x := float(ship["x"])
	var z := float(ship["z"])
	var heading := float(ship.get("heading", 0.0))
	position = Vector3(x, 0.0, z)
	rotation = Vector3(0.0, -heading, 0.0)
	var status := str(ship.get("status", "afloat"))
	var heave := 0.0
	var pitch := 0.0
	var roll := 0.0
	if sea != null:
		var fx := cos(heading)
		var fz := sin(heading)
		var half := length * 0.4
		var side_off := beam * 0.5
		var bow := sea.height_at(x + fx * half, z + fz * half)
		var stern := sea.height_at(x - fx * half, z - fz * half)
		var port := sea.height_at(x + fz * side_off, z - fx * side_off)
		var starboard := sea.height_at(x - fz * side_off, z + fx * side_off)
		heave = (bow + stern + port + starboard) * 0.25
		pitch = atan2(bow - stern, half * 2.0) * 0.8
		roll = atan2(port - starboard, side_off * 2.0) * 0.6
	# Coque percée : gîte et enfoncement.
	var damage := 1.0 - clampf(float(ship.get("hull", 1.0)) / maxf(float(ship.get("hull_max", 1.0)), 1.0), 0.0, 1.0)
	roll += damage * 0.1 * (1.0 if id % 2 == 0 else -1.0)
	heave -= damage * 0.6
	if status in ["sinking", "sunk"]:
		_sink = minf(_sink + dt / (30.0 if status == "sinking" else 10.0), 1.3)
	var k := 1.0 if snap else clampf(dt * 2.5, 0.0, 1.0)
	_heave = lerpf(_heave, heave, k)
	_pitch = lerpf(_pitch, pitch, k)
	_roll = lerpf(_roll, roll, k)
	# Naufrage : la poupe s'enfonce, la proue se dresse, la coque disparaît.
	var sink_depth := _sink * (float(LAYOUT[model_key]["draft"]) + float(ship.get("freeboard", 2.0)) + 14.0)
	var sink_pitch := _sink * 0.35
	var sink_roll := _sink * 0.25 * (1.0 if id % 2 == 0 else -1.0)
	_pivot.position = Vector3(0.0, _heave - sink_depth, 0.0)
	_pivot.rotation = Vector3(_roll + sink_roll, 0.0, _pitch + sink_pitch)
	if status == "escaped":
		_escaped_fade += dt
	visible = _sink < 1.25 and _escaped_fade < 4.0
	_ring.visible = _selected and visible
	if _fire != null:
		_fire.visible = _sink < 0.6


func _update_fire(fire: float, wind_to: float, wind_strength: float) -> void:
	# Le feu laisse le bois noirci (ne s'efface pas).
	if fire > _charred + 0.02:
		_charred = fire
		for mat in _surface_mats:
			mat.set_shader_parameter("charred", _charred)
	if fire <= 0.02 and _fire == null:
		return
	if _fire == null:
		_fire = NavalFire.make(length, beam)
		_fire.position.y = float((LAYOUT[model_key]["waist"] as Array)[3])
		_pivot.add_child(_fire)
	NavalFire.set_intensity(_fire, fire, Vector2(cos(wind_to), sin(wind_to)) * (1.0 + wind_strength * 3.0))
