class_name SiegeCrewFx
extends Node3D

## SG3 — servants des engins de siège : figurines skinnées V2 (`crew_0` sans arme, `crew_1` au
## refouloir ; `tools/blender_scripts/battle_skinned_figures.py`) qui manœuvrent l'engin au fil du
## rechargement exposé par le cœur (`SiegeEnginesFx` choisit le clip de chaque servant : treuil
## `crank`, chargement `load`, écouvillon `swab`, poussée `push`, corde `haul`, repos `idle`).
## Un MultiMesh par (figurine, clip, camp) en mode CUSTOM du shader skinné : INSTANCE_CUSTOM.x =
## instant où le servant a commencé son geste (le clip boucle depuis cet instant). Chaque servant
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


## Clip d'un rôle (`clips` des réglages), ou le nom tel quel.
func clip_of(role: String) -> String:
	return str(cfg.get("clips", {}).get(role, role))


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
			# Nouveau geste : le clip repart de son début (léger décalage entre servants).
			st = {"clip": str(e["clip"]), "since": time_now - 0.13 * float(int(_slots[key]) % 5)}
			_state[key] = st
		e["since"] = float(st["since"])
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
			var values := [b.x.x, b.y.x, b.z.x, t.origin.x, b.x.y, b.y.y, b.z.y, t.origin.y, b.x.z, b.y.z, b.z.z, t.origin.z, float(e["since"]), 0.0, 0.0, 0.0]
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
