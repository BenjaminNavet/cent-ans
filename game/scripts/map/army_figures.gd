class_name ArmyFigures
extends Node3D

## Armée figurée sur la carte de campagne (lot CV2) : général à cheval,
## porte-étendard à pied et quelques soldats selon l'effectif et la composition, en figurines
## skinnées du lot V2 (maillages `battle_skinned`, animations cuites en texture d'os, un
## `MultiMesh` par figurine) aux couleurs et armes de la faction. Animées : marche pendant le
## déplacement (`set_walking`, piloté par l'animation M4), repos à l'arrêt ; camp (tentes et
## fumée) en siège ou au bivouac en rase campagne. Flotte : cogues ou nefs (voile aux couleurs
## de la faction, `campaign_sail.gdshader`) qui tanguent. Au loin (palier « loin » de
## `ZoomTiers`), les figurines se fondent dans l'étendard et la plaque d'effectif.
## Rendu seulement : lit le dictionnaire `get_army` du pont, aucune règle.
##
## Lot CV3-5 : « lord » à l'échelle de la grande stratégie. Le général à cheval est agrandi
## (`map.army_figure_scale`, `data/ui/campaign_map.json`) et porte lui-même l'étendard de l'ost
## (plus de porte-étendard à pied) ; l'escorte reste derrière lui. Au palier « loin », il se fond
## comme le reste dans l'étendard et la plaque (retour au marqueur).

## Hauteur d'homme ≈ 1,8 m (maillages V2) → ≈ 4,7 unités du repère du marqueur (hampe 8,4).
const FIGURE_SCALE := 2.3
## Échelle des navires (unités Blender → repère du marqueur) : mât ≈ 1,75 → ≈ 9,6.
const SHIP_SCALE := 5.5
## Soldats d'escorte (en plus du général et du porte-étendard) : [effectif max, nombre].
const ESCORT_BY_MEN := [[300, 2], [800, 3], [1500, 4], [3000, 5], [1000000000, 6]]
## Navires d'une flotte : un par tranche de `MEN_PER_SHIP` hommes, au plus `MAX_SHIPS`.
const MEN_PER_SHIP := 900
const MAX_SHIPS := 3
## Places (repère du groupe : la troupe regarde +X, Z à droite).
const LEADER_SLOT := Vector2(1.5, 0.2)
const BEARER_SLOT := Vector2(0.1, -1.4)
## Pied de la hampe dans le repère du porte-étendard (main droite, un peu en avant).
const POLE_IN_HAND := Vector2(0.35, 0.55)
## Lot CV3-5 : main droite du cavalier (repère du général à l'échelle 1 : en avant, à droite,
## hauteur de la main au-dessus du sol) ; recul du général par unité d'agrandissement (sa monture
## grandit vers l'avant, pas sur l'escorte).
const LORD_HAND := Vector3(0.4, 3.8, 0.7)
const LORD_ADVANCE := 2.6
const CAMPAIGN_MAP_DATA := "ui/campaign_map.json"
const MAP_PATHS_SCRIPT := preload("res://scripts/map/map_paths.gd")
const ESCORT_ROWS_X := [-1.4, -3.0]
const ESCORT_FILE_Z := [-1.3, 0.0, 1.3]
## Pied de la hampe de poupe (`campaign_fleet.py`, STERN_STAFF), repère du navire.
const STERN_STAFF := Vector3(-0.9, 0.95, 0.0)
const SHIP_SLOTS := [Vector2(0.0, 0.0), Vector2(-7.5, 5.0), Vector2(-8.0, -5.5)]
const CAMP_SLOT := Vector3(-4.5, 0.0, 4.2)
const SIEGE_CAMP_SLOT := Vector3(4.5, 0.0, 4.0)
## Niveaux de détail des figurines selon la distance du rig de caméra.
const LOD0_DISTANCE := 90.0
const LOD1_DISTANCE := 320.0
## Au-delà, pas de fumée (invisible et coûteuse).
const SMOKE_DISTANCE := 420.0
## Le bivouac n'apparaît qu'en vue rapprochée (au palier moyen il encombrerait la carte).
const BIVOUAC_DISTANCE := 190.0
const SMOKE_SHADER := preload("res://shaders/fire_smoke.gdshader")
const SMOKE_FLIPBOOK := "res://assets/textures/fx/smoke_flipbook.png"
const SAIL_SHADER := preload("res://shaders/campaign_sail.gdshader")
const TRIM_GOLD := Color(0.83, 0.66, 0.24)
const TRIM_SILVER := Color(0.78, 0.8, 0.82)

