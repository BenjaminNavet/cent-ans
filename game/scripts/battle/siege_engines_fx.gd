class_name SiegeEnginesFx
extends Node3D

## SG2 — engins de siège animés d'après le cœur. Chaque figurine d'engin (trébuchet, mangonneau,
## bombarde : `data/fx/siege_engines.json`, `engines`) est un modèle Blender à pièces nommées
## (`game/assets/models/siege/`, `tools/blender_scripts/siege_engines.py`) posé sur le
## repère de la figurine que dessine `BattleSoldiers` (les servants y restent, sans l'engin).
## - Trébuchet : le contrepoids tombe, la verge bascule, la fronde fouette et lâche la pierre
##   au tir du cœur (`get_shots`) ; puis la verge oscille, et le treuil la ramène au fil du
##   rechargement que donne le cœur (`get_units().reload / reload_period`). Quand le cœur
##   s'apprête à tirer (rechargement presque fini, régiment au tir), le basculement commence
##   en avance pour que la fronde lâche exactement à l'instant du tir.
## - Mangonneau : la verge claque contre la traverse ; treuil au rechargement.
## - Bombarde : recul du fût, mantelet relevé pendant le tir (éclair et fumée : `BattleEffects`).
## - Bélier et beffroi (nœuds de `BattleSiege`) : roues qui tournent et caisse qui oscille selon
##   le chemin parcouru (position du cœur) ; poutre du bélier balancée (`SiegeAssaultFx`).
## `release()` donne aux projectiles (`SiegeAssaultFx`, `BattleEffects`) le point et l'instant
## où la fronde ou la bouche les lâche. Rendu seulement : aucune règle ici.
## GA3-L5 : trébuchet et bélier peuvent être posés dans leur variante générée (`ga3_variant`,
## mêmes nœuds animés, `--no-ga3` pour les modèles procéduraux).

const SETTINGS_FILE := "fx/siege_engines.json"
const SWING_CURVE_FILE := "fx/trebuchet_swing_curve.json"  # AS8d, CC BY-SA 3.0 (fichier propre)
const WOOD_TINTS := {
	"Timber": Color(0.72, 0.58, 0.44),
	"TimberDark": Color(0.45, 0.35, 0.26),
}

static var _settings: Dictionary = {}
static var _scenes: Dictionary = {}  # modèle -> PackedScene (null : absent)
static var _materials: Dictionary = {}

var soldiers: BattleSoldiers
var effects: BattleEffects
var siege_view: BattleSiege
var time_now := 0.0
var cfg: Dictionary = {}

var _engines: Dictionary = {}  # id -> {model, figures: [Node3D], swing: float, fired: bool}
var _movers: Dictionary = {}  # id -> {last: Vector3, roll: float, sway: float}
var swings_started := 0  # tests et captures
var predicted_swings := 0
var crew: SiegeCrewFx  # SG3 : servants (null : figurines skinnées absentes)
var _camera: Variant = null  # position de la caméra à cette image (null : aucune)


## Réglages (`data/fx/siege_engines.json`), lus une fois ; `{}` si introuvables.
static func settings() -> Dictionary:
	if not _settings.is_empty():
		return _settings
	if DataFile.exists(SETTINGS_FILE):
		var parsed: Variant = DataFile.read_json(SETTINGS_FILE)
		if parsed is Dictionary:
			_settings = parsed
			_attach_swing_curve()
			return _settings
	return {}


## Courbe de bascule mesurée (fichier à part, licence propre) : posée sous `trebuchet.swing_curve`
## si présente ; sinon `_swing_progress` retombe sur la courbe procédurale d'origine.
static func _attach_swing_curve() -> void:
	if not DataFile.exists(SWING_CURVE_FILE) or not _settings.get("trebuchet") is Dictionary:
		return
	var curve: Variant = DataFile.read_json(SWING_CURVE_FILE)
	if curve is Dictionary:
		_settings["trebuchet"]["swing_curve"] = curve


## Modèle d'engin d'un type d'unité (`""` : pas de modèle animé).
static func model_of(unit_type: String) -> String:
	var engines: Dictionary = settings().get("engines", {})
	var model := str(engines.get(unit_type, ""))
	return model if model != "" and has_model(model) else ""


