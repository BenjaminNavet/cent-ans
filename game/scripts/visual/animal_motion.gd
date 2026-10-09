class_name AnimalMotion
extends RefCounted

## Lot AS1 (ADR 0188) : mouvement procédural des bêtes, en déplacement de sommets. Rendu seulement.
## - Campagne : bœufs, vaches, moutons, chevaux de trait et bêtes attelées des charrettes
##   (`folk_prop.gdshader` + `animal_motion.gdshaderinc`) ; `apply_prop` pose les mesures d'un
##   modèle sur le matériau d'une surface.
## - Bataille : chevaux au piquet des camps (`camp_horse.gdshader`), un `MultiMesh` par modèle.
## Réglages : `data/fx/animal_motion.json` (schéma `fx_animal_motion.schema.json`) ; éteint par
## `enabled`.

const FX_PATH := "fx/animal_motion.json"
const CAMP_FX_PATH := "fx/camp_horse_motion.json"  # AS8c : valeurs tirées de vidéos CC BY-SA, fichier propre
const CAMP_SHADER := preload("res://shaders/camp_horse.gdshader")

static var _motion := JsonLookup.new(FX_PATH)
static var _camp_motion := JsonLookup.new(CAMP_FX_PATH)
static var _settings: Dictionary = {}
static var _camp_meshes: Dictionary = {}


## Réglages fusionnés (mouvement + chevaux de camp), construits une fois.
static func settings() -> Dictionary:
	if _settings.is_empty():
		_settings = _motion.data().duplicate(true)
		_merge_camp_horse_motion(_camp_motion.data())
	return _settings


## Ajoute les réglages des chevaux de camp et le mâchonnement du cheval de la campagne, lus dans
## leur fichier à part (`camp_horse_motion.json`, licence propre).
static func _merge_camp_horse_motion(camp: Dictionary) -> void:
	if camp.is_empty():
		return
	_settings["camp_horse"] = camp.get("camp_horse", {})
	var models: Variant = (_settings.get("campaign", {}) as Dictionary).get("models", null)
	if models is Dictionary and (models as Dictionary).get("horse") is Dictionary:
		(models["horse"] as Dictionary).merge(camp.get("campaign_horse", {}), true)


static func enabled() -> bool:
	return bool(settings().get("enabled", false))


## Réglages fusionnés d'un modèle de la carte : `defaults` < `base` < entrée du modèle ; {} si
## le modèle n'est pas décrit.
static func model_params(model: String) -> Dictionary:
	var campaign: Dictionary = settings().get("campaign", {})
	var models: Dictionary = campaign.get("models", {})
	if not models.has(model):
		return {}
	var out: Dictionary = (campaign.get("defaults", {}) as Dictionary).duplicate()
	var entry: Dictionary = models[model]
	var base := str(entry.get("base", ""))
	if base != "" and models.has(base):
		out.merge(models[base] as Dictionary, true)
	out.merge(entry, true)
	out.erase("base")
	return out


## Pose les uniformes de `animal_motion.gdshaderinc` sur `material` (modèle `model`). Sans effet
## (bêtes immobiles, roues fixes) si le lot est éteint ou le modèle absent des données.
static func apply_prop(material: ShaderMaterial, model: String) -> void:
	var p := model_params(model)
	if p.is_empty() or not enabled():
		material.set_shader_parameter("am_on", 0.0)
		material.set_shader_parameter("am_cart", Vector4(0.0, 0.65, 1.0, 1.26))
		return
	var has_beast := float(p.get("half_m", 0.0)) > 0.0
	material.set_shader_parameter("am_on", 1.0 if has_beast else 0.0)
	var anchor := float(p["anchor_m"])
	material.set_shader_parameter("am_body", Vector4(anchor, float(p.get("half_m", 0.0)), float(p.get("belly_m", 0.5)), float(p.get("withers_m", 1.0))))
	material.set_shader_parameter("am_neck", Vector4(float(p.get("neck_start_m", 0.5)), float(p.get("neck_len_m", 0.3)), float(p.get("neck_pivot_m", 1.0)), float(p["tail_len_m"])))
	material.set_shader_parameter("am_gait", Vector4(float(p["stride_m"]), float(p["swing_m"]), float(p["lift_m"]), float(p["bob_m"])))
	material.set_shader_parameter("am_head", Vector4(float(p["graze_rad"]), float(p["nod_rad"]), float(p["graze_cycle_s"]), float(p["graze_share"])))
	material.set_shader_parameter("am_tail", Vector4(float(p["tail_swing_m"]), float(p["tail_hz"]), float(p["tail_walk_gain"]), float(p["shift_m"])))
	material.set_shader_parameter("am_breath", Vector4(float(p["breath_m"]), float(p["breath_hz"]), float(p["chew_hz"]), float(p["chew_rad"])))
	# Le cahot suit la foulée de la bête attelée ; sans bête, la période des données.
	var period := float(p["stride_m"]) if has_beast else float(p["jolt_period_m"])
	material.set_shader_parameter("am_cart", Vector4(float(p.get("wheel_radius_m", 0.0)), float(p.get("wheel_x_min_m", 0.65)), 1.0, period))
	# Lot AS8c : courbes de pas, cahot et roulis mesurés (data/fx/animal_motion_measured.json).
	material.set_shader_parameter("am_extra", Vector4(float(p["graze_ramp_s"]), float(p["roll_rad"]), 0.0, 0.0))
	material.set_shader_parameter("am_swing", _lut(p["swing_lut"]))
	material.set_shader_parameter("am_lift", _lut(p["lift_lut"]))
	var amps := Vector3.ZERO
	var ratios := Vector3.ONE
	var lines: Array = p["jolt_lines"]
	for k in mini(lines.size(), 3):
		amps[k] = float(lines[k]["amp_m"])
		ratios[k] = float(lines[k]["stride_ratio"])
	material.set_shader_parameter("am_jolt_amp", amps)
	material.set_shader_parameter("am_jolt_ratio", ratios)


