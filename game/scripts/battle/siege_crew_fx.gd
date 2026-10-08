class_name SiegeCrewFx
extends Node3D

## SG3 — servants des engins de siège : figurines skinnées V2 (`crew_0` sans arme, `crew_1` au
## refouloir ; `tools/blender_scripts/battle_skinned_figures.py`) qui manœuvrent l'engin au fil du
## rechargement exposé par le cœur (`SiegeEnginesFx` choisit le clip de chaque servant : treuil
## `crank`, chargement `load`, écouvillon `swab`, poussée `push`, corde `haul`, repos `idle`).
## Un MultiMesh par (figurine, clip, camp) en mode CUSTOM du shader skinné : INSTANCE_CUSTOM.x =
## instant où le servant a commencé son geste (le clip boucle depuis cet instant) ; NT10 : y =
## geste précédent en fondu (`BattleSkinned.pack_fade`), z = début de ce geste précédent. Chaque servant
## garde le même rang d'instance dans toutes les couches : même variante (tête, bonnet) quel que
## soit son geste. Rendu seulement : aucune règle ici.

const FLOATS := 16  # 12 (transform) + 4 (custom)

var soldiers: BattleSoldiers
var cfg: Dictionary = {}
var time_now := 0.0
var shown := 0  # servants dessinés à la dernière image (tests, captures)

var _layers: Dictionary = {}  # "figure/clip/side" -> {mmi, mat, figure}
var _slots: Dictionary = {}  # clé du servant -> rang d'instance
var _state: Dictionary = {}  # clé du servant -> {clip, since}
var _frame: Array = []  # [{key, xform, clip, figure, side}]
var _capacity := 16
var _props_frame: Array = []  # AS4 : [{object, xform}] (tas de munitions, objet porté)
var _props: Dictionary = {}  # AS4 : objet -> {mmi, capacity}
var props_shown := 0  # AS4 : objets dessinés à la dernière image (tests, captures)


## Vrai quand les réglages `crew` et la figurine skinnée des servants existent.
static func enabled() -> bool:
	var c: Dictionary = SiegeEnginesFx.settings().get("crew", {})
	return not c.is_empty() and BattleSkinned.has_figure(str(c.get("figure_kind", "crew")), 0)


func setup(p_soldiers: BattleSoldiers) -> void:
	soldiers = p_soldiers
	cfg = SiegeEnginesFx.settings().get("crew", {})


func begin(now: float) -> void:
	time_now = now
	_frame.clear()
	_props_frame.clear()


## Clip d'un rôle (`clips` des réglages), ou le nom tel quel. NT7 : `clips_by_engine[engine]`
## surcharge le rôle pour un type d'engin (chargeur de trébuchet : pierre lourde) ; une liste
## alterne les gestes d'un servant à l'autre (`index`) ; un clip absent du rig (kit antérieur)
## revient au premier de la liste ou au clip par défaut du rôle.
func clip_of(role: String, engine: String = "", index: int = 0) -> String:
	var default: Variant = cfg.get("clips", {}).get(role, role)
	var value: Variant = (cfg.get("clips_by_engine", {}) as Dictionary).get(engine, {}).get(role, default)
	var options: Array = value if value is Array else [value]
	var fallback: Array = default if default is Array else [default]
	var clip := str(options[posmod(index, options.size())]) if not options.is_empty() else role
	var rig_clips: Dictionary = {}
	var kind := str(cfg.get("figure_kind", "crew"))
	if BattleSkinned.has_figure(kind, 0):
		rig_clips = BattleSkinned.rig(kind, 0).get("clips", {})
	if rig_clips.is_empty() or rig_clips.has(clip):
		return clip
	for c in options + fallback:
		if rig_clips.has(str(c)):
			return str(c)
	return str(fallback[0]) if not fallback.is_empty() else role


## Servant `key` (stable d'une image à l'autre) posé en `xform`, faisant le geste `clip`.
func add(key: String, xform: Transform3D, clip: String, figure: int, side: String) -> void:
	_frame.append({"key": key, "xform": xform, "clip": clip, "figure": figure, "side": side})


