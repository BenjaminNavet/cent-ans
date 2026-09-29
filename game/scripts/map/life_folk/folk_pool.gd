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
const DEFAULT_FIGURE_HEIGHT := 1.26
const HUMAN_HEIGHT_M := 1.8
const DEFAULT_MIN_VIEW_FRACTION := 0.045
## Couples rôle:activité préchauffés (vie ordinaire, marchands) ; les autres sont créés à la
## première demande.
const WARM_FIGURES := ["peasant:walk", "peasant_b:walk", "porter:walk", "rider:ride", "pilgrim:walk", "merchant:walk", "guard:guard_walk", "peasant:plough", "peasant_b:plough", "porter:harvest", "reaper:scythe", "peasant:idle", "peasant_b:herd", "peasant:chop"]
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
## Délai maximal d'un placement différé pendant un mouvement continu de la caméra.
const MAX_DEFER_SECONDS := 0.5

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
## Vrai après le premier `refresh` : pas de placement avant (état du tour encore inconnu).
var _refreshed := false
var _height_usec := 0
var _create_usec := 0
## Groupes dont le modèle manque (clé → vrai).
var _missing: Dictionary = {}
## FK6 : vrai quand le réservoir est à jour pour la vue courante (préchauffage fini, placement
## fait ou rien à placer) ; les captures `--screenshot` l'attendent.
var _settled := false
## FK6 : préchauffage (accessoires, première matière de figurine) étalé sur les premières images,
## avant tout placement : la première lecture des sommets d'un glb (`surface_get_arrays`) attend
## le fil de rendu (50-170 ms mesurés pour la première, quelques ms ensuite) ; faite pendant un
## placement, elle coûtait 100-480 ms d'un coup.
var _warm_queue: Array = []
var _warm_usec := 0
## Budget de préchauffage par image (µs) ; au moins un élément par image.
const WARM_BUDGET_USEC := 4000
## FK6 : cercles (x, z, rayon) des villes emblématiques (maquette L1 et ville 1:1) : aucune
## figurine n'y est posée (les scènes se tiennent à leur bord).
var exclusions: PackedVector3Array = PackedVector3Array()
## Couche des colonies dont on tire `exclusions` au premier `refresh` (nulle en test).
var landmark_layer: Node = null
## FK6 : hauteur d'une figurine au moins égale à cette fraction de la distance caméra (taille à
## l'écran à peu près constante au palier proche, lisible), au-dessus de l'échelle des
## accessoires (`MapPropScale`) qui la ramène sinon à la taille réelle (1 px) près du sol.
var figure_min_view_fraction: float = DEFAULT_MIN_VIEW_FRACTION
var _prev_focus := Vector2(INF, INF)
var _prev_distance := 0.0
var _since_place := 0.0


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
	figure_min_view_fraction = float(settings.get("figure_min_view_fraction", DEFAULT_MIN_VIEW_FRACTION))
	# Groupes de figurines de la routine et des marchands (≈ 0,2-1 ms chacun, 9 ms le premier).
	_warm_queue = []
	for pair in WARM_FIGURES:
		_warm_queue.append("figure:" + pair)
	for role in FolkModels.PROPS:
		_warm_queue.append("prop:" + str(role))


## Réglages de `data/rules/map_scenes.json` (source unique, schéma `map_scenes_rules`, miroir
## `MapSceneRules` du cœur) ; lus dans le fichier car le réservoir existe avant la campagne.
## Dictionnaire vide si absent (les fournisseurs gardent alors leurs valeurs de repli).
static func load_settings() -> Dictionary:
	var path := ArmyFigures._data_dir().path_join(DATA_FILE)
	if not FileAccess.file_exists(path):
		return {}
	var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
	if not (parsed is Dictionary):
		push_warning("FolkPool: %s invalid" % path)
		return {}
	return (parsed as Dictionary).duplicate()


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
	if exclusions.is_empty() and landmark_layer != null:
		exclusions = landmark_disks(landmark_layer)
	for provider in _providers:
		if provider.has_method("refresh"):
			provider.call("refresh", sim)
	_refreshed = true
	_dirty = true


## Force un nouveau placement (option, test).
func invalidate() -> void:
	_dirty = true