## Valeur d'une table de pas (16 échantillons bouclés) à la phase `u` en cycles : même
## interpolation que `am_swing_at` du shader (utilisée par les tests).
static func lut_at(table: Array, u: float) -> float:
	var x := fposmod(u, 1.0) * float(table.size())
	var i := int(floor(x))
	return lerpf(float(table[i % table.size()]), float(table[(i + 1) % table.size()]), x - float(i))


static func _lut(values: Array) -> PackedFloat32Array:
	var out := PackedFloat32Array()
	for v in values:
		out.append(float(v))
	return out


## Matériau d'un cheval de camp pour une surface du modèle du kit (couleur et rugosité de
## l'original, `hair` : crins et queue).
static func camp_horse_material(source: Material, hair: bool) -> ShaderMaterial:
	var material := ShaderMaterial.new()
	material.shader = CAMP_SHADER
	var base := source as BaseMaterial3D
	if base != null:
		material.set_shader_parameter("albedo", base.albedo_color)
		material.set_shader_parameter("rough", base.roughness)
	material.set_shader_parameter("ch_hair", 1.0 if hair else 0.0)
	var s: Dictionary = settings().get("camp_horse", {})
	material.set_shader_parameter("ch_on", 1.0 if enabled() else 0.0)
	material.set_shader_parameter("ch_head", Vector4(_f(s, "head_pivot_x_m"), _f(s, "head_pivot_y_m"), _f(s, "head_start_x_m"), _f(s, "head_end_x_m")))
	material.set_shader_parameter("ch_head2", Vector4(_f(s, "head_lift_rad"), _f(s, "head_cycle_s"), _f(s, "head_up_share"), _f(s, "head_nod_rad")))
	material.set_shader_parameter("ch_head3", Vector4(_f(s, "head_nod_hz"), _f(s, "chew_hz"), _f(s, "chew_rad"), _f(s, "head_ramp_s")))
	material.set_shader_parameter("ch_tail", Vector4(_f(s, "tail_start_x_m"), _f(s, "tail_end_x_m"), _f(s, "tail_top_y_m"), maxf(_f(s, "tail_len_m"), 0.05)))
	material.set_shader_parameter("ch_tail2", Vector4(_f(s, "tail_swing_m"), _f(s, "tail_hz"), maxf(_f(s, "tail_swish_period_s"), 1.0), 0.0))
	material.set_shader_parameter("ch_shift", Vector4(maxf(_f(s, "shift_period_s"), 1.0), _f(s, "shift_lift_m"), _f(s, "shift_back_m"), _f(s, "shift_sway_m")))
	material.set_shader_parameter("ch_breath", Vector4(_f(s, "breath_m"), _f(s, "breath_side"), _f(s, "breath_hz"), 0.0))
	return material


## Chevaux animés d'un camp à la place du `BuildingKit.Batch` : `transforms` = nom de modèle
## (`horse_N`) → Array[Transform3D]. Renvoie les `MultiMeshInstance3D` créés (aucun si le lot est
## éteint : l'appelant pose alors les chevaux comme avant).
static func build_camp_horses(parent: Node3D, transforms: Dictionary, visibility_end: float) -> Array[MultiMeshInstance3D]:
	var out: Array[MultiMeshInstance3D] = []
	for model_name in transforms:
		var mesh := camp_horse_mesh(str(model_name))
		var list: Array = transforms[model_name]
		if mesh == null or list.is_empty():
			continue
		var mm := MultiMesh.new()
		mm.transform_format = MultiMesh.TRANSFORM_3D
		mm.mesh = mesh
		mm.instance_count = list.size()
		for i in list.size():
			mm.set_instance_transform(i, list[i])
		var mmi := MultiMeshInstance3D.new()
		mmi.name = "Kit_%s_alive" % model_name
		mmi.multimesh = mm
		if visibility_end > 0.0:
			mmi.visibility_range_end = visibility_end
		parent.add_child(mmi)
		out.append(mmi)
	return out


## Maillage animé d'un cheval du kit (une surface par matière d'origine), mis en cache.
static func camp_horse_mesh(model_name: String) -> Mesh:
	if _camp_meshes.has(model_name):
		return _camp_meshes[model_name]
	var source := BuildingKit.source_mesh(model_name)
	var mesh: Mesh = null
	if source != null:
		mesh = source.duplicate() as Mesh
		for i in mesh.get_surface_count():
			var original := mesh.surface_get_material(i)
			var label := original.resource_name if original != null else ""
			mesh.surface_set_material(i, camp_horse_material(original, label.begins_with("Hair")))
	_camp_meshes[model_name] = mesh
	return mesh


static func _f(d: Dictionary, key: String) -> float:
	return float(d.get(key, 0.0))

