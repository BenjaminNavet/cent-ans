class_name BattleStandards
extends Node3D

## Vent de la bataille et porte-étendards (lot BV3), rendu seulement.
## - Vent : direction tirée de la graine de la bataille, force et rafales selon la météo
##   (`data/fx/battle_finish.json`, `wind`) ; partagé par les drapeaux-repères, les étendards
##   portés et l'herbe. Aucune règle n'en dépend.
## - Étendards portés : un drapeau à l'échelle 1 (hampe de 4,2 m, étoffe du régiment) tenu par
##   une figurine de la formation (`bearer_rank` dans le tampon des figurines), qui avance, charge
##   et se bat avec elle ; celui du général est plus grand (étendard royal quand il existe).
##   Affichés en deçà de `max_distance_m` ; au plus près, le drapeau-repère (V4) s'efface devant
##   eux (`BattleScene`).

const SETTINGS_FILE := "fx/battle_finish.json"
const BANNER_SHADER := preload("res://shaders/battle_banner.gdshader")

static var _settings: Dictionary = {}

var wind_dir: Vector2 = Vector2(1, 0)
var wind_strength: float = 0.5
var wind_gust: float = 0.2
var shown_count: int = 0

var _cfg: Dictionary = {}
var _standards: Dictionary = {}  # unit id -> {node, flag_mat, mounted, visible}


static func settings() -> Dictionary:
	if _settings.is_empty():
		_settings = read_data(SETTINGS_FILE)
	return _settings


## Lit un fichier JSON de `data/` (dossier de données du jeu, puis `data/` du dépôt), comme
## `BattleGore.settings()`.
static func read_data(relative: String) -> Dictionary:
	var candidates: Array[String] = []
	var tree := Engine.get_main_loop() as SceneTree
	var paths: Node = tree.root.get_node_or_null("/root/MapPaths") if tree != null else null
	if paths != null:
		candidates.append(str(paths.get("data_dir")))
	candidates.append(ProjectSettings.globalize_path("res://").path_join("../data").simplify_path())
	for dir in candidates:
		var path := dir.path_join(relative)
		if FileAccess.file_exists(path):
			var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
			if parsed is Dictionary:
				return parsed
	push_warning("BattleStandards: %s introuvable" % relative)
	return {}


## Vent de la bataille : direction déterministe (graine), force selon la météo.
static func wind_for(weather: String, seed_value: int) -> Dictionary:
	var wind: Dictionary = settings().get("wind", {})
	var entry: Dictionary = (wind.get("by_weather", {}) as Dictionary).get(weather, wind.get("default", {}))
	var angle := float(absi(seed_value * 2654435761) % 3600) / 3600.0 * TAU
	return {"dir": Vector2(cos(angle), sin(angle)), "strength": float(entry.get("strength", 0.5)), "gust": float(entry.get("gust", 0.2)), "grass_scale": float(wind.get("grass_strength_scale", 1.8))}


## Cap (rotation Y) qui couche un drapeau (étoffe le long de +X local) dans le sens du vent.
func downwind_yaw() -> float:
	return atan2(-wind_dir.y, wind_dir.x)


## Applique le vent au matériau d'un drapeau (`battle_banner.gdshader`).
func apply_wind(mat: ShaderMaterial) -> void:
	mat.set_shader_parameter("wind_strength", wind_strength)
	mat.set_shader_parameter("wind_gust", wind_gust)


## `cloth_of(unit)` → {texture, size, full} (étoffe du drapeau-repère du régiment).
func setup(units: Array, side_colors: Dictionary, cloth_of: Callable, wind: Dictionary) -> void:
	_cfg = settings().get("standards", {})
	wind_dir = wind["dir"]
	wind_strength = float(wind["strength"])
	wind_gust = float(wind["gust"])
	for unit in units:
		var render := str(unit.get("render", ""))
		if render == "siege":
			continue
		var id := int(unit["id"])
		var general := bool(unit.get("is_general", false))
		var cloth: Dictionary = cloth_of.call(unit)
		var pole_m := float(_cfg.get("general_pole_m" if general else "pole_m", 4.2))
		var scale := float(_cfg.get("general_cloth_scale" if general else "cloth_scale", 0.55))
		var size: Vector2 = (cloth["size"] as Vector2) * scale
		var node := Node3D.new()
		node.name = "Standard%d" % id
		node.visible = false
		add_child(node)
		var pole := MeshInstance3D.new()
		pole.mesh = BattleMeshes.pole()
		pole.scale = Vector3(1, pole_m, 1)
		pole.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		node.add_child(pole)
		var mat := ShaderMaterial.new()
		mat.shader = BANNER_SHADER
		mat.set_shader_parameter("livery", side_colors.get(str(unit["side"]), Color(0.5, 0.5, 0.5)))
		mat.set_shader_parameter("phase", float(id) * 2.3 + 0.7)
		mat.set_shader_parameter("heraldry", cloth["texture"])
		mat.set_shader_parameter("has_heraldry", cloth["texture"] != null)
		mat.set_shader_parameter("full_texture", cloth["full"])
		mat.set_shader_parameter("flag_length", size.x)
		apply_wind(mat)
		var flag := MeshInstance3D.new()
		flag.mesh = BattleMeshes.flag(size.x, size.y)
		flag.material_override = mat
		flag.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON
		flag.position = Vector3(0.03, pole_m - 0.05, 0)
		node.add_child(flag)
		_standards[id] = {"node": node, "flag_mat": mat, "mounted": render == "cavalry", "routing": false}


## Place les étendards sur leur porteur (figurine du tampon courant), dans le vent.
func update(units: Array, soldiers: BattleSoldiers, camera_pos: Vector3) -> void:
	var max_d := float(_cfg.get("max_distance_m", 220.0))
	var rank := float(_cfg.get("bearer_rank", 0.5))
	var lift := float(_cfg.get("mounted_lift_m", 1.2))
	var yaw := downwind_yaw()
	shown_count = 0
	for unit in units:
		var id := int(unit["id"])
		if not _standards.has(id):
			continue
		var rec: Dictionary = _standards[id]
		var node: Node3D = rec["node"]
		var center := Vector3(float(unit["x"]), float(unit.get("y", 0.0)), float(unit["z"]))
		var show := bool(unit["present"]) and camera_pos.distance_to(center) < max_d
		var bearer: Variant = soldiers.figure_frame(id, rank) if show else null
		if bearer == null:
			node.visible = false
			continue
		var frame: Transform3D = bearer
		# Hampe tenue à droite du porteur, un peu en avant ; plus haut à cheval.
		var right := frame.basis.x.normalized()
		var fwd := frame.basis.z.normalized()
		node.position = frame.origin + right * 0.32 + fwd * 0.15 + Vector3(0, lift if bool(rec["mounted"]) else 0.35, 0)
		node.rotation = Vector3(0, yaw, 0)
		node.visible = true
		shown_count += 1
		var routing := str(unit.get("state", "")) == "routing"
		if routing != bool(rec["routing"]):
			rec["routing"] = routing
			(rec["flag_mat"] as ShaderMaterial).set_shader_parameter("routing", routing)


## Échelle du drapeau-repère en deçà de laquelle il s'efface devant l'étendard porté.
func hide_scale() -> float:
	return float(_cfg.get("marker_hide_scale", 1.05))


func is_shown(id: int) -> bool:
	return _standards.has(id) and ((_standards[id] as Dictionary)["node"] as Node3D).visible