## "army" ou "fleet".
var kind: String = "army"
var walking: bool = false
var camped: bool = false
var men: int = 0
## Figurines : clé "kind_variant" → {mm: MultiMeshInstance3D, kind, variant, material}.
var _groups: Dictionary = {}
var _ships: Array[Node3D] = []
var _smokes: Array[GPUParticles3D] = []
var _anim_time: float = 0.0
var _level: int = -1
var _weight: float = 1.0
var _color: Color = Color.WHITE
var _heraldry: Texture2D
## Lot CV3-5 : agrandissement du général porte-étendard (1 = pas de lord : porte-étendard à pied).
var lord_scale: float = 1.0

static var _map_settings: Dictionary = {}

static var _sail_meshes: Dictionary = {}  # "modèle|couleur" → Mesh aux voiles teintes


## Vrai si les figurines skinnées sont disponibles et que la comparaison `--legacy-army-markers`
## n'est pas demandée.
static func enabled() -> bool:
	if OS.get_cmdline_user_args().has("--legacy-army-markers"):
		return false
	return BattleSkinned.has_figure("cavalry", 0) and BattleSkinned.has_figure("infantry", 0)


static func clear_cache() -> void:
	_sail_meshes.clear()
	_map_settings.clear()


## Lot CV3-5 : réglages de rendu `map` de `data/ui/campaign_map.json` (repli : pas de lord).
static func map_settings() -> Dictionary:
	if _map_settings.is_empty():
		var fallback := {"army_figure_scale": 1.0}
		_map_settings = fallback.duplicate()
		var path := _data_dir().path_join(CAMPAIGN_MAP_DATA)
		if not FileAccess.file_exists(path):
			path = MAP_PATHS_SCRIPT.project_root().path_join("data").path_join(CAMPAIGN_MAP_DATA)
		var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path)) if FileAccess.file_exists(path) else null
		if parsed is Dictionary and parsed.get("map") is Dictionary:
			_map_settings.merge(parsed["map"], true)
		else:
			push_warning("ArmyFigures: %s missing or invalid" % path)
	return _map_settings


static func _data_dir() -> String:
	var tree := Engine.get_main_loop() as SceneTree
	if tree != null and tree.root != null:
		var map_paths := tree.root.get_node_or_null("MapPaths")
		if map_paths != null:
			return str(map_paths.get("data_dir"))
	return MAP_PATHS_SCRIPT.project_root().path_join("data")


## Construit la représentation d'une armée (`army` = dictionnaire du pont).
static func build(army: Dictionary, color: Color, heraldry: Texture2D, seed_text: String) -> ArmyFigures:
	var figures := ArmyFigures.new()
	figures.name = ModelLibrary.MODEL_NODE
	figures._color = color
	figures._heraldry = heraldry
	figures.lord_scale = maxf(float(map_settings().get("army_figure_scale", 1.0)), 1.0)  # CV3-5
	figures._anim_time = float(absi(hash(seed_text)) % 1000) * 0.013
	for unit in army.get("units", []):
		figures.men += int(unit.get("strength", 0))
	if bool(army.get("embarked", false)) or bool(army.get("at_sea", false)):
		figures.kind = "fleet"
		figures._build_fleet()
	else:
		figures._build_troop(army)
		var stance := str(army.get("stance", ""))
		var in_field := army.has("position") and str(army.get("settlement", "")) == ""
		var moving := ArmyMarker.army_status(army) == "moving"
		# GC (ADR 0158) : camps réglés par rapport aux maquettes des lieux (camp de siège ≈ un bourg,
		# bivouac ≈ un village à hauteur de jeu), `map.siege_camp_scale` / `map.bivouac_scale`.
		if stance == "siege":
			figures._build_camp("siege_camp", SIEGE_CAMP_SLOT, float(map_settings().get("siege_camp_scale", 1.3)))
		elif in_field and not moving:
			figures._build_camp("fleet/bivouac", CAMP_SLOT, float(map_settings().get("bivouac_scale", 0.75)))
	return figures


