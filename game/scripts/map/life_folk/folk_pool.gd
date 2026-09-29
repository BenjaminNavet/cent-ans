class_name FolkPool
extends Node3D

## Chantier FK, lot FK3 (`docs/design/2026-09-29-carte-vivante-folk.md` § 2.2, § 5) : réservoir de
## figurines de la carte vivante. Rendu seulement.
##
## - Un `MultiMesh` par (rôle, activité) pour les figurines skinnées des batailles (mêmes
##   maillages et texture d'os que `ArmyFigures`, shader `battle_soldier_skinned` compilé avec
##   `FK_TRAVEL`) et un par accessoire (`folk_prop.gdshader`) ; modèles : `FolkModels`.
## - Plafond fixe de figurines (`pool_cap` de `data/rules/map_scenes.json`, repli 600), divisé par
##   deux tant que le budget d'image (`FrameBudget`) est dépassé ; accessoires : un quart du
##   plafond.
## - Actif au palier proche seulement (`ZoomTiers.near_weight`), dans un rayon autour du point
##   visé (`activity_radius`, borné par la distance caméra).
## - Placement recalculé seulement quand le point visé s'éloigne d'une fraction du rayon, que
##   l'échelle des figurines change nettement (zoom) ou qu'un tour passe (`refresh`), jamais à
##   chaque image. Animation et déplacement entièrement en shader : chaque instance parcourt en
##   boucle un segment droit (phase propre) ; hauteurs aux deux bouts par
##   `TerrainBuilder.surface_height_at`.
##
## Fournisseurs (`register`) : objets avec `refresh(sim)` (une fois par tour) et
## `populate(pool, focus, radius)` (au placement), appelés dans l'ordre d'enregistrement : les
## premiers sont servis d'abord quand le plafond est atteint (scènes FK4, marchands, routine).
## Ils posent leurs figurines avec `add` (en marche) et `add_static` (sur place).

const DATA_FILE := "rules/map_scenes.json"
const DEFAULT_CAP := 600
const DEFAULT_RADIUS := 60.0
## Hauteur d'une figurine (unités monde) à l'échelle de la carte, au-dessus de `shrink_start` de
## `MapPropScale` ; elle rejoint la taille réelle (1,8 m) au palier vallée.
const DEFAULT_FIGURE_HEIGHT := 0.5
const HUMAN_HEIGHT_M := 1.8
## Déplacement du point visé (fraction du rayon) qui déclenche un nouveau placement.
const MOVE_FRACTION := 0.2
## Variation relative d'échelle qui déclenche un nouveau placement.
const RESCALE_STEP := 0.12
## Poids du palier proche sous lequel le réservoir est vidé.
const NEAR_MIN := 0.35
## Budget : part des images en dépassement (sur `BUDGET_WINDOW`) qui divise le plafond par deux,
## et durée sans dépassement avant de le rétablir.
const BUDGET_WINDOW := 90
const BUDGET_OVER_SHARE := 0.6
const BUDGET_RESTORE_SECONDS := 20.0
const OVER_FRAME_SECONDS := 1.0 / 40.0

var cap: int = DEFAULT_CAP
## Plafond appliqué (moitié de `cap` en cas de dépassement du budget d'image).
var effective_cap: int = DEFAULT_CAP
var activity_radius: float = DEFAULT_RADIUS
var figure_height: float = DEFAULT_FIGURE_HEIGHT
## Réglages lus dans `map_scenes.json` (clés inconnues ignorées), partagés avec les fournisseurs.
var settings: Dictionary = {}
var stats: Dictionary = {"figures": 0, "props": 0, "placements": 0, "place_ms": 0.0, "halved": false}

var _map_data: MapData = null
var _terrain: TerrainBuilder = null
var _providers: Array = []
## clé → {mmi, material (figurines), kind, variant, role, activity, prop: bool, buffer, count}
var _groups: Dictionary = {}
var _anim_time := 0.0
var _last_focus := Vector2(INF, INF)
var _last_scale := 0.0
var _dirty := true
var _active := false
var _level := -1
## Échelle monde / modèle du placement en cours.
var _scale := 1.0
var _focus := Vector2.ZERO
var _radius := DEFAULT_RADIUS
var _figures := 0
var _props := 0
var _over_history := PackedByteArray()
var _quiet_seconds := 0.0
var _warned: Dictionary = {}


func setup(map_data: MapData, terrain: TerrainBuilder, cap_override: int = -1) -> void:
	_map_data = map_data
	_terrain = terrain
	settings = load_settings()
	cap = int(settings.get("pool_cap", DEFAULT_CAP))
	if cap_override >= 0:
		cap = cap_override
	effective_cap = cap
	activity_radius = float(settings.get("activity_radius", DEFAULT_RADIUS))
	figure_height = float(settings.get("figure_height", DEFAULT_FIGURE_HEIGHT))