## Répartit les servants de l'image dans les couches ; `camera` : position de la caméra (null :
## pas de caméra, détail maximal).
func finish(camera: Variant) -> void:
	var kind := str(cfg.get("figure_kind", "crew"))
	var hide := float(cfg.get("hide_beyond_m", 300.0))
	var detail := float(cfg.get("detail_m", 24.0))
	var medium := float(cfg.get("medium_m", 75.0))
	var buckets: Dictionary = {}  # layer key -> [entries]
	var nearest: Dictionary = {}  # layer key -> distance
	shown = 0
	for e in _frame:
		var xform: Transform3D = e["xform"]
		var dist := 0.0 if camera == null else (camera as Vector3).distance_to(xform.origin)
		if dist > hide:
			continue
		var key := str(e["key"])
		if not _slots.has(key):
			_slots[key] = _slots.size()
		var st: Dictionary = _state.get(key, {})
		if str(st.get("clip", "")) != str(e["clip"]):
			# Nouveau geste : le clip repart de son début (léger décalage entre servants). NT10 :
			# l'ancien geste reste lu (depuis son propre début) le temps du fondu.
			var fresh := {"clip": str(e["clip"]), "since": time_now - 0.13 * float(int(_slots[key]) % 5)}
			if not st.is_empty() and BattleSkinned.role_blend_s() > 0.0:
				fresh["prev"] = BattleSkinned.clip_index(BattleSkinned.rig(kind, int(e["figure"])), str(st["clip"]))
				fresh["prev_since"] = float(st["since"])
				fresh["at"] = time_now
			st = fresh
			_state[key] = st
		if st.has("prev") and (time_now - float(st["at"]) > BattleSkinned.role_blend_s() + 0.1 or time_now < float(st["at"])):
			st.erase("prev")
		e["since"] = float(st["since"])
		e["fade_y"] = BattleSkinned.pack_fade(0, int(st.get("prev", -1)), float(st.get("at", 0.0)))
		e["prev_since"] = float(st.get("prev_since", 0.0))
		var layer_key := "%d/%s/%s" % [int(e["figure"]), str(e["clip"]), str(e["side"])]
		if not buckets.has(layer_key):
			buckets[layer_key] = []
			nearest[layer_key] = dist
		(buckets[layer_key] as Array).append(e)
		nearest[layer_key] = minf(float(nearest[layer_key]), dist)
		shown += 1
	while _capacity < _slots.size():
		_capacity *= 2
	for layer_key in buckets:
		var entries: Array = buckets[layer_key]
		var first: Dictionary = entries[0]
		var layer := _layer(layer_key, kind, int(first["figure"]), str(first["clip"]), str(first["side"]))
		var d := float(nearest[layer_key])
		var level := 0 if d < detail else 1 if d < medium else 2
		var mmi: MultiMeshInstance3D = layer["mmi"]
		var mm := mmi.multimesh
		if int(layer.get("level", -1)) != level:
			layer["level"] = level
			mm.mesh = BattleSkinned.mesh(kind, int(first["figure"]), level)
		mmi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON if d < medium * 2.0 else GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		var buf := PackedFloat32Array()
		buf.resize(_capacity * FLOATS)  # rangs inutilisés : échelle nulle (invisibles)
		for e in entries:
			var o := int(_slots[str(e["key"])]) * FLOATS
			var t: Transform3D = e["xform"]
			var b := t.basis
			var values := [b.x.x, b.y.x, b.z.x, t.origin.x, b.x.y, b.y.y, b.z.y, t.origin.y, b.x.z, b.y.z, b.z.z, t.origin.z, float(e["since"]), float(e["fade_y"]), float(e["prev_since"]), 0.0]
			for k in FLOATS:
				buf[o + k] = values[k]
		if mm.instance_count != _capacity:
			mm.instance_count = _capacity
		mm.buffer = buf
		mm.visible_instance_count = _capacity
		mmi.visible = true
		(layer["mat"] as ShaderMaterial).set_shader_parameter("anim_time", time_now)
	for layer_key in _layers:
		if not buckets.has(layer_key):
			(_layers[layer_key]["mmi"] as MultiMeshInstance3D).visible = false
	_finish_props(camera, hide)


# --- AS4 : servants porteurs de munitions ------------------------------------------------


## Vrai quand les porteurs sont actifs : réglage `crew.haul.enabled` et pas de `--no-as4` (banc A/B).
func haul_enabled() -> bool:
	return bool(cfg.get("haul", {}).get("enabled", false)) and not CmdArgs.has("--no-as4")