## `focus` : point visé (carte) ; `near_weight` : poids du palier proche (`ZoomTiers`).
func update_view(focus: Vector2, camera_distance: float, near_weight: float) -> void:
	var delta := get_process_delta_time()
	_track_budget(delta)
	_settled = false
	if not _warm_queue.is_empty():
		_warm_step(WARM_BUDGET_USEC)
		if not _warm_queue.is_empty():
			return
	if near_weight < NEAR_MIN or not _refreshed:
		if _active:
			clear()
		_settled = _refreshed or near_weight < NEAR_MIN
		return
	_anim_time += delta
	var scale_now := world_scale(camera_distance)
	var radius := active_radius(camera_distance)
	var moved := focus.distance_to(_last_focus) > radius * MOVE_FRACTION
	var rescaled := _last_scale <= 0.0 or absf(scale_now / _last_scale - 1.0) > RESCALE_STEP
	# Pendant un déplacement ou un zoom continu, placement différé jusqu'à ce que la caméra se
	# pose (ou au plus tard après `MAX_DEFER_SECONDS`) : pas un placement par image.
	var steady := focus.distance_to(_prev_focus) < radius * 0.004 and absf(camera_distance / maxf(_prev_distance, 1e-3) - 1.0) < 0.004
	_prev_focus = focus
	_prev_distance = camera_distance
	_since_place += delta
	var pending := not _active or _dirty or moved or rescaled
	if not _active or (pending and (steady or _since_place > MAX_DEFER_SECONDS)):
		place(focus, radius, scale_now)
		pending = false
	_settled = not pending
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
	# Borné à `shrink_start` : au-delà, la taille de carte (`figure_height`) prend le relais
	# (0,018 × 28 ≈ 0,5, raccord continu).
	var readable := figure_min_view_fraction * minf(camera_distance, props.shrink_start)
	return maxf(maxf(real, shown), readable) / HUMAN_HEIGHT_M


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
	_height_usec = 0
	_create_usec = 0
	var provider_ms := {}
	for provider in _providers:
		if provider.has_method("populate"):
			var tp := Time.get_ticks_usec()
			provider.call("populate", self, focus, radius)
			provider_ms[str(provider.get_script().get_global_name())] = float(Time.get_ticks_usec() - tp) / 1000.0
	stats["provider_ms"] = provider_ms
	stats["height_ms"] = float(_height_usec) / 1000.0
	stats["create_ms"] = float(_create_usec) / 1000.0
	stats["warm_ms"] = float(_warm_usec) / 1000.0
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
	_since_place = 0.0
	_dirty = false
	_active = true
	visible = true
	stats["figures"] = _figures
	stats["props"] = _props
	stats["placements"] = int(stats["placements"]) + 1
	stats["place_ms"] = float(Time.get_ticks_usec() - t0) / 1000.0
	if OS.get_environment("FK_DEBUG") != "":
		_debug_dump()


# --- API des fournisseurs ----------------------------------------------------------


## Données d'instance (longueur, vitesse, phase, dénivelé) posées au dernier placement (tests).
func instance_custom(key: String, index: int) -> Color:
	var group: Dictionary = _groups.get(key, {})
	var buffer: PackedFloat32Array = group.get("buffer", PackedFloat32Array())
	var o := index * 16 + 12
	if o + 3 >= buffer.size():
		return Color(0, 0, 0, 0)
	return Color(buffer[o], buffer[o + 1], buffer[o + 2], buffer[o + 3])


## Positions (carte) de départ de toutes les instances posées au dernier placement (tests).
func instance_origins() -> PackedVector2Array:
	var out := PackedVector2Array()
	for key in _groups:
		var buffer: PackedFloat32Array = _groups[key]["buffer"]
		for i in int(_groups[key]["count"]):
			out.append(Vector2(buffer[i * 16 + 3], buffer[i * 16 + 11]))
	return out


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
	if excluded(a) or excluded(b):
		return false
	var ya := _height(a)
	var yb := _height(b)
	var length_m := length / _scale
	var speed := FolkModels.speed_of(role, activity)
	var custom := Color(length_m, speed, fposmod(phase, 1.0) * length_m - behind_m, (yb - ya) / _scale)
	return _push(role, activity, Vector3(a.x, ya, a.y), atan2(dir.x, dir.y), custom)


## Instance immobile (travaux des champs, bergers, bêtes) tournée de `yaw` (radians).
func add_static(role: String, activity: String, at: Vector2, yaw: float) -> bool:
	if excluded(at):
		return false
	return _push(role, activity, Vector3(at.x, _height(at), at.y), yaw, Color(0, 0, 0, 0))


func _height(p: Vector2) -> float:
	if _terrain != null:
		var t := Time.get_ticks_usec()
		var h := _terrain.surface_height_at(p.x, p.y)
		_height_usec += Time.get_ticks_usec() - t
		return h
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
	if _missing.has(key):
		return {}
	var t_create := Time.get_ticks_usec()
	var group := _create_group(key, role, activity, prop)
	_create_usec += Time.get_ticks_usec() - t_create
	if group.is_empty():
		# Modèle absent : échec mémorisé (pas une nouvelle tentative à chaque instance).
		_missing[key] = true
	return group


func _create_group(key: String, role: String, activity: String, prop: bool) -> Dictionary:
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