static func has_model(model: String) -> bool:
	return _scene(model) != null


static func _scene(model: String) -> PackedScene:
	if _scenes.has(model):
		return _scenes[model]
	var dir := str(settings().get("models_dir", "res://assets/models/siege/"))
	var path := dir + model + ".glb"
	var scene: PackedScene = load(path) as PackedScene if ResourceLoader.exists(path) else null
	_scenes[model] = scene
	return scene


## GA3-L5 (ADR 0140) : variante générée du modèle `model` (`ga3` des réglages, `_lod` suivi) ;
## "" si aucune, absente ou coupée par `--no-ga3`. Même hiérarchie et mêmes noms de nœuds que
## le modèle procédural (`tools/blender_scripts/ga3_siege_rig.py`) : l'animation est inchangée.
static func ga3_variant(model: String) -> String:
	if not Ga3Kit.requested():
		return ""
	var base := model.trim_suffix("_lod")
	var entry: Variant = (settings().get("ga3", {}) as Dictionary).get(base, null)
	if not (entry is Dictionary) or str((entry as Dictionary).get("model", "")) == "":
		return ""
	var variant := str(entry["model"]) + ("_lod" if model.ends_with("_lod") else "")
	return variant if _scene(variant) != null else ""


## Réglages de l'engin `kind` (`ram`, `siege_tower`…) pour le nœud `node` : ceux de sa variante
## GA3 (roues, cordes de la poutre) remplacent les communs.
static func kind_settings(kind: String, node: Node) -> Dictionary:
	var c: Dictionary = settings().get(kind, {})
	if node != null and node.has_meta("ga3"):
		var over: Variant = (settings().get("ga3", {}) as Dictionary).get(kind, null)
		if over is Dictionary:
			c = c.merged(over, true)
	return c


## Nouvelle instance du modèle `model` (sa variante GA3 si elle existe), habillée des matières
## de bataille ; null si absent. Le nœud garde le nom du modèle ; méta `ga3` = variante posée.
static func instantiate(model: String) -> Node3D:
	var variant := ga3_variant(model)
	var scene := _scene(variant if variant != "" else model)
	if scene == null:
		return null
	var node := scene.instantiate() as Node3D
	node.name = model.capitalize().replace(" ", "")
	if variant != "":
		node.set_meta("ga3", variant)
	_dress(node)
	return node


## Matières nommées du modèle (Blender) -> matières texturées de la bataille, en projection
## triplanaire locale (les pièces bougent : la texture les suit).
static func _dress(node: Node) -> void:
	if node is MeshInstance3D:
		var mesh_instance := node as MeshInstance3D
		var mesh := mesh_instance.mesh
		for s in mesh.get_surface_count():
			var mat := mesh.surface_get_material(s)
			var mat_name := mat.resource_name if mat != null else ""
			mesh_instance.set_surface_override_material(s, _material(mat_name, mat))
	for child in node.get_children():
		_dress(child)


static func _material(mat_name: String, fallback: Material) -> Material:
	if _materials.has(mat_name):
		return _materials[mat_name]
	var mat: Material = fallback
	match mat_name:
		"Timber", "TimberDark":
			mat = _wood(WOOD_TINTS[mat_name])
		"Hide":
			var hide := StandardMaterial3D.new()
			hide.albedo_color = Color(0.36, 0.25, 0.15)
			hide.roughness = 0.8
			hide.metallic_specular = 0.35
			mat = hide
		"Rope":
			var rope := StandardMaterial3D.new()
			rope.albedo_color = Color(0.52, 0.44, 0.31)
			rope.roughness = 1.0
			mat = rope
		"Iron":
			var iron := StandardMaterial3D.new()
			iron.albedo_color = Color(0.11, 0.11, 0.115)
			iron.metallic = 0.75
			iron.roughness = 0.5
			mat = iron
		"Stone":
			var stone := StandardMaterial3D.new()
			stone.albedo_color = Color(0.46, 0.44, 0.4)
			stone.roughness = 0.95
			mat = stone
	_materials[mat_name] = mat
	return mat


