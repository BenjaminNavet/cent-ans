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

static var _map_lookup := JsonLookup.new(CAMPAIGN_MAP_DATA, {"army_figure_scale": 1.0}, "map")

static var _sail_meshes: Dictionary = {}  # "modèle|couleur" → Mesh aux voiles teintes


## Vrai si les figurines skinnées sont disponibles et que la comparaison `--legacy-army-markers`
## n'est pas demandée.
static func enabled() -> bool:
	if CmdArgs.has("--legacy-army-markers"):
		return false
	return BattleSkinned.has_figure("cavalry", 0) and BattleSkinned.has_figure("infantry", 0)


static func clear_cache() -> void:
	_sail_meshes.clear()
	_map_lookup.reload()


## Lot CV3-5 : réglages de rendu `map` de `data/ui/campaign_map.json` (repli : pas de lord).
static func map_settings() -> Dictionary:
	return _map_lookup.data()


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
		# AS2 : horloge propre au groupe (cadence de marche calée sur la vitesse réelle).
		_groups[key] = {"mm": instance, "kind": figure_kind, "variant": variant, "material": material, "clock": _anim_time, "factor": 1.0}


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
		BattleSkinned.apply_config(group["material"], _state_config(str(group["kind"]), int(group["variant"])), float(group["clock"]))
	for smoke in _smokes:
		smoke.emitting = not walking


# --- Lot AS2 : cadence de marche et balancement de la hampe ----------------------------

const WALK_DATA := "fx/campaign_army_walk.json"
static var _walk_lookup := JsonLookup.new(WALK_DATA, {"enabled": false})
## Vitesse au sol lissée (unités monde / s) et dernière position, mesurées sur le marqueur.
var _ground_speed: float = 0.0
var _last_position: Vector3 = Vector3.INF
## Balancement de l'étendard : amplitude lissée (0 à l'arrêt), phase de pas, pose courante.
var _sway_amp: float = 0.0
var _sway_phase: float = 0.0
var _bearer_tilt: Basis = Basis.IDENTITY
var _bearer_bob: float = 0.0


static func walk_settings() -> Dictionary:
	return _walk_lookup.data()


## Éteint par les données (`enabled`) ou par `--no-as2` après `--` (banc A/B).
static func as2_enabled() -> bool:
	return bool(walk_settings().get("enabled", false)) and not CmdArgs.has("--no-as2")


## Facteur de cadence pour une vitesse au sol `ground_speed` (unités monde / s) d'un groupe dont
## le clip de marche avance à `nominal_mps` (m/s à la vitesse 1) à l'échelle monde `world_scale`.
static func cadence_factor(ground_speed: float, nominal_mps: float, world_scale: float) -> float:
	var cfg: Dictionary = walk_settings().get("cadence", {})
	var nominal := maxf(nominal_mps * world_scale, 0.001)
	return clampf(ground_speed / nominal, float(cfg.get("min_factor", 0.35)), float(cfg.get("max_factor", 1.8)))


## Vitesse nominale (m/s) du clip de locomotion actif du groupe ; repli des données.
func _nominal_mps(group: Dictionary) -> float:
	var config := _state_config(str(group["kind"]), int(group["variant"]))
	var names: Array = config.get("names", [])
	var table: Dictionary = BattleGore.settings().get("cadence", {})
	var fallback := float(walk_settings().get("cadence", {}).get("default_nominal_mps", 1.35))
	if names.is_empty():
		return fallback
	return float(table.get(str(names[0]), fallback)) * float(config.get("speed", 1.0))


## Échelle monde des figurines d'un groupe (taille de figurine × échelle du marqueur).
func _group_world_scale(key: String) -> float:
	var marker := get_parent() as Node3D
	var marker_scale := marker.scale.x if marker != null else 1.0
	var size := lord_scale if (key == "cavalry_0" and is_lord()) else 1.0
	return FIGURE_SCALE * size * marker_scale


## Horloges des groupes : en marche, le temps avance de (vitesse écran / vitesse du clip) ;
## à l'arrêt, au rythme normal (le fondu idle/walk du shader fait la transition).
func _update_cadence(delta: float) -> void:
	var marker := get_parent() as Node3D
	var enabled := as2_enabled() and marker != null and delta > 0.0
	if enabled:
		var here := marker.global_position
		if _last_position != Vector3.INF:
			var flat := Vector2(here.x - _last_position.x, here.z - _last_position.z)
			var instant := flat.length() / delta
			var k := 1.0 - exp(-float(walk_settings().get("cadence", {}).get("smoothing_per_s", 5.0)) * delta)
			_ground_speed = lerpf(_ground_speed, instant if walking else 0.0, k)
		_last_position = here
	for key in _groups:
		var group: Dictionary = _groups[key]
		var target := 1.0
		if enabled and walking:
			target = cadence_factor(_ground_speed, _nominal_mps(group), _group_world_scale(str(key)))
		var factor := float(group["factor"])
		factor = lerpf(factor, target, 1.0 - exp(-8.0 * delta)) if enabled else 1.0
		group["factor"] = factor
		group["clock"] = float(group["clock"]) + delta * factor