# --- Troupe --------------------------------------------------------------------------


## Figurines d'escorte ["kind", variant] proportionnelles à la composition (plus fort reste).
static func escort_roster(army: Dictionary) -> Array:
	var weights: Dictionary = {}
	var men_total := 0
	for unit in army.get("units", []):
		var strength := int(unit.get("strength", 0))
		men_total += strength
		var unit_type := str(unit.get("unit_type", ""))
		var figure_kind: String = {"ranged": "archer", "cavalry": "cavalry", "infantry": "infantry"}.get(ModelLibrary.unit_category(unit_type), "")
		# Lot UR1 : la figurine déclarée prime (tireurs montés : famille cavalry).
		if figure_kind != "" and BattleMeshes.figure_kind_of(unit_type) != "":
			figure_kind = BattleMeshes.figure_kind_of(unit_type)
		if figure_kind == "":
			continue  # engins de siège
		var key := "%s_%d" % [figure_kind, BattleMeshes.variant_of(unit_type)]
		weights[key] = int(weights.get(key, 0)) + maxi(strength, 1)
	var count := 0
	for step in ESCORT_BY_MEN:
		if men_total <= int(step[0]):
			count = int(step[1])
			break
	var roster: Array = []
	if weights.is_empty():
		for i in count:
			roster.append(["infantry", 0])
		return roster
	var total := 0
	for key in weights:
		total += int(weights[key])
	var keys: Array = weights.keys()
	keys.sort_custom(func(a: String, b: String) -> bool: return int(weights[a]) > int(weights[b]) or (int(weights[a]) == int(weights[b]) and a < b))
	var shares: Dictionary = {}
	var remaining := count
	for key in keys:
		shares[key] = int(floor(float(weights[key]) * count / total))
		remaining -= int(shares[key])
	var by_remainder := keys.duplicate()
	by_remainder.sort_custom(func(a: String, b: String) -> bool:
		var ra := float(weights[a]) * count / total - int(shares[a])
		var rb := float(weights[b]) * count / total - int(shares[b])
		return ra > rb or (is_equal_approx(ra, rb) and a < b))
	for key in by_remainder:
		if remaining <= 0:
			break
		shares[key] = int(shares[key]) + 1
		remaining -= 1
	# Cavaliers devant, fantassins, puis tireurs en queue de colonne.
	var order := {"cavalry": 0, "infantry": 1, "archer": 2}
	keys.sort_custom(func(a: String, b: String) -> bool:
		var oa: int = order.get(a.get_slice("_", 0), 3)
		var ob: int = order.get(b.get_slice("_", 0), 3)
		return oa < ob or (oa == ob and a < b))
	for key in keys:
		for i in int(shares[key]):
			roster.append([key.get_slice("_", 0), int(key.get_slice("_", 1))])
	return roster


func _build_troop(army: Dictionary) -> void:
	var slots: Dictionary = {}  # "kind_variant" → Array[Transform3D]
	if is_lord():
		# Lot CV3-5 : le général agrandi porte l'étendard ; pas de porte-étendard à pied.
		_add_slot(slots, "cavalry", 0, lord_slot(), 0.0, lord_scale)
	else:
		_add_slot(slots, "cavalry", 0, LEADER_SLOT, 0.0)
		_add_slot(slots, "infantry", 0, BEARER_SLOT, 0.0)
	var roster := escort_roster(army)
	for i in roster.size():
		var row := i / ESCORT_FILE_Z.size()
		var file := i % ESCORT_FILE_Z.size()
		if row >= ESCORT_ROWS_X.size():
			break
		var jitter := float((i * 37 + 11) % 17) / 17.0 - 0.5
		var slot := Vector2(float(ESCORT_ROWS_X[row]) + jitter * 0.4, float(ESCORT_FILE_Z[file]) + jitter * 0.3)
		# Les cavaliers d'escorte sont plus longs : un pas de plus en arrière.
		if str(roster[i][0]) == "cavalry":
			slot.x -= 0.6
		_add_slot(slots, str(roster[i][0]), int(roster[i][1]), slot, jitter * 0.15)
	var troop := Node3D.new()
	troop.name = "Troop"
	add_child(troop)
	for key in slots:
		var figure_kind := str(key).get_slice("_", 0)
		var variant := int(str(key).get_slice("_", 1))
		if not BattleSkinned.has_figure(figure_kind, variant):
			variant = 0
		var transforms: Array = slots[key]
		var mm := MultiMesh.new()
		mm.transform_format = MultiMesh.TRANSFORM_3D
		mm.mesh = BattleSkinned.mesh(figure_kind, variant, 1)
		mm.instance_count = transforms.size()
		for i in transforms.size():
			mm.set_instance_transform(i, transforms[i])
		var instance := MultiMeshInstance3D.new()
		instance.name = "Fig_" + str(key)
		instance.multimesh = mm
		instance.layers = 2
		var material := _make_material(figure_kind, variant)
		instance.material_override = material
		troop.add_child(instance)
		_groups[key] = {"mm": instance, "kind": figure_kind, "variant": variant, "material": material}