static func _wood(tint: Color) -> StandardMaterial3D:
	var mat := BattleSiege._textured("wood", tint)
	mat.uv1_world_triplanar = false
	mat.uv1_scale = Vector3(0.6, 0.6, 0.6)
	return mat


func setup(p_soldiers: BattleSoldiers, p_effects: BattleEffects, p_siege_view: BattleSiege) -> void:
	soldiers = p_soldiers
	effects = p_effects
	siege_view = p_siege_view
	cfg = settings()
	if SiegeCrewFx.enabled() and crew == null:
		crew = SiegeCrewFx.new()
		crew.name = "Crew"
		add_child(crew)
		crew.setup(soldiers)


## SG3 : distances des niveaux de détail des engins (`data/fx/siege_engines.json`, `lod`) :
## `simple_m` (maillage simplifié au-delà), `far_m` (pose ralentie, servants cachés au-delà ;
## même distance que les imposteurs de figurines BV3). Lisible par les préréglages de qualité.
static func lod_distances() -> Dictionary:
	var lod: Dictionary = settings().get("lod", {})
	return {"simple_m": float(lod.get("simple_m", 140.0)), "far_m": float(lod.get("far_m", BattleImpostors.DISTANCE)), "far_pose_hz": float(lod.get("far_pose_hz", 6.0))}


## Chaque image, avant les effets (les tirs y démarrent les basculements que `release()` lit).
func update(units: Array, shots: Variant, now: float, dt: float) -> void:
	time_now = now
	_camera = _camera_position()
	if crew != null:
		crew.begin(now)
	var seen := {}
	for unit in units:
		var id := int(unit["id"])
		if str(unit.get("render", "")) == "siege":
			var model := model_of(str(unit.get("type", "")))
			if model == "":
				continue
			seen[id] = true
			_update_engine(unit, id, model)
		elif siege_view != null and (str(unit.get("render", "")) == "ram" or str(unit.get("render", "")) == "tower"):
			_update_mover(unit, id, dt)
	if shots is Array:
		for shot in shots:
			var id := int((shot as Dictionary).get("shooter", -1))
			if _engines.has(id):
				_on_shot(id)
	for id in _engines.keys():
		if not seen.has(id):
			_engines[id]["shown"] = 0
			for node in _engines[id]["figures"] + _engines[id].get("lods", []):
				if node != null:
					(node as Node3D).visible = false
	for id in _engines:
		_pose_engine(id, _engines[id])
	if crew != null:
		crew.finish(_camera)


func _camera_position() -> Variant:
	var viewport := get_viewport()
	var camera := viewport.get_camera_3d() if viewport != null else null
	return camera.global_position if camera != null else null


func _distance(p: Vector3) -> float:
	return 0.0 if _camera == null else (_camera as Vector3).distance_to(p)


# --- Servants (SG3) ---------------------------------------------------------------------


## Servants de la figurine `i` de l'engin `id` : geste de chaque rôle d'après le rechargement
## du cœur (treuil tant que la verge remonte, chargeur en fin de treuil ; bombarde : écouvillon
## puis charge de la poudre et du boulet), repos sinon.
func _crew_engine(id: int, i: int, node: Node3D, model: String, c: Dictionary, unit: Dictionary, entry: Dictionary, tau: float) -> void:
	var layout: Array = crew.cfg.get("layouts", {}).get(model, [])
	if layout.is_empty():
		return
	var fired := bool(entry["fired"])
	var reloading := fired and float(unit.get("reload", 0.0)) > 0.0
	var side := str(unit.get("side", ""))
	var period := maxf(float(unit.get("reload_period", 12.0)), 0.1)
	var phase := 1.0 - float(unit.get("reload", 0.0)) / period  # 0 : vient de tirer, 1 : prêt
	var settled := tau >= float(c.get("swing_s", 0.0)) + float(c.get("settle_s", 0.0))
	var wound := _wound(c, unit, fired)
	for k in layout.size():
		var s: Dictionary = layout[k]
		var role := str(s["role"])
		var act := "idle"
		match role:
			"winch":
				if reloading and settled and wound < 0.98:
					act = "winch"
			"loader":
				if model == "bombard":
					var span: Array = crew.cfg.get("bombard_load", [0.45, 0.9])
					if reloading and phase >= float(span[0]) and phase < float(span[1]):
						act = "loader"
				elif reloading and settled and wound >= float(crew.cfg.get("load_from", 0.55)) and wound < 0.98:
					act = "loader"
			"swab":
				if reloading and tau > float(c.get("recoil_s", 0.1)) + float(c.get("hold_s", 0.0)) and phase < float(crew.cfg.get("swab_until", 0.5)):
					act = "swab"
		if act == "idle" and s.has("rest"):
			# Au repos hors du souffle de la bouche (bombarde) : place de repos.
			var rest: Array = s["rest"]
			s = {"x": rest[0], "z": rest[1], "yaw_deg": rest[2], "figure": s.get("figure", 0)}
		_add_servant("e%d/%d/%d" % [id, i, k], node.global_transform, s, crew.clip_of(act, model, k), side)
	crew.add_haulers(id * 100 + i, model, node.global_transform, phase if reloading else -1.0, side)  # AS4