func _debug_dump() -> void:
	var cam := get_viewport().get_camera_3d() if is_inside_tree() else null
	print("FKDBG place focus=%s radius=%.2f scale=%.5f cam=%s level=%d" % [_focus, _radius, _scale, cam.global_position if cam else Vector3.ZERO, _level])
	for provider in _providers:
		print("FKDBG  provider %s %s" % [provider.get_script().get_global_name(), provider.get("stats")])
	for key in _groups:
		var g: Dictionary = _groups[key]
		var n := int(g["count"])
		if n == 0:
			continue
		var buf: PackedFloat32Array = g["buffer"]
		var o := Vector3(buf[3], buf[7], buf[11])
		var mmi := g["mmi"] as MultiMeshInstance3D
		var maabb := mmi.multimesh.mesh.get_aabb() if mmi.multimesh.mesh else AABB()
		var sh := _terrain.surface_height_at(o.x, o.z) if _terrain else 0.0
		var screen := ""
		if cam:
			var top := o + Vector3(0, maabb.size.y * _scale, 0)
			screen = "px_h=%.1f vis=%s" % [(cam.unproject_position(o) - cam.unproject_position(top)).length(), not cam.is_position_behind(o)]
		print("FKDBG  %s n=%d first=%s surf=%.3f mesh_h=%.2f world_h=%.3f %s aabb=%s visible=%s" % [key, n, o, sh, maabb.size.y, maabb.size.y * _scale, screen, mmi.multimesh.custom_aabb, mmi.is_visible_in_tree()])


## Vrai si `p` (carte) tombe dans l'emprise d'une ville emblématique (`exclusions`).
func excluded(p: Vector2) -> bool:
	for disk in exclusions:
		if p.distance_squared_to(Vector2(disk.x, disk.y)) < disk.z * disk.z:
			return true
	return false


## Cercles (x, z, rayon) des villes emblématiques d'une `SettlementLayer` : maquette L1
## (`landmark_zones`) et ville 1:1 (`landmark_cities.zone_of`), le plus grand des deux.
static func landmark_disks(layer: Node) -> PackedVector3Array:
	var disks := PackedVector3Array()
	if layer == null:
		return disks
	if layer.has_method("landmark_zones"):
		disks.append_array(layer.call("landmark_zones"))
	var cities: Variant = layer.get("landmark_cities")
	if cities is Node and (cities as Node).has_method("city_ids"):
		for id in (cities as Node).call("city_ids"):
			disks.append((cities as Node).call("zone_of", id))
	return disks


## Préchauffage : un élément de `_warm_queue` au moins, puis tant que `budget_usec` n'est pas
## épuisé.
func _warm_step(budget_usec: int) -> void:
	var t0 := Time.get_ticks_usec()
	while not _warm_queue.is_empty():
		var item: String = _warm_queue.pop_front()
		var parts := item.split(":")
		if parts[0] == "prop":
			_group(parts[1], "")
		else:
			_group(parts[1], parts[2])
		if Time.get_ticks_usec() - t0 >= budget_usec:
			break
	_warm_usec += Time.get_ticks_usec() - t0
	stats["warm_ms"] = float(_warm_usec) / 1000.0


## Préchauffage complet, d'un coup (tests, chargement).
func warm_all() -> void:
	_warm_step(1 << 60)


func is_warm() -> bool:
	return _warm_queue.is_empty()


## Vrai si le réservoir est à jour pour la vue courante (captures `--screenshot`).
func settled() -> bool:
	return _settled


## Contrôle chiffré d'une capture : instances dans le champ de `camera` et taille écran (px)
## de leur hauteur (médiane, min, max), par nature (figurines / accessoires).
func view_report(camera: Camera3D) -> Dictionary:
	var report := {"figures": _figures, "props": _props, "in_view": 0, "px_median": 0.0, "px_min": 0.0, "px_max": 0.0, "settled": _settled, "scale": _scale}
	if camera == null:
		return report
	var sizes: Array = []
	var frustum := camera.get_frustum()
	for key in _groups:
		var group: Dictionary = _groups[key]
		var n := int(group["count"])
		if n == 0 or not (group["mmi"] as MultiMeshInstance3D).is_visible_in_tree():
			continue
		var mesh := (group["mmi"] as MultiMeshInstance3D).multimesh.mesh
		var height := (mesh.get_aabb().end.y if mesh != null else HUMAN_HEIGHT_M) * _scale
		var buffer: PackedFloat32Array = group["buffer"]
		for i in n:
			var o := Vector3(buffer[i * 16 + 3], buffer[i * 16 + 7], buffer[i * 16 + 11])
			var inside := true
			for plane in frustum:
				if plane.is_point_over(o):
					inside = false
					break
			if not inside:
				continue
			sizes.append((camera.unproject_position(o) - camera.unproject_position(o + Vector3(0.0, height, 0.0))).length())
	sizes.sort()
	report["in_view"] = sizes.size()
	if not sizes.is_empty():
		report["px_median"] = snappedf(sizes[sizes.size() / 2], 0.1)
		report["px_min"] = snappedf(sizes[0], 0.1)
		report["px_max"] = snappedf(sizes[sizes.size() - 1], 0.1)
	return report