## Réglages de `data/rules/map_scenes.json` (lot FK1) ; dictionnaire vide si absent. Les clés
## du réservoir peuvent être au premier niveau ou sous `folk`.
static func load_settings() -> Dictionary:
	var path := ArmyFigures._data_dir().path_join(DATA_FILE)
	if not FileAccess.file_exists(path):
		return {}
	var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
	if not (parsed is Dictionary):
		push_warning("FolkPool: %s invalid" % path)
		return {}
	var out: Dictionary = (parsed as Dictionary).duplicate()
	if parsed.get("folk") is Dictionary:
		out.merge(parsed["folk"], true)
	if parsed.get("densities") is Dictionary:
		out.merge(parsed["densities"], false)
	return out


## Avertissement unique (spec § 6 : donnée absente → routine sautée, un seul message).
func warn_once(key: String, message: String) -> void:
	if _warned.has(key):
		return
	_warned[key] = true
	push_warning(message)


func register(provider: Object) -> void:
	if provider != null and not _providers.has(provider):
		_providers.append(provider)


## Une fois par tour : relecture des fournisseurs, placement refait à la prochaine image.
func refresh(sim: Object) -> void:
	for provider in _providers:
		if provider.has_method("refresh"):
			provider.call("refresh", sim)
	_dirty = true


## Force un nouveau placement (option, test).
func invalidate() -> void:
	_dirty = true


## `focus` : point visé (carte) ; `near_weight` : poids du palier proche (`ZoomTiers`).
func update_view(focus: Vector2, camera_distance: float, near_weight: float) -> void:
	var delta := get_process_delta_time()
	_track_budget(delta)
	if near_weight < NEAR_MIN:
		if _active:
			clear()
		return
	_anim_time += delta
	var scale_now := world_scale(camera_distance)
	var radius := active_radius(camera_distance)
	var moved := focus.distance_to(_last_focus) > radius * MOVE_FRACTION
	var rescaled := _last_scale <= 0.0 or absf(scale_now / _last_scale - 1.0) > RESCALE_STEP
	if _dirty or moved or rescaled or not _active:
		place(focus, radius, scale_now)
	_apply_level(camera_distance)
	for key in _groups:
		var group: Dictionary = _groups[key]
		for material in group.get("materials", [group["material"]]):
			if material is ShaderMaterial:
				(material as ShaderMaterial).set_shader_parameter("anim_time", _anim_time)
	visible = true


## Échelle monde / modèle (1 unité de modèle = 1 m) à la distance `camera_distance`.
func world_scale(camera_distance: float) -> float:
	var props := MapPropScale.shared()
	var real := HUMAN_HEIGHT_M / (_map_data.meters_per_px if _map_data != null else 719.0)
	var shown := figure_height * props.exaggeration(camera_distance) / maxf(props.max_exaggeration, 1.0)
	return maxf(real, shown) / HUMAN_HEIGHT_M


## Rayon d'activité (unités monde) : `activity_radius`, borné par la distance caméra.
func active_radius(camera_distance: float) -> float:
	return minf(activity_radius, maxf(camera_distance * 1.3, 6.0))


## Vide le réservoir (palier moyen ou lointain, `--no-folk`).
func clear() -> void:
	for key in _groups:
		var group: Dictionary = _groups[key]
		(group["mmi"] as MultiMeshInstance3D).multimesh.instance_count = 0
		group["count"] = 0
		group["buffer"] = PackedFloat32Array()
	_figures = 0
	_props = 0
	_active = false
	visible = false
	stats["figures"] = 0
	stats["props"] = 0


## Placement : les fournisseurs posent leurs instances autour de `focus`.
func place(focus: Vector2, radius: float, world_scale_value: float) -> void:
	var t0 := Time.get_ticks_usec()
	_focus = focus
	_radius = radius
	_scale = world_scale_value
	_figures = 0
	_props = 0
	for key in _groups:
		_groups[key]["buffer"] = PackedFloat32Array()
		_groups[key]["count"] = 0
	for provider in _providers:
		if provider.has_method("populate"):
			provider.call("populate", self, focus, radius)
	var margin := radius + 10.0
	var aabb := AABB(Vector3(focus.x - margin, -200.0, focus.y - margin), Vector3(margin * 2.0, 4000.0, margin * 2.0))
	for key in _groups:
		var group: Dictionary = _groups[key]
		var mm := (group["mmi"] as MultiMeshInstance3D).multimesh
		mm.instance_count = int(group["count"])
		if int(group["count"]) > 0:
			mm.buffer = group["buffer"]
		mm.custom_aabb = aabb
	_last_focus = focus
	_last_scale = world_scale_value
	_dirty = false
	_active = true
	visible = true
	stats["figures"] = _figures
	stats["props"] = _props
	stats["placements"] = int(stats["placements"]) + 1
	stats["place_ms"] = float(Time.get_ticks_usec() - t0) / 1000.0