func _add_servant(key: String, frame: Transform3D, s: Dictionary, clip: String, side: String) -> void:
	var local := Transform3D(Basis(Vector3.UP, deg_to_rad(float(s["yaw_deg"]))), Vector3(float(s["x"]), 0.0, float(s["z"])))
	var xform := frame * local
	# Pieds au sol (l'engin peut pencher : repère remis d'aplomb).
	xform.basis = Basis(Vector3.UP, atan2(xform.basis.z.x, xform.basis.z.z))
	crew.add(key, xform, clip, int(s.get("figure", 0)), side)


# --- Engins à tir ----------------------------------------------------------------------


func _update_engine(unit: Dictionary, id: int, model: String) -> void:
	if not _engines.has(id):
		_engines[id] = {"model": model, "figures": [], "lods": [], "shown": 0, "swing": -1000.0, "fired": false, "unit": unit, "mantlet": 0.0}
	var entry: Dictionary = _engines[id]
	entry["unit"] = unit
	var figures: Array = entry["figures"]
	var lods: Array = entry["lods"]
	var present := bool(unit.get("present", false))
	var count := int(unit.get("figures", 0)) if present else 0
	var frames := _frames(unit, id, count)
	while figures.size() < frames.size():
		var node := instantiate(model)
		if node == null:
			return
		add_child(node)
		figures.append(node)
		# SG3 : maillage simplifié (`<modèle>_lod.glb`, mêmes pièces nommées) pour le lointain.
		var lod := instantiate(model + "_lod") if has_model(model + "_lod") else null
		if lod != null:
			add_child(lod)
		lods.append(lod)
	entry["shown"] = frames.size()
	var simple := float(lod_distances()["simple_m"])
	for i in figures.size():
		var node: Node3D = figures[i]
		var lod: Node3D = lods[i]
		var shown := i < frames.size()
		var far := lod != null and shown and _distance(frames[i].origin) > simple
		node.visible = shown and not far
		if lod != null:
			lod.visible = far
		if shown:
			node.transform = frames[i]
			if lod != null:
				lod.transform = frames[i]
	_maybe_predict(unit, entry)


## Repères des figurines d'engin (position des figurines de `BattleSoldiers`, cap du régiment).
func _frames(unit: Dictionary, id: int, count: int) -> Array[Transform3D]:
	var out: Array[Transform3D] = []
	if count <= 0:
		return out
	var basis := Basis(Vector3.UP, float(unit.get("facing", 0.0)))
	var positions := PackedVector3Array()
	if soldiers != null and soldiers._previous.has(id):
		var slice: PackedFloat32Array = soldiers._previous[id]
		for k in mini(soldiers.figure_count(id), count):
			positions.append(Vector3(slice[k * 12 + 3], slice[k * 12 + 7], slice[k * 12 + 11]))
	if positions.is_empty():
		positions.append(Vector3(float(unit["x"]), float(unit.get("y", 0.0)), float(unit["z"])))
	for p in positions:
		out.append(Transform3D(basis, p))
	return out