## Lot CV3-5 : vrai quand le général agrandi porte l'étendard (réglage > 1).
func is_lord() -> bool:
	return lord_scale > 1.001


## Lot CV3-5 : place du général agrandi (avancé pour que sa monture ne couvre pas l'escorte).
func lord_slot() -> Vector2:
	return LEADER_SLOT + Vector2(LORD_ADVANCE * (lord_scale - 1.0), 0.0)



static func _add_slot(slots: Dictionary, figure_kind: String, variant: int, slot: Vector2, yaw: float, size: float = 1.0) -> void:
	var key := "%s_%d" % [figure_kind, variant]
	if not slots.has(key):
		slots[key] = []
	# Figurines V2 : regard +Z ; le groupe regarde +X (rotation d'un quart de tour).
	var basis := Basis(Vector3.UP, PI * 0.5 + yaw).scaled(Vector3.ONE * FIGURE_SCALE * size)
	(slots[key] as Array).append(Transform3D(basis, Vector3(slot.x, 0.0, slot.y)))


func _make_material(figure_kind: String, variant: int) -> ShaderMaterial:
	var material := ShaderMaterial.new()
	material.shader = BattleSkinned.SHADER
	material.set_shader_parameter("livery", _color)
	var silver := _color.get_luminance() > 0.55 or (_color.r > 0.6 and _color.g > 0.5)
	material.set_shader_parameter("trim", TRIM_SILVER if silver else TRIM_GOLD)
	material.set_shader_parameter("heraldry", _heraldry)
	material.set_shader_parameter("has_heraldry", _heraldry != null)
	BattleSkinned.setup_material(material, figure_kind, variant)
	# Le chef et sa maison portent la livrée ; l'escorte à moitié.
	material.set_shader_parameter("livery_share", 0.95 if variant == 0 else 0.7)
	# Carte : la caméra est toujours à plus de 45 unités, images sans interpolation au-delà
	# de 150 (la marche reste fluide en vue rapprochée).
	material.set_shader_parameter("interp_distance", 150.0)
	material.set_shader_parameter("far_start", 260.0)
	material.set_shader_parameter("anim_time", _anim_time)
	BattleSkinned.apply_config(material, _state_config(figure_kind, variant), _anim_time)
	return material


func _state_config(figure_kind: String, variant: int) -> Dictionary:
	return BattleSkinned.state_config(figure_kind, variant, "marching" if walking else "idle", false)


## SA (ADR 0160) : éclaircissement des figurines (survol, sélection), 0 = aucun.
func set_highlight(value: float) -> void:
	for key in _groups:
		(_groups[key]["material"] as ShaderMaterial).set_shader_parameter("highlight", value)


## Marche (animation M4 en cours) ou repos.
func set_walking(value: bool) -> void:
	if value == walking:
		return
	walking = value
	for key in _groups:
		var group: Dictionary = _groups[key]
		BattleSkinned.apply_config(group["material"], _state_config(str(group["kind"]), int(group["variant"])), _anim_time)
	for smoke in _smokes:
		smoke.emitting = not walking