## Facteur de cadence courant du groupe `key` (tests, réglage) ; 1 si absent.
func group_factor(key: String) -> float:
	return float(_groups[key]["factor"]) if _groups.has(key) else 1.0


## Balancement de l'étendard : à pied, rebond et roulis au rythme des pas (cadence incluse) ;
## en mer, inclinaison réelle du navire amiral (le calcul est dans `bearer_anchor`).
func _update_bearer_sway(delta: float) -> void:
	var cfg: Dictionary = walk_settings().get("standard_sway", {})
	var local := Basis.IDENTITY
	_bearer_bob = 0.0
	if not as2_enabled():
		_sway_amp = 0.0
	elif kind == "fleet":
		if not _ships.is_empty():
			local = Basis.from_euler(_ships[0].rotation * float(cfg.get("ship_tilt_gain", 1.0)))
	else:
		_sway_amp = move_toward(_sway_amp, 1.0 if walking else 0.0, float(cfg.get("fade_per_s", 6.0)) * delta)
		if _sway_amp > 0.0:
			var key := "cavalry_0" if is_lord() else "infantry_0"
			_sway_phase = fposmod(_sway_phase + delta * float(cfg.get("step_hz", 1.9)) * group_factor(key) * TAU, TAU * 2.0)
			# Un pas = un demi-tour de `_sway_phase * 0.5` : rebond à chaque pas, roulis d'un pied à l'autre.
			_bearer_bob = absf(sin(_sway_phase * 0.5)) * float(cfg.get("bob", 0.05)) * FIGURE_SCALE * _sway_amp
			var roll := sin(_sway_phase * 0.5) * deg_to_rad(float(cfg.get("roll_deg", 3.0))) * _sway_amp
			var lean := deg_to_rad(float(cfg.get("lean_deg", 2.0))) * _sway_amp
			# Repère de la figurine : le groupe regarde +X ; roulis autour de X, penché vers l'avant.
			local = Basis.from_euler(Vector3(roll, 0.0, -lean))
	var yaw := Basis(Vector3.UP, rotation.y)
	_bearer_tilt = yaw * local * yaw.inverse()


## L'étendard bouge-t-il à cette image (marche en cours ou fondu, navire) ? Sert à n'animer la
## hampe du marqueur que lorsqu'il le faut.
func bearer_dynamic() -> bool:
	return as2_enabled() and (kind == "fleet" or _sway_amp > 0.0 or walking)


## Inclinaison de la hampe dans le repère du marqueur (identité si AS2 est coupé).
func bearer_tilt() -> Basis:
	return _bearer_tilt


## Amplitude de rafale transmise au tissu (0 à l'arrêt) : le drapeau s'agite en marche.
func bearer_gust() -> float:
	return _sway_amp * float(walk_settings().get("standard_sway", {}).get("cloth_gust", 0.0))


## Pied de la hampe de l'étendard dans le repère du marqueur (suit le porte-étendard quand la
## troupe tourne) ; pour une flotte, pied de la hampe de poupe du navire amiral.
func bearer_anchor() -> Vector3:
	if kind == "fleet":
		# Hampe enfoncée dans le château de poupe (étendard au-dessus du mât, lisible).
		var stern := STERN_STAFF * SHIP_SCALE
		if as2_enabled() and not _ships.is_empty():
			# AS2 : le pied de la hampe suit le tangage, le roulis et le pilonnement du navire.
			var ship := _ships[0]
			stern = Basis.from_euler(ship.rotation) * stern + Vector3(0.0, ship.position.y, 0.0)
		return Basis(Vector3.UP, rotation.y) * stern - Vector3(0.0, 3.0, 0.0)
	if is_lord():
		# Lot CV3-5 : dans la main du général ; suit le fondu des figurines (au loin, la hampe
		# redescend au pied du marqueur).
		var slot := lord_slot()
		var hand := Vector3(slot.x + LORD_HAND.x * lord_scale, LORD_HAND.y * lord_scale, slot.y + LORD_HAND.z * lord_scale)
		return Basis(Vector3.UP, rotation.y) * (hand * clampf(_weight, 0.0, 1.0)) + Vector3(0.0, _bearer_bob, 0.0)
	var local := Vector3(BEARER_SLOT.x + POLE_IN_HAND.x, 0.0, BEARER_SLOT.y + POLE_IN_HAND.y)
	return Basis(Vector3.UP, rotation.y) * local + Vector3(0.0, _bearer_bob, 0.0)


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
	_update_cadence(delta)
	_update_bearer_sway(delta)
	for key in _groups:
		var group: Dictionary = _groups[key]
		(group["material"] as ShaderMaterial).set_shader_parameter("anim_time", float(group["clock"]))
	# Léger tangage et roulis, déphasés d'un navire à l'autre.
	for i in _ships.size():
		var t := _anim_time + i * 1.7
		var ship := _ships[i]
		ship.rotation = Vector3(sin(t * 1.15) * 0.045, 0.0, sin(t * 0.8 + 0.6) * 0.03)
		ship.position.y = sin(t * 0.95) * 0.12 - 0.15