func _kind_cfg(model: String) -> Dictionary:
	return cfg.get(model, {})


## Temps entre le début du basculement (figurine 0) et le lâcher de la pierre.
func _lead(model: String) -> float:
	var c := _kind_cfg(model)
	if model == "bombard":
		return 0.0
	return float(c.get("swing_s", 1.0)) * float(c.get("release_phase", 0.6))


## Basculement anticipé : le cœur tire dès que le rechargement est fini (régiment au tir,
## munitions) ; on commence la bascule `lead` secondes avant pour lâcher au tir.
func _maybe_predict(unit: Dictionary, entry: Dictionary) -> void:
	var model := str(entry["model"])
	var c := _kind_cfg(model)
	if model == "bombard" or not bool(c.get("predict", false)):
		return
	var reload := float(unit.get("reload", 0.0))
	var lead := _lead(model)
	if reload <= 0.0 or reload > lead or str(unit.get("state", "")) != "shooting" or int(unit.get("ammo", 0)) <= 0:
		return
	var since := time_now - float(entry["swing"])
	if since < float(c.get("swing_s", 1.0)) + float(c.get("settle_s", 0.0)):
		return  # basculement en cours
	entry["swing"] = time_now - (lead - reload)
	entry["predicted"] = true
	entry["fired"] = true
	swings_started += 1
	predicted_swings += 1


## Tir du cœur : recale le basculement pour que la figurine 0 lâche maintenant (anticipé), ou
## le démarre (lâcher `lead` s plus tard).
func _on_shot(id: int) -> void:
	var entry: Dictionary = _engines[id]
	var model := str(entry["model"])
	var lead := _lead(model)
	var since := time_now - float(entry["swing"])
	if bool(entry.get("predicted", false)) and since >= 0.0 and since <= lead + 0.6:
		entry["swing"] = time_now - lead
	else:
		entry["swing"] = time_now
		swings_started += 1
	entry["predicted"] = false
	entry["fired"] = true


## Pour chaque figurine d'engin visible du régiment `id` : `{pos, dir, delay}` = point où la
## pierre quitte la fronde (ou la bouche), direction du tir, délai avant ce lâcher. `[]` si le
## régiment n'a pas de modèle animé (les projectiles gardent leur ancien départ).
func release(id: int) -> Array:
	var out: Array = []
	if not _engines.has(id):
		return out
	var entry: Dictionary = _engines[id]
	var model := str(entry["model"])
	var c := _kind_cfg(model)
	var stagger := float(c.get("stagger_s", 0.3))
	var lead := _lead(model)
	var figures: Array = entry["figures"]
	for i in figures.size():
		var node: Node3D = figures[i]
		if i >= int(entry.get("shown", 0)):
			continue
		var start := float(entry["swing"]) + stagger * i
		var delay := maxf(start + lead - time_now, 0.0)
		var pos := node.global_position + Vector3(0, 3.0, 0)
		var forward := node.global_basis.z.normalized()
		match model:
			"trebuchet":
				pos = _stone_at_release(node, c)
			"mangonel":
				pos = _cup_at_release(node, c)
			"bombard":
				var barrel := node.find_child("Barrel", true, false) as Node3D
				if barrel != null:
					pos = barrel.global_position + forward * 1.6
		out.append({"pos": pos, "dir": forward, "delay": delay})
	return out


func _stone_at_release(node: Node3D, c: Dictionary) -> Vector3:
	var arm := node.find_child("Arm", true, false) as Node3D
	var sling := node.find_child("Sling", true, false) as Node3D
	var stone := node.find_child("Stone", true, false) as Node3D
	if arm == null or sling == null or stone == null:
		return node.global_position + Vector3(0, 9, 0)
	var keep := [arm.rotation.x, sling.rotation.x]
	var u := float(c.get("release_phase", 0.6))
	var pose := _trebuchet_swing(c, u)
	arm.rotation.x = pose.x
	sling.rotation.x = pose.y
	var pos := stone.global_position
	arm.rotation.x = keep[0]
	sling.rotation.x = keep[1]
	return pos