## Étape du trajet d'un porteur à la phase `phase` du rechargement (0 : vient de tirer, 1 : prêt ;
## négatif : l'engin ne recharge pas). `trip` = [départ, arrivée au tas, saisie finie, retour au poste,
## charge finie]. Renvoie {stage: idle|out|grab|back|drop, t: avancement 0..1 dans l'étape,
## holding: l'objet est en main}.
static func haul_stage(trip: Array, phase: float) -> Dictionary:
	if phase < 0.0 or phase < float(trip[0]) or phase >= float(trip[4]):
		return {"stage": "idle", "t": 0.0, "holding": false}
	for i in 4:
		var a := float(trip[i])
		var b := float(trip[i + 1])
		if phase < b:
			var t := clampf((phase - a) / maxf(b - a, 0.0001), 0.0, 1.0)
			match i:
				0:
					return {"stage": "out", "t": t, "holding": false}
				1:
					return {"stage": "grab", "t": t, "holding": t > 0.6}
				2:
					return {"stage": "back", "t": t, "holding": true}
				_:
					return {"stage": "drop", "t": t, "holding": t < 0.5}
	return {"stage": "idle", "t": 0.0, "holding": false}


## Pose locale (x, z, cap rad) d'un porteur entre son poste et son tas selon l'étape.
func haul_pose(post: Array, pile_xz: Vector2, stage: Dictionary) -> Vector3:
	var post_xz := Vector2(float(post[0]), float(post[1]))
	var post_yaw := deg_to_rad(float(post[2]))
	var t := float(stage["t"])
	var to_pile := pile_xz - post_xz
	var yaw_out := atan2(to_pile.x, to_pile.y)
	match str(stage["stage"]):
		"out":
			var p := post_xz.lerp(pile_xz, smoothstep(0.0, 1.0, t))
			return Vector3(p.x, p.y, yaw_out)
		"grab":
			return Vector3(pile_xz.x, pile_xz.y, yaw_out)
		"back":
			var p := pile_xz.lerp(post_xz, smoothstep(0.0, 1.0, t))
			return Vector3(p.x, p.y, atan2(-to_pile.x, -to_pile.y))
	return Vector3(post_xz.x, post_xz.y, post_yaw)


## Porteurs et tas de munitions de l'engin `engine_id` (modèle `model`) posé en `frame` ; `phase` :
## voir `haul_stage`. Les tas sont toujours dessinés ; l'objet porté n'apparaît qu'en main.
func add_haulers(engine_id: int, model: String, frame: Transform3D, phase: float, side: String) -> void:
	if not haul_enabled():
		return
	var haul: Dictionary = cfg["haul"]
	var spec: Dictionary = haul.get("by_engine", {}).get(model, {})
	if spec.is_empty():
		return
	var level := Transform3D(Basis(Vector3.UP, atan2(frame.basis.z.x, frame.basis.z.z)), frame.origin)
	var piles: Array = spec["piles"]
	for pi in piles.size():
		var pile: Dictionary = piles[pi]
		for n in int(pile["count"]):
			var ang := 2.399963 * float(n)  # angle d'or : tas sans motif visible
			var rad := float(pile["spread"]) * sqrt((float(n) + 0.5) / float(pile["count"]))
			var at := Vector3(float(pile["x"]) + cos(ang) * rad, 0.0, float(pile["z"]) + sin(ang) * rad)
			_props_frame.append({"object": str(pile["object"]), "xform": level * Transform3D(Basis(Vector3.UP, ang), at), "ground": true})
	var haulers: Array = spec["haulers"]
	for hi in haulers.size():
		var h: Dictionary = haulers[hi]
		var pile: Dictionary = piles[int(h["pile"])]
		var stage := haul_stage(h["trip"], phase)
		var pose := haul_pose(h["post"], Vector2(float(pile["x"]), float(pile["z"])), stage)
		var carrying := str(stage["stage"]) == "back" or (str(stage["stage"]) in ["grab", "drop"] and bool(stage["holding"]))
		var clip := clip_of("idle", model, hi)
		match str(stage["stage"]):
			"out":
				clip = _haul_clip(str(haul["walk_clip"]), false)
			"grab":
				clip = _haul_clip(str(haul["grab_clip"]), false)
			"back":
				clip = _haul_clip(str(haul["carry_clip"]), true)
			"drop":
				clip = clip_of("loader", model, hi)  # même geste que le chargeur de l'engin
		var xform := level * Transform3D(Basis(Vector3.UP, pose.z), Vector3(pose.x, 0.0, pose.y))
		add("h%d/%d" % [engine_id, hi], xform, clip, int(h["figure"]), side)
		if carrying:
			var hold: Array = h["hold"]
			_props_frame.append({"object": str(pile["object"]), "xform": xform * Transform3D(Basis(), Vector3(float(hold[0]), float(hold[1]), float(hold[2]))), "ground": false})