# --- API des fournisseurs ----------------------------------------------------------


## Données d'instance (longueur, vitesse, phase, dénivelé) posées au dernier placement (tests).
func instance_custom(key: String, index: int) -> Color:
	var group: Dictionary = _groups.get(key, {})
	var buffer: PackedFloat32Array = group.get("buffer", PackedFloat32Array())
	var o := index * 16 + 12
	if o + 3 >= buffer.size():
		return Color(0, 0, 0, 0)
	return Color(buffer[o], buffer[o + 1], buffer[o + 2], buffer[o + 3])


## Places de figurines restantes sous le plafond.
func remaining() -> int:
	return maxi(effective_cap - _figures, 0)


func prop_remaining() -> int:
	return maxi(effective_cap / 4 - _props, 0)


func figure_count() -> int:
	return _figures


func prop_count() -> int:
	return _props


## Vrai si `p` est dans le rayon d'activité du placement en cours.
func in_radius(p: Vector2) -> bool:
	return p.distance_squared_to(_focus) <= _radius * _radius


func focus() -> Vector2:
	return _focus


func radius() -> float:
	return _radius


## Échelle monde / modèle du placement en cours (1 m du modèle = `current_scale()` unités).
func current_scale() -> float:
	return _scale


## Instance en marche de `from` vers `to` (carte), en boucle : `phase` ∈ [0, 1) fraction du
## trajet, `lateral_m` décalage à droite (m), `behind_m` retard le long du trajet (m, groupes :
## charretier, marchand, gardes). `role` : rôle de figurine ou accessoire (`FolkModels`).
## Faux si le plafond est atteint ou le modèle absent.
func add(role: String, activity: String, from: Vector2, to: Vector2, phase: float = 0.0, lateral_m: float = 0.0, behind_m: float = 0.0) -> bool:
	var length := from.distance_to(to)
	if length < 1e-3:
		return add_static(role, activity, from, 0.0)
	var dir := (to - from) / length
	var right := Vector2(-dir.y, dir.x)
	var offset := right * (lateral_m * _scale)
	var a := from + offset
	var b := to + offset
	var ya := _height(a)
	var yb := _height(b)
	var length_m := length / _scale
	var speed := FolkModels.speed_of(role, activity)
	var custom := Color(length_m, speed, fposmod(phase, 1.0) * length_m - behind_m, (yb - ya) / _scale)
	return _push(role, activity, Vector3(a.x, ya, a.y), atan2(dir.x, dir.y), custom)


## Instance immobile (travaux des champs, bergers, bêtes) tournée de `yaw` (radians).
func add_static(role: String, activity: String, at: Vector2, yaw: float) -> bool:
	return _push(role, activity, Vector3(at.x, _height(at), at.y), yaw, Color(0, 0, 0, 0))


func _height(p: Vector2) -> float:
	if _terrain != null:
		return _terrain.surface_height_at(p.x, p.y)
	if _map_data != null:
		return _map_data.surface_world_at(p.x, p.y)
	return 0.0


func _push(role: String, activity: String, origin: Vector3, yaw: float, custom: Color) -> bool:
	var prop := FolkModels.is_prop(role)
	if prop and prop_remaining() <= 0:
		return false
	if not prop and remaining() <= 0:
		return false
	var group := _group(role, activity)
	if group.is_empty():
		return false
	var basis := Basis(Vector3.UP, yaw).scaled(Vector3.ONE * _scale)
	var buffer: PackedFloat32Array = group["buffer"]
	buffer.append_array([basis.x.x, basis.y.x, basis.z.x, origin.x, basis.x.y, basis.y.y, basis.z.y, origin.y, basis.x.z, basis.y.z, basis.z.z, origin.z, custom.r, custom.g, custom.b, custom.a])
	group["buffer"] = buffer
	group["count"] = int(group["count"]) + 1
	if prop:
		_props += 1
	else:
		_figures += 1
	return true