func _cup_at_release(node: Node3D, c: Dictionary) -> Vector3:
	var arm := node.find_child("Arm", true, false) as Node3D
	var stone := node.find_child("Stone", true, false) as Node3D
	if arm == null or stone == null:
		return node.global_position + Vector3(0, 3, 0)
	var keep := arm.rotation.x
	var u := float(c.get("release_phase", 0.8))
	arm.rotation.x = deg_to_rad(lerpf(float(c["cocked_deg"]), float(c["fired_deg"]), u * u))
	var pos := stone.global_position
	arm.rotation.x = keep
	return pos


## Part de l'arc (0 : armé, 1 : fin du dépassement) à la phase `u`. Courbe mesurée sur la vidéo
## d'un vrai trébuchet (`swing_curve.lut`, lot AS8d) quand elle est fournie, sinon la courbe
## procédurale d'origine (verge qui accélère sous le contrepoids).
static func _swing_progress(c: Dictionary, u: float) -> float:
	var curve: Variant = c.get("swing_curve", null)
	if curve is Dictionary:
		var lut: Array = curve.get("lut", [])
		if lut.size() >= 2:
			var x := clampf(u, 0.0, 1.0) * float(lut.size() - 1)
			var i := mini(int(x), lut.size() - 2)
			return lerpf(float(lut[i]), float(lut[i + 1]), x - float(i))
	return u * u * (2.2 - 1.2 * u)


## Pose (verge, fronde relative) en radians à la phase `u` (0-1) du basculement du trébuchet :
## la verge accélère (contrepoids qui tombe), la fronde traîne puis fouette par-dessus.
static func _trebuchet_swing(c: Dictionary, u: float) -> Vector2:
	var cocked := float(c["cocked_deg"])
	var over := float(c["overswing_deg"])
	var theta := lerpf(cocked, over, _swing_progress(c, u))
	var u_rel := float(c["release_phase"])
	var phi: float
	if u <= u_rel:
		phi = float(c["sling_release_deg"]) * pow(u / u_rel, float(c["sling_power"]))
	else:
		phi = lerpf(float(c["sling_release_deg"]), float(c["sling_end_deg"]), (u - u_rel) / (1.0 - u_rel))
	return Vector2(deg_to_rad(theta), deg_to_rad(phi - theta))


func _pose_engine(id: int, entry: Dictionary) -> void:
	var model := str(entry["model"])
	var c := _kind_cfg(model)
	var unit: Dictionary = entry["unit"]
	var figures: Array = entry["figures"]
	var stagger := float(c.get("stagger_s", 0.3))
	var lods: Array = entry.get("lods", [])
	var lod_cfg := lod_distances()
	for i in figures.size():
		if i >= int(entry.get("shown", 0)):
			continue
		var node: Node3D = figures[i]
		var tau := time_now - (float(entry["swing"]) + stagger * i)
		if crew != null:
			_crew_engine(id, i, node, model, c, unit, entry, tau)
		if i < lods.size() and lods[i] != null and (lods[i] as Node3D).visible:
			# SG3 : au loin, le maillage simplifié ; au-delà de `far_m`, pose rafraîchie
			# `far_pose_hz` fois par seconde seulement.
			node = lods[i]
			if _distance(node.global_position) > float(lod_cfg["far_m"]):
				var tick := int(floor(time_now * float(lod_cfg["far_pose_hz"])))
				if int(node.get_meta("pose_tick", -1)) == tick:
					continue
				node.set_meta("pose_tick", tick)
		match model:
			"trebuchet":
				_pose_trebuchet(node, c, unit, tau, bool(entry["fired"]))
			"mangonel":
				_pose_mangonel(node, c, unit, tau, bool(entry["fired"]))
			"bombard":
				_pose_bombard(node, c, unit, entry, tau)


## Part du treuil accomplie (0 : verge au repos après le tir, 1 : armé), d'après le
## rechargement du cœur.
static func _wound(c: Dictionary, unit: Dictionary, fired: bool) -> float:
	if not fired:
		return 1.0
	var period := maxf(float(unit.get("reload_period", 12.0)), 0.1)
	var reload := float(unit.get("reload", 0.0))
	var margin := float(c.get("ready_margin_s", 1.0))
	var span := maxf(period - float(c.get("swing_s", 1.0)) - float(c.get("settle_s", 1.0)) - margin, 0.5)
	return clampf(1.0 - (reload - margin) / span, 0.0, 1.0)