## Clip demandé s'il existe dans le rig des servants ; `carry` absent (kit grossier) : `walk`.
func _haul_clip(clip: String, carrying: bool) -> String:
	var rig_clips: Dictionary = BattleSkinned.rig(str(cfg.get("figure_kind", "crew")), 0).get("clips", {})
	if rig_clips.is_empty() or rig_clips.has(clip):
		return clip
	if carrying and rig_clips.has(str(cfg["haul"]["walk_clip"])):
		return str(cfg["haul"]["walk_clip"])
	return "idle"


func _finish_props(camera: Variant, hide: float) -> void:
	props_shown = 0
	var by_object: Dictionary = {}
	for e in _props_frame:
		var xform: Transform3D = e["xform"]
		if camera != null and (camera as Vector3).distance_to(xform.origin) > hide:
			continue
		var object_name := str(e["object"])
		if not by_object.has(object_name):
			by_object[object_name] = []
		(by_object[object_name] as Array).append(e)
	var objects: Dictionary = cfg.get("haul", {}).get("objects", {})
	for object_name in objects:
		var list: Array = by_object.get(object_name, [])
		var prop := _prop_layer(str(object_name), objects[object_name])
		var mm: MultiMesh = (prop["mmi"] as MultiMeshInstance3D).multimesh
		if mm.instance_count < list.size():
			mm.instance_count = maxi(list.size(), 2 * mm.instance_count)
		for i in list.size():
			var xform: Transform3D = list[i]["xform"]
			if bool(list[i]["ground"]):
				xform.origin.y += float(prop["rest_y"])
			mm.set_instance_transform(i, xform)
		mm.visible_instance_count = list.size()
		props_shown += list.size()


func _prop_layer(object_name: String, spec: Dictionary) -> Dictionary:
	if _props.has(object_name):
		return _props[object_name]
	var mesh: PrimitiveMesh
	var radius := float(spec["radius"])
	var rest_y := radius
	if str(spec["shape"]) == "cylinder":
		var cyl := CylinderMesh.new()
		cyl.top_radius = radius
		cyl.bottom_radius = radius
		cyl.height = float(spec.get("height", radius * 2.0))
		cyl.radial_segments = 10
		cyl.rings = 1
		mesh = cyl
		rest_y = cyl.height * 0.5
	else:
		var sph := SphereMesh.new()
		sph.radius = radius
		sph.height = radius * 2.0 * float(spec.get("y_scale", 1.0))
		sph.radial_segments = 10
		sph.rings = 5
		mesh = sph
		rest_y = sph.height * 0.5
	var mat := StandardMaterial3D.new()
	mat.albedo_color = Color(str(spec["color"]))
	mat.roughness = 0.95
	mesh.material = mat
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.mesh = mesh
	mm.instance_count = 8
	var mmi := MultiMeshInstance3D.new()
	mmi.name = "Prop_%s" % object_name
	mmi.multimesh = mm
	add_child(mmi)
	var prop := {"mmi": mmi, "rest_y": rest_y}
	_props[object_name] = prop
	return prop


func _layer(layer_key: String, kind: String, figure: int, clip: String, side: String) -> Dictionary:
	if _layers.has(layer_key):
		return _layers[layer_key]
	var mat: ShaderMaterial
	if soldiers != null:
		mat = soldiers._make_skinned_material(side, kind, figure, false)
	else:
		mat = ShaderMaterial.new()
		mat.shader = BattleSkinned.SHADER
		BattleSkinned.setup_material(mat, kind, figure)
	var rig := BattleSkinned.rig(kind, figure)
	var config := {"key": "%s/%d/%s" % [kind, figure, clip], "set": [BattleSkinned.clip_index(rig, clip)], "mode": BattleSkinned.M_CUSTOM, "speed": 1.0, "cycle": 1.0, "release": 1.0}
	BattleSkinned.apply_config(mat, config, time_now)
	# NT10 : fondu depuis le geste précédent, lu depuis son propre début (INSTANCE_CUSTOM.z).
	mat.set_shader_parameter("custom_fade", 2)
	mat.set_shader_parameter("role_blend", BattleSkinned.role_blend_s())
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.use_custom_data = true
	mm.mesh = BattleSkinned.mesh(kind, figure, 0)
	var mmi := MultiMeshInstance3D.new()
	mmi.name = "Crew_%s" % layer_key.replace("/", "_")
	mmi.multimesh = mm
	mmi.material_override = mat
	add_child(mmi)
	var layer := {"mmi": mmi, "mat": mat, "level": 0}
	_layers[layer_key] = layer
	return layer