## Pied de la hampe de l'étendard dans le repère du marqueur (suit le porte-étendard quand la
## troupe tourne) ; pour une flotte, pied de la hampe de poupe du navire amiral.
func bearer_anchor() -> Vector3:
	if kind == "fleet":
		# Hampe enfoncée dans le château de poupe (étendard au-dessus du mât, lisible).
		return Basis(Vector3.UP, rotation.y) * (STERN_STAFF * SHIP_SCALE) - Vector3(0.0, 3.0, 0.0)
	if is_lord():
		# Lot CV3-5 : dans la main du général ; suit le fondu des figurines (au loin, la hampe
		# redescend au pied du marqueur).
		var slot := lord_slot()
		var hand := Vector3(slot.x + LORD_HAND.x * lord_scale, LORD_HAND.y * lord_scale, slot.y + LORD_HAND.z * lord_scale)
		return Basis(Vector3.UP, rotation.y) * (hand * clampf(_weight, 0.0, 1.0))
	var local := Vector3(BEARER_SLOT.x + POLE_IN_HAND.x, 0.0, BEARER_SLOT.y + POLE_IN_HAND.y)
	return Basis(Vector3.UP, rotation.y) * local


# --- Flotte --------------------------------------------------------------------------


func _build_fleet() -> void:
	var count := clampi(ceili(float(men) / MEN_PER_SHIP), 1, MAX_SHIPS)
	for i in count:
		var model_name := "fleet/cog" if i % 2 == 0 else "fleet/nef"
		var ship := ModelLibrary.instantiate(model_name, SHIP_SCALE * (1.0 if i == 0 else 0.85))
		if ship == null:
			ship = ModelLibrary.instantiate("ship", ModelLibrary.ARMY_SCALE * 1.5)
			if ship == null:
				continue
			ModelLibrary.tint_banner(ship, _color)
		else:
			_dress_sail(ship, model_name)
		ship.name = "Ship_%d" % i
		var slot: Vector2 = SHIP_SLOTS[i]
		ship.position = Vector3(slot.x, 0.0, slot.y)
		for geometry in ship.find_children("*", "GeometryInstance3D", true, false):
			(geometry as GeometryInstance3D).layers = 2
		add_child(ship)
		_ships.append(ship)


## Voile aux couleurs de la faction (matériau `Sail` remplacé, maillage dupliqué par couleur ;
## `Banner` teint comme les autres modèles).
func _dress_sail(ship: Node3D, model_name: String) -> void:
	ModelLibrary.tint_banner(ship, _color)
	for child in ship.find_children("*", "MeshInstance3D", true, false):
		var mesh_instance := child as MeshInstance3D
		if mesh_instance.mesh == null:
			continue
		var key := "%s|%s|%s" % [model_name, _color.to_html(), str(_heraldry.get_rid().get_id()) if _heraldry != null else "-"]
		if _sail_meshes.has(key):
			mesh_instance.mesh = _sail_meshes[key]
			continue
		var dressed: Mesh = null
		for surface in mesh_instance.mesh.get_surface_count():
			var material := mesh_instance.mesh.surface_get_material(surface)
			if material == null or material.resource_name != "Sail":
				continue
			if dressed == null:
				dressed = mesh_instance.mesh.duplicate() as Mesh
			var sail := ShaderMaterial.new()
			sail.shader = SAIL_SHADER
			sail.set_shader_parameter("faction_color", _color)
			sail.set_shader_parameter("heraldry", _heraldry)
			sail.set_shader_parameter("has_heraldry", _heraldry != null)
			dressed.surface_set_material(surface, sail)
		if dressed != null:
			_sail_meshes[key] = dressed
			mesh_instance.mesh = dressed


# --- Camp ----------------------------------------------------------------------------


func _build_camp(model_name: String, slot: Vector3, model_scale: float) -> void:
	var camp := ModelLibrary.instantiate(model_name, ModelLibrary.ARMY_SCALE * model_scale)
	if camp == null:
		return
	camp.name = "SiegeCamp" if model_name == "siege_camp" else "Bivouac"
	camp.position = slot
	ModelLibrary.tint_banner(camp, _color)
	for geometry in camp.find_children("*", "GeometryInstance3D", true, false):
		(geometry as GeometryInstance3D).layers = 2
	add_child(camp)
	camped = true
	# Fumée du feu de camp (foyer du modèle : siège (-0,1 ; 0,05), bivouac à l'origine).
	var hearth := Vector3(-0.1, 0.0, -0.05) * ModelLibrary.ARMY_SCALE * model_scale if model_name == "siege_camp" else Vector3.ZERO
	var smoke := _make_smoke()
	if smoke != null:
		smoke.position = slot + hearth + Vector3(0.0, 1.0, 0.0)
		add_child(smoke)
		_smokes.append(smoke)