func _pose_trebuchet(node: Node3D, c: Dictionary, unit: Dictionary, tau: float, fired: bool) -> void:
	var arm := node.find_child("Arm", true, false) as Node3D
	var sling := node.find_child("Sling", true, false) as Node3D
	var stone := node.find_child("Stone", true, false) as Node3D
	var counter := node.find_child("Counterweight", true, false) as Node3D
	var winch := node.find_child("Winch", true, false) as Node3D
	if arm == null or sling == null:
		return
	var swing_s := float(c["swing_s"])
	var settle_s := float(c["settle_s"])
	var rest := deg_to_rad(float(c["rest_deg"]))
	var theta: float
	var rel: float
	var omega := 0.0
	var loaded := false
	if tau >= 0.0 and tau < swing_s:
		var u := tau / swing_s
		var pose := _trebuchet_swing(c, u)
		theta = pose.x
		rel = pose.y
		omega = (_trebuchet_swing(c, minf(u + 0.02, 1.0)).x - theta) / (0.02 * swing_s)
		loaded = u < float(c["release_phase"])
	elif tau >= swing_s and tau < swing_s + settle_s:
		# Oscillation amortie autour de la verticale ; la fronde vide pend et se balance.
		var t := tau - swing_s
		var over := deg_to_rad(float(c["overswing_deg"]))
		var w := TAU * float(c["settle_hz"])
		var decay := exp(-float(c["settle_damping"]) * t)
		theta = rest + (over - rest) * cos(w * t) * decay
		omega = -(over - rest) * w * sin(w * t) * decay
		var hang := deg_to_rad(90.0 + 25.0 * sin(w * t * 1.3) * decay)
		rel = hang - theta
	else:
		var wound := _wound(c, unit, fired)
		var k := smoothstep(0.0, 1.0, wound)
		theta = lerpf(rest, deg_to_rad(float(c["cocked_deg"])), k)
		# Fronde pendante quand la verge est haute, couchée dans l'auge une fois armée.
		rel = deg_to_rad(lerpf(90.0, 0.0, k)) - theta
		loaded = wound >= 0.98
		if winch != null:
			winch.rotation.x = -wound * float(c["winch_turns"]) * TAU
	arm.rotation.x = theta
	sling.rotation.x = rel
	if counter != null:
		counter.rotation.x = -theta - clampf(omega * float(c["counterweight_lag"]), -0.7, 0.7)
	if stone != null:
		stone.visible = loaded


func _pose_mangonel(node: Node3D, c: Dictionary, unit: Dictionary, tau: float, fired: bool) -> void:
	var arm := node.find_child("Arm", true, false) as Node3D
	var stone := node.find_child("Stone", true, false) as Node3D
	var winch := node.find_child("Winch", true, false) as Node3D
	if arm == null:
		return
	var swing_s := float(c["swing_s"])
	var settle_s := float(c["settle_s"])
	var cocked := float(c["cocked_deg"])
	var fired_deg := float(c["fired_deg"])
	var angle: float
	var loaded := false
	if tau >= 0.0 and tau < swing_s:
		var u := tau / swing_s
		angle = lerpf(cocked, fired_deg, u * u)
		loaded = u < float(c["release_phase"])
	elif tau >= swing_s and tau < swing_s + settle_s:
		var t := tau - swing_s
		angle = fired_deg - float(c["bounce_deg"]) * absf(sin(t * 14.0)) * exp(-4.0 * t)
	else:
		var wound := _wound(c, unit, fired)
		angle = lerpf(fired_deg, cocked, smoothstep(0.0, 1.0, wound))
		loaded = wound >= 0.98
		if winch != null:
			winch.rotation.x = -wound * float(c["winch_turns"]) * TAU
	arm.rotation.x = deg_to_rad(angle)
	if stone != null:
		stone.visible = loaded