## Groupe (MultiMesh) d'un rôle et d'une activité, créé à la première demande ; {} si le
## modèle est absent.
func _group(role: String, activity: String) -> Dictionary:
	var prop := FolkModels.is_prop(role)
	var key := role if prop else "%s|%s" % [role, activity]
	if _groups.has(key):
		return _groups[key]
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.use_custom_data = true
	var material: ShaderMaterial = null
	var kind := ""
	var variant := 0
	if prop:
		mm.mesh = FolkModels.prop_mesh(role)
		if mm.mesh == null:
			return {}
		# Un matériau par surface (couleurs d'origine) : l'heure d'animation est posée sur
		# chacun ; `material` garde le premier pour la boucle d'`update_view`.
		material = mm.mesh.surface_get_material(0) as ShaderMaterial
	else:
		var figure := FolkModels.figure_of(role)
		if figure.is_empty():
			warn_once("figure:" + role, "FolkPool: no skinned figure for role %s" % role)
			return {}
		kind = figure[0]
		variant = figure[1]
		mm.mesh = BattleSkinned.mesh(kind, variant, maxi(_level, 1))
		if mm.mesh == null:
			return {}
		material = _figure_material(role, activity, kind, variant)
	var mmi := MultiMeshInstance3D.new()
	mmi.name = "Folk_" + key.replace("|", "_")
	mmi.multimesh = mm
	mmi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	if not prop:
		mmi.material_override = material
	add_child(mmi)
	var group := {"mmi": mmi, "material": material, "kind": kind, "variant": variant, "prop": prop, "buffer": PackedFloat32Array(), "count": 0}
	_groups[key] = group
	if prop:
		# Surfaces suivantes : même heure d'animation (voir `update_view`).
		group["materials"] = []
		for s in mm.mesh.get_surface_count():
			group["materials"].append(mm.mesh.surface_get_material(s))
	return group


func _figure_material(role: String, activity: String, kind: String, variant: int) -> ShaderMaterial:
	var material := ShaderMaterial.new()
	material.shader = BattleSkinned.SHADER
	var livery := FolkModels.livery_of(role)
	material.set_shader_parameter("livery", livery)
	material.set_shader_parameter("trim", livery.darkened(0.3))
	material.set_shader_parameter("has_heraldry", false)
	BattleSkinned.setup_material(material, kind, variant)
	# Déplacement en shader : variante `FK_TRAVEL` (conserve `FG3_BAKED` des figurines fines).
	var fine := material.shader != BattleSkinned.SHADER and material.shader.code.contains("#define FG3_BAKED")
	material.shader = BattleSkinned._variant(["FG3_BAKED", "FK_TRAVEL"] if fine else ["FK_TRAVEL"])
	material.set_shader_parameter("livery_share", 0.4)
	material.set_shader_parameter("plain_count", FolkModels.DRAB.size())
	material.set_shader_parameter("plain_colors", PackedColorArray(FolkModels.DRAB))
	material.set_shader_parameter("interp_distance", 40.0)
	material.set_shader_parameter("far_start", 80.0)
	material.set_shader_parameter("anim_time", _anim_time)
	BattleSkinned.apply_config(material, FolkModels.activity_config(kind, variant, activity), _anim_time)
	return material


## Niveau de détail des figurines selon la distance (toutes petites à l'écran au palier proche).
func _apply_level(camera_distance: float) -> void:
	var level := 2 if camera_distance > 35.0 else (1 if camera_distance > 10.0 else 0)
	if level == _level:
		return
	_level = level
	for key in _groups:
		var group: Dictionary = _groups[key]
		if bool(group["prop"]):
			continue
		(group["mmi"] as MultiMeshInstance3D).multimesh.mesh = BattleSkinned.mesh(str(group["kind"]), int(group["variant"]), level)


## Dépassement du budget d'image : plafond divisé par deux tant que la plupart des images
## récentes dépassent (`FrameBudget` épuisé et image lente), rétabli après un calme durable.
func _track_budget(delta: float) -> void:
	var over := delta > OVER_FRAME_SECONDS and FrameBudget.in_frame() and not FrameBudget.has_time()
	_over_history.append(1 if over else 0)
	if _over_history.size() > BUDGET_WINDOW:
		_over_history = _over_history.slice(_over_history.size() - BUDGET_WINDOW)
	var overs := 0
	for v in _over_history:
		overs += v
	_quiet_seconds = 0.0 if over else _quiet_seconds + delta
	if effective_cap == cap and _over_history.size() >= BUDGET_WINDOW and overs > int(BUDGET_WINDOW * BUDGET_OVER_SHARE):
		set_budget_exceeded(true)
	elif effective_cap < cap and _quiet_seconds > BUDGET_RESTORE_SECONDS:
		set_budget_exceeded(false)


## Plafond divisé par deux (`true`) ou rétabli ; placement refait à la prochaine image.
func set_budget_exceeded(exceeded: bool) -> void:
	effective_cap = cap / 2 if exceeded else cap
	stats["halved"] = exceeded
	_over_history = PackedByteArray()
	_dirty = true