func _make_smoke() -> GPUParticles3D:
	if not ResourceLoader.exists(SMOKE_FLIPBOOK):
		return null
	var material := ShaderMaterial.new()
	material.shader = SMOKE_SHADER
	material.set_shader_parameter("flipbook", load(SMOKE_FLIPBOOK))
	material.set_shader_parameter("smoke_color", Color(0.62, 0.6, 0.57))
	material.set_shader_parameter("ember_glow_energy", 0.0)
	material.set_shader_parameter("soft_distance", 1.5)
	var particles := GPUParticles3D.new()
	particles.name = "CampSmoke"
	particles.amount = 10
	particles.lifetime = 7.0
	particles.preprocess = 7.0
	particles.local_coords = true
	particles.visibility_aabb = AABB(Vector3(-15, -2, -15), Vector3(30, 40, 30))
	particles.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	particles.layers = 2
	var process := ParticleProcessMaterial.new()
	process.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_SPHERE
	process.emission_sphere_radius = 0.4
	process.direction = Vector3.UP
	process.spread = 8.0
	process.initial_velocity_min = 1.2
	process.initial_velocity_max = 1.8
	process.gravity = Vector3(0.35, 0.25, 0.1)
	process.damping_min = 0.1
	process.damping_max = 0.2
	process.scale_min = 1.6
	process.scale_max = 2.6
	process.angle_min = -180.0
	process.angle_max = 180.0
	process.angular_velocity_min = -10.0
	process.angular_velocity_max = 10.0
	var growth := Curve.new()
	growth.add_point(Vector2(0.0, 0.4))
	growth.add_point(Vector2(0.4, 0.9))
	growth.add_point(Vector2(1.0, 1.8))
	var growth_texture := CurveTexture.new()
	growth_texture.curve = growth
	process.scale_curve = growth_texture
	var ramp := Gradient.new()
	ramp.set_color(0, Color(1, 1, 1, 0.55))
	ramp.set_color(1, Color(1, 1, 1, 0.0))
	var ramp_texture := GradientTexture1D.new()
	ramp_texture.gradient = ramp
	process.color_ramp = ramp_texture
	particles.process_material = process
	var quad := QuadMesh.new()
	quad.size = Vector2.ONE
	quad.material = material
	particles.draw_pass_1 = quad
	return particles


# --- Vue -----------------------------------------------------------------------------


## `camera_distance` = distance du rig ; `weight` = présence des figurines (1 près, 0 au
## palier « loin » : elles se fondent dans l'étendard).
func set_view(camera_distance: float, weight: float) -> void:
	_weight = weight
	visible = weight > 0.02
	scale = Vector3.ONE * maxf(weight, 0.02)
	var level := 0 if camera_distance < LOD0_DISTANCE else (1 if camera_distance < LOD1_DISTANCE else 2)
	if level != _level:
		_level = level
		for key in _groups:
			var group: Dictionary = _groups[key]
			(group["mm"] as MultiMeshInstance3D).multimesh.mesh = BattleSkinned.mesh(str(group["kind"]), int(group["variant"]), level)
	var bivouac := get_node_or_null("Bivouac") as Node3D
	if bivouac != null:
		bivouac.visible = camera_distance < BIVOUAC_DISTANCE
	var smoky := visible and camera_distance < (BIVOUAC_DISTANCE if bivouac != null else SMOKE_DISTANCE)
	for smoke in _smokes:
		smoke.visible = smoky


func figure_count() -> int:
	var count := 0
	for key in _groups:
		count += (_groups[key]["mm"] as MultiMeshInstance3D).multimesh.instance_count
	return count


func ship_count() -> int:
	return _ships.size()


func _process(delta: float) -> void:
	if not is_visible_in_tree():
		return
	_anim_time += delta
	for key in _groups:
		(_groups[key]["material"] as ShaderMaterial).set_shader_parameter("anim_time", _anim_time)
	# Léger tangage et roulis, déphasés d'un navire à l'autre.
	for i in _ships.size():
		var t := _anim_time + i * 1.7
		var ship := _ships[i]
		ship.rotation = Vector3(sin(t * 1.15) * 0.045, 0.0, sin(t * 0.8 + 0.6) * 0.03)
		ship.position.y = sin(t * 0.95) * 0.12 - 0.15