func _pose_bombard(node: Node3D, c: Dictionary, unit: Dictionary, entry: Dictionary, tau: float) -> void:
	var barrel := node.find_child("Barrel", true, false) as Node3D
	var mantlet := node.find_child("Mantlet", true, false) as Node3D
	if barrel != null:
		if not barrel.has_meta("rest_z"):
			barrel.set_meta("rest_z", barrel.position.z)
		var back := 0.0
		var recoil_s := float(c["recoil_s"])
		var hold := float(c["hold_s"])
		if tau >= 0.0 and bool(entry["fired"]):
			if tau < recoil_s:
				back = tau / recoil_s
			elif tau < recoil_s + hold:
				back = 1.0
			else:
				back = 1.0 - smoothstep(0.0, 1.0, (tau - recoil_s - hold) / float(c["return_s"]))
		barrel.position.z = float(barrel.get_meta("rest_z")) - float(c["recoil_m"]) * back
	if mantlet != null:
		# Mantelet relevé tant que le régiment tire (baissé pour recharger à couvert).
		var want := 1.0 if str(unit.get("state", "")) == "shooting" else 0.0
		var step := 1.0 / maxf(float(c["mantlet_s"]), 0.05) * get_process_delta_time() * Engine.time_scale
		entry["mantlet"] = move_toward(float(entry["mantlet"]), want, maxf(step, 0.02))
		mantlet.rotation.x = deg_to_rad(float(c["mantlet_raised_deg"])) * smoothstep(0.0, 1.0, float(entry["mantlet"]))


# --- Bélier et beffroi -----------------------------------------------------------------


## Roues qui tournent du chemin parcouru, caisse qui tangue un peu en roulant.
func _update_mover(unit: Dictionary, id: int, _dt: float) -> void:
	var machine: Node3D = siege_view._machines.get(id)
	if machine == null or not machine.visible:
		return
	var is_ram := str(unit.get("render", "")) == "ram"
	var c := kind_settings("ram" if is_ram else "siege_tower", machine)
	var pos := Vector3(float(unit["x"]), 0.0, float(unit["z"]))
	if not _movers.has(id):
		_movers[id] = {"last": pos, "roll": 0.0, "sway": 0.0}
	var state: Dictionary = _movers[id]
	var delta: Vector3 = pos - (state["last"] as Vector3)
	state["last"] = pos
	var forward := Vector3(sin(float(unit.get("facing", 0.0))), 0.0, cos(float(unit.get("facing", 0.0))))
	var moved := delta.dot(forward)
	if delta.length() > 20.0:
		moved = 0.0  # téléportation (déploiement)
	state["roll"] = float(state["roll"]) + moved / maxf(float(c.get("wheel_radius", 0.6)), 0.1)
	state["sway"] = float(state["sway"]) + absf(moved) * float(c.get("sway_per_m", 0.5)) * TAU
	for wheel in machine.find_children("Wheel*", "", true, false):
		(wheel as Node3D).rotation.x = float(state["roll"])
	var amp := deg_to_rad(float(c.get("sway_deg", 1.0)))
	var moving := absf(moved) > 0.001
	if moving:
		state["moved_at"] = time_now
	if crew != null:
		# SG3 : poussée tant que l'engin avance (une seconde de grâce entre deux pas du cœur) ;
		# à l'arrêt, les servants du bélier tirent les cordes de la poutre.
		var pushing := time_now - float(state.get("moved_at", -1000.0)) < 1.0
		var still := "pusher_still" if is_ram else "idle"
		var layout: Array = crew.cfg.get("layouts", {}).get("ram" if is_ram else "siege_tower", [])
		for k in layout.size():
			_add_servant("m%d/%d" % [id, k], machine.global_transform, layout[k], crew.clip_of("pusher" if pushing else still, "ram" if is_ram else "siege_tower", k), str(unit.get("side", "")))
	var body := machine.find_child("Shed" if is_ram else "Body", true, false) as Node3D
	if body != null:
		var target := amp * sin(float(state["sway"])) if moving else 0.0
		body.rotation.z = lerpf(body.rotation.z, target, 0.2)
		body.rotation.x = lerpf(body.rotation.x, amp * 0.5 * sin(float(state["sway"]) * 0.5) if moving else 0.0, 0.2)
