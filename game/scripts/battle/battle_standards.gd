class_name BattleStandards
extends Node3D

## Vent de la bataille, porte-étendards et musiciens (lots BV3 puis EP5, ADR 0034), rendu seulement.
## - Vent : direction tirée de la graine de la bataille, force et rafales selon la météo
##   (`data/fx/battle_finish.json`, `wind`) ; partagé par les drapeaux-repères, les étendards
##   portés et l'herbe. Aucune règle n'en dépend.
## - EP5 : chaque régiment a une figurine dédiée qui porte son étendard (`standard_0` à pied,
##   `standard_1` à cheval), au rang du tampon que donne le cœur (`bearer_slots` : premier rang
##   au centre, rang du milieu pour les tireurs ; deux porte-étendards pour les grands
##   régiments). La figurine ordinaire de ce rang est masquée (`BattleSoldiers.reserved`).
##   Forme et étoffe de l'étendard selon l'unité (`data/fx/battle_standards.json` : pennon,
##   bannière, étendard long, bannière royale, oriflamme de Saint-Denis quand le roi de France
##   est présent). Tambours et busines (`musician_0`, `musician_1`) à côté du porte-étendard.
## - État venu du cœur (`standard`) : porté ; tombé (le porte-étendard meurt, l'étendard gît à
##   `standard_x/z` jusqu'à ce qu'il soit relevé) ; pris (porté bas, étoffe renversée, par une
##   figurine du régiment vainqueur) ; perdu (reste au sol).
## - Coût : aucun nœud par drapeau. Figurines par (camp, figurine, niveau de détail) et étoffes
##   par (rig, état) en MultiMesh, rebâtis chaque image depuis le tampon des soldats ; étoffes
##   dans un Texture2DArray ; le drapeau lit la hampe animée dans la texture d'os (shader
##   `battle_standard_flag.gdshader`) et grandit au loin (lisible jusqu'à ~600 m).

const SETTINGS_FILE := "fx/battle_finish.json"
const FX_FILE := "fx/battle_standards.json"
const BANNER_SHADER := preload("res://shaders/battle_banner.gdshader")
const FLAG_SHADER := preload("res://shaders/battle_standard_flag.gdshader")
const BANNERS_DIR := "res://assets/heraldry/banners/"
const LAYER_SIZE := 256
const MAX_LAYERS := 64
const TRIM_GOLD := Color(0.83, 0.66, 0.24)
const TRIM_SILVER := Color(0.85, 0.85, 0.82)
## Jeux de clips (mode CUSTOM : INSTANCE_CUSTOM.y choisit le clip) : arrêt, marche, course,
## mêlée / sonnerie.
const SETS := {
	"standard": ["std_idle", "std_walk", "std_run", "std_wave"],
	"standard_mounted": ["c_std_idle", "c_std_walk", "c_std_gallop", "c_std_wave"],
	"drum": ["drum_idle", "drum_march", "run", "drum_beat"],
	"horn": ["horn_idle", "horn_walk", "run", "horn_blow"],
}
const DEATH_SETS := {"standard": ["std_death"], "standard_mounted": ["c_std_death"]}
## Figurine (famille, variante) par rôle.
const FIGURES := {
	"standard": ["standard", 0],
	"standard_mounted": ["standard", 1],
	"drum": ["musician", 0],
	"horn": ["musician", 1],
}
## État de l'étoffe (partie entière de INSTANCE_CUSTOM.w du shader).
const FLAG_CARRIED := 0.0
const FLAG_CAPTURED := 2.0
const FLAG_GROUND := 3.0

static var _settings: Dictionary = {}
static var _fx_cache: Dictionary = {}

var wind_dir: Vector2 = Vector2(1, 0)
var wind_strength: float = 0.5
var wind_gust: float = 0.2
## Étendards portés affichés à la dernière image (tests, banc).
var shown_count: int = 0
## Figurines dédiées affichées à la dernière image (porte-étendards, musiciens, porteurs de trophées).
var figure_count: int = 0
## Étendards à terre affichés à la dernière image.
var fallen_count: int = 0

var _cfg: Dictionary = {}
var _fx: Dictionary = {}
var _render: Dictionary = {}
var _records: Dictionary = {}  # unit id -> fiche du régiment (cf. `_record`)
var _shown: Dictionary = {}  # unit id -> true : étendard porté affiché
var _side_colors: Dictionary = {}
var _side_factions: Dictionary = {}
var _side_houses: Dictionary = {}  # DA1b : id de la maison du général par camp
var _battle: Object = null
var _cloth_of: Callable
var _layer_keys: Dictionary = {}
var _layer_images: Array[Image] = []
var _cell_info: Array[Vector4] = []
var _cloths: Texture2DArray = null
var _groups: Dictionary = {}  # "side/role" -> {lods: [MultiMeshInstance3D ×3], rows: [[], [], []]}
var _dead_groups: Dictionary = {}  # "side/role" -> {mmi, rows}
var _flag_layers: Dictionary = {}  # "carried/human" ... -> {mmi, rows}
var _fallen: Array = []  # porte-étendards tombés : {id, xform, instant, layer, role, side, flag}
var _no_quarter_poll: float = 0.0
var _audio: Script = null
var _anim_time: float = 0.0


static func settings() -> Dictionary:
	if _settings.is_empty():
		_settings = read_data(SETTINGS_FILE)
	return _settings


## Réglages de rendu des étendards d'EP5 (`data/fx/battle_standards.json`).
static func fx() -> Dictionary:
	if _fx_cache.is_empty():
		_fx_cache = read_data(FX_FILE)
	return _fx_cache


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


## Applique le vent au matériau d'un drapeau (`battle_banner.gdshader` ou étoffe d'EP5).
func apply_wind(mat: ShaderMaterial) -> void:
	mat.set_shader_parameter("wind_strength", wind_strength)
	mat.set_shader_parameter("wind_gust", wind_gust)


## `cloth_of(unit)` → {texture, size, full} : étoffe de repli (drapeau-repère du régiment)
## quand les étoffes d'EP5 manquent. `side_factions` : faction de chaque camp ; `battle` : la
## simulation (ordre « pas de quartier » du général : oriflamme, dragon).
## `side_houses` (DA1b) : maison du général de chaque camp (nom ou id de `houses.json`) ; son
## étendard et ceux des unités nobles de sa retenue portent ses armes.
func setup(units: Array, side_colors: Dictionary, cloth_of: Callable, wind: Dictionary, side_factions: Dictionary = {}, battle: Object = null, side_houses: Dictionary = {}) -> void:
	_cfg = settings().get("standards", {})
	_fx = fx()
	_render = _fx.get("render", {})
	wind_dir = wind["dir"]
	wind_strength = float(wind["strength"])
	wind_gust = float(wind["gust"])
	_side_colors = side_colors
	_side_factions = side_factions
	for side in side_houses:
		_side_houses[side] = HouseArms.id_of(str(side_houses[side]))
	_battle = battle
	_cloth_of = cloth_of
	if ResourceLoader.exists("res://scripts/audio/battle_audio.gd"):
		_audio = load("res://scripts/audio/battle_audio.gd")
	for unit in units:
		if str(unit.get("render", "")) == "siege" or bool(unit.get("synthetic", false)):
			continue
		_records[int(unit["id"])] = _record(unit)
	_build_cloths()
	for key in ["carried/human", "carried/cavalry", "ground/human", "ground/cavalry"]:
		_flag_layer(key)


## Fiche d'un régiment : rôle de ses porte-étendards, couches d'étoffe, musiciens.
func _record(unit: Dictionary) -> Dictionary:
	var side := str(unit["side"])
	var faction := str(_side_factions.get(side, ""))
	var mounted := str(unit.get("render", "")) == "cavalry"
	var general := bool(unit.get("is_general", false))
	var id := int(unit["id"])
	var bearer_role := "standard_mounted" if mounted else "standard"
	var rec := {
		"id": id,
		"side": side,
		"faction": faction,
		"type": str(unit.get("type", "")),
		"mounted": mounted,
		"general": general,
		"role": bearer_role,
		"dedicated": _has_role(bearer_role),
		"layers": _bearer_layers(unit, faction, str(_side_houses.get(side, ""))),
		"musicians": [] if mounted else _instruments(unit),
		"phase": fmod(float(id) * 0.618, 1.0) * 9.0,
		"standard": "carried",
		"state": "",
		"drum_at": -100.0,
	}
	if general:
		# « Pas de quartier » donné en cours de bataille : étoffe prête d'avance.
		rec["layer_no_quarter"] = _layer_for_kind(unit, faction, _kind_of(unit, faction, true))
	return rec


## Couches des deux porte-étendards d'un régiment. DA1b : le général porte la bannière de sa
## maison ; son second porte-étendard garde l'étendard de l'armée (bannière royale, saint
## Georges, oriflamme du roi en personne). « Pas de quartier » remplace toujours la première
## (`layer_no_quarter`). Les unités nobles de la retenue (`house_arms.unit_types`) portent les
## armes de la maison du général ; le commun garde celles de la faction.
func _bearer_layers(unit: Dictionary, faction: String, house: String) -> Array:
	var kind := _kind_of(unit, faction, false)
	var second := str(_fx.get("second_bearer_kind", "pennon"))
	if house != "" and bool(unit.get("is_general", false)):
		var house_kind := str((_fx.get("house_arms", {}) as Dictionary).get("general_kind", "royal"))
		return [_layer_for_kind(unit, faction, house_kind, house), _layer_for_kind(unit, faction, kind)]
	if house != "" and is_house_retinue(unit):
		return [_layer_for_kind(unit, faction, kind, house), _layer_for_kind(unit, faction, second)]
	return [_layer_for_kind(unit, faction, kind), _layer_for_kind(unit, faction, second)]


## Unité noble de la retenue du général (DA1b) : ses étendards portent les armes de sa maison.
static func is_house_retinue(unit: Dictionary) -> bool:
	var types: Array = (fx().get("house_arms", {}) as Dictionary).get("unit_types", [])
	return str(unit.get("type", "")) in types


func _has_role(role: String) -> bool:
	var figure: Array = FIGURES[role]
	return BattleSkinned.has_figure(str(figure[0]), int(figure[1]))


func _instruments(unit: Dictionary) -> Array:
	var musicians: Dictionary = _fx.get("musicians", {})
	var list: Array = musicians.get("general", []) if bool(unit.get("is_general", false)) else (musicians.get("by_unit_type", {}) as Dictionary).get(str(unit.get("type", "")), [])
	var out: Array = []
	for instrument in list:
		if _has_role(str(instrument)):
			out.append(str(instrument))
	return out


## Genre d'étendard d'un régiment (données) ; `no_quarter` : celui du général après l'ordre.
func _kind_of(unit: Dictionary, faction: String, no_quarter: bool) -> String:
	if bool(unit.get("is_general", false)):
		var g: Dictionary = _fx.get("general", {})
		if no_quarter and (g.get("no_quarter_by_faction", {}) as Dictionary).has(faction):
			return str(g["no_quarter_by_faction"][faction])
		if bool(unit.get("sovereign", false)) and (g.get("sovereign_by_faction", {}) as Dictionary).has(faction):
			return str(g["sovereign_by_faction"][faction])
		if (g.get("by_faction", {}) as Dictionary).has(faction):
			return str(g["by_faction"][faction])
		return str(g.get("kind", "royal"))
	return str((_fx.get("by_unit_type", {}) as Dictionary).get(str(unit.get("type", "")), _fx.get("default_kind", "pennon")))


## Couche d'étoffe (Texture2DArray) d'un genre d'étendard pour la faction ; repli sur la
## bannière ou le pennon de la faction, puis sur l'étoffe du drapeau-repère.
func _layer_for_kind(unit: Dictionary, faction: String, kind: String, house: String = "") -> int:
	var kinds: Dictionary = _fx.get("kinds", {})
	var entry: Dictionary = kinds.get(kind, kinds.get("pennon", {}))
	var size := Vector2(1.9, 0.5)
	var dims: Array = entry.get("size_m", [])
	if dims.size() == 2:
		size = Vector2(float(dims[0]), float(dims[1]))
	if bool(unit.get("is_general", false)):
		size *= float(_render.get("general_scale", 1.25))
	var cloth := str(entry.get("cloth", "pennon"))
	var paths: Array[String] = []
	if cloth in ["pennon", "banner", "standard"]:
		if house != "":
			# DA1b : étoffe aux armes de la maison du général, repli sur celle de la faction.
			paths.append(BANNERS_DIR + "houses/%s_%s.png" % [house, cloth])
			paths.append(BANNERS_DIR + "houses/%s_banner.png" % house)
		paths.append(BANNERS_DIR + "%s_%s.png" % [faction, cloth])
		paths.append(BANNERS_DIR + "%s_banner.png" % faction)
	else:
		paths.append(BANNERS_DIR + "%s.png" % cloth)
	for path in paths:
		var texture := PortraitLoader.load_texture(path)
		if texture != null:
			return _layer(path, texture, size, true)
	if _cloth_of.is_valid():
		var fallback: Dictionary = _cloth_of.call(unit)
		var tex: Texture2D = fallback.get("texture")
		if tex != null:
			return _layer(tex.resource_path + str(tex.get_instance_id()), tex, (fallback["size"] as Vector2) * 0.55, bool(fallback.get("full", false)))
	return _layer("livery", null, size, false)


func _layer(key_path: String, texture: Texture2D, size: Vector2, full: bool) -> int:
	var key := "%s|%.2f|%.2f|%s" % [key_path, size.x, size.y, full]
	if _layer_keys.has(key):
		return int(_layer_keys[key])
	if _layer_images.size() >= MAX_LAYERS:
		return 0
	var image: Image = null
	if texture != null:
		image = texture.get_image()
	if image == null or image.is_empty():
		image = Image.create(LAYER_SIZE, LAYER_SIZE, false, Image.FORMAT_RGBA8)
		image.fill(Color(1, 1, 1, 0))
		full = false
	else:
		image = image.duplicate()
		if image.is_compressed():
			image.decompress()
		image.clear_mipmaps()
		image.convert(Image.FORMAT_RGBA8)
		if not full:
			# Écu de la faction : son centre (comme `battle_banner.gdshader`).
			var w := image.get_width()
			var h := image.get_height()
			image = image.get_region(Rect2i(int(w * 0.2), int(h * 0.12), maxi(int(w * 0.6), 1), maxi(int(h * 0.46), 1)))
		image.resize(LAYER_SIZE, LAYER_SIZE, Image.INTERPOLATE_LANCZOS)
	image.generate_mipmaps()
	var index := _layer_images.size()
	_layer_images.append(image)
	_cell_info.append(Vector4(size.x, size.y, 1.0 if full else 0.0, 0.0))
	_layer_keys[key] = index
	return index


func _build_cloths() -> void:
	if _layer_images.is_empty():
		_layer("livery", null, Vector2(1.9, 0.5), false)
	_cloths = Texture2DArray.new()
	var err := _cloths.create_from_images(_layer_images)
	if err != OK:
		push_warning("BattleStandards: Texture2DArray %s" % error_string(err))
	while _cell_info.size() < MAX_LAYERS:
		_cell_info.append(Vector4(1.9, 0.5, 0.0, 0.0))


# --- Couches instanciées --------------------------------------------------------------------


func _new_mmi(mesh: Mesh, mat: Material, shadows: bool) -> MultiMeshInstance3D:
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.use_custom_data = true
	mm.mesh = mesh
	mm.instance_count = 0
	var mmi := MultiMeshInstance3D.new()
	mmi.multimesh = mm
	mmi.material_override = mat
	mmi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON if shadows else GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	add_child(mmi)
	return mmi


## Matériau skinné d'une figurine dédiée (livrée du camp, clips en mode CUSTOM).
func _figure_material(side: String, role: String, clip_names: Array) -> ShaderMaterial:
	var figure: Array = FIGURES[role]
	var kind := str(figure[0])
	var variant := int(figure[1])
	var mat := ShaderMaterial.new()
	mat.shader = BattleSkinned.SHADER
	var color: Color = _side_colors.get(side, Color(0.5, 0.5, 0.5))
	mat.set_shader_parameter("livery", color)
	mat.set_shader_parameter("trim", TRIM_SILVER if color.get_luminance() > 0.55 or (color.r > 0.6 and color.g > 0.5) else TRIM_GOLD)
	var arms := PortraitLoader.heraldry_texture(str(_side_factions.get(side, "")))
	mat.set_shader_parameter("heraldry", arms)
	mat.set_shader_parameter("has_heraldry", arms != null)
	BattleSkinned.setup_material(mat, kind, variant)
	mat.set_shader_parameter("size_jitter", 0.0)
	mat.set_shader_parameter("livery_share", 0.95 if BattleSkinned.is_noble(kind, variant) else 0.75)
	BattleSkinned.apply_config(mat, _custom_config(kind, variant, clip_names), 0.0)
	# AN1a : surcot et caparaçon au vent (pas pour le porte-étendard tombé).
	BattleSecondaryMotion.setup_material(mat, kind, variant)
	var dead := not clip_names.is_empty() and str(clip_names[0]).ends_with("death")
	if dead:
		mat.set_shader_parameter("sm_enabled", false)
	# NT10 : fondu au changement de clip (INSTANCE_CUSTOM.y empaqueté, clip précédent même phase).
	mat.set_shader_parameter("custom_fade", 0 if dead else 1)
	return mat


## NT10 : indices globaux (`clips[]` du rig) du jeu de clips d'un rôle ; vide sans figurine skinnée.
static var _role_ids_cache: Dictionary = {}


static func role_ids(role: String) -> Array:
	if _role_ids_cache.has(role):
		return _role_ids_cache[role]
	var figure: Array = FIGURES[role]
	var kind := str(figure[0])
	var variant := int(figure[1])
	var ids: Array = []
	if BattleSkinned.has_figure(kind, variant):
		var rig := BattleSkinned.rig(kind, variant)
		for c in role_set(role):
			ids.append(BattleSkinned.clip_index(rig, str(c)))
	_role_ids_cache[role] = ids
	return ids


## NT10 : INSTANCE_CUSTOM.y d'une figurine de rôle jouant l'emplacement `slot` de son jeu, avec le
## clip précédent en fondu (`state` : dictionnaire propre à la figurine).
func _fade_y(role: String, slot: int, state: Dictionary) -> float:
	var ids := role_ids(role)
	if ids.is_empty() or slot < 0 or slot >= ids.size():
		return float(slot)
	var prev := BattleSkinned.fade_prev(state, int(ids[slot]), _anim_time)
	return BattleSkinned.pack_fade(slot, prev, float(state.get("at", 0.0)))


func _custom_config(kind: String, variant: int, clip_names: Array) -> Dictionary:
	var rig := BattleSkinned.rig(kind, variant)
	var ids: Array[int] = []
	for c in clip_names:
		ids.append(BattleSkinned.clip_index(rig, str(c)))
	return {"key": "ep5/%s/%d/%s" % [kind, variant, ",".join(clip_names)], "set": ids, "mode": BattleSkinned.M_CUSTOM, "speed": 1.0, "cycle": 1.0, "release": 1.0}


func _group(side: String, role: String) -> Dictionary:
	var key := side + "/" + role
	if _groups.has(key):
		return _groups[key]
	var figure: Array = FIGURES[role]
	var mat := _figure_material(side, role, role_set(role))
	var lods: Array = []
	for level in 3:
		var mmi := _new_mmi(BattleSkinned.mesh(str(figure[0]), int(figure[1]), level), mat, level < 2)
		mmi.name = "EP5_%s_%s_lod%d" % [side, role, level]
		mmi.set_meta("far_blend", [0.0, 0.1, 0.7][level])
		lods.append(mmi)
	var group := {"lods": lods, "rows": [[], [], []], "mat": mat}
	_groups[key] = group
	return group


func _dead_group(side: String, role: String) -> Dictionary:
	var key := side + "/" + role
	if _dead_groups.has(key):
		return _dead_groups[key]
	var figure: Array = FIGURES[role]
	var mat := _figure_material(side, role, DEATH_SETS[role])
	var mmi := _new_mmi(BattleSkinned.mesh(str(figure[0]), int(figure[1]), 1), mat, true)
	mmi.name = "EP5_%s_%s_fallen" % [side, role]
	var group := {"mmi": mmi, "rows": [], "mat": mat}
	_dead_groups[key] = group
	return group


## Couche d'étoffes `state/rig` (`carried` ou `ground`, `human` ou `cavalry`).
func _flag_layer(key: String) -> Dictionary:
	if _flag_layers.has(key):
		return _flag_layers[key]
	var parts := key.split("/")
	var ground := parts[0] == "ground"
	var mounted := parts[1] == "cavalry"
	var role := "standard_mounted" if mounted else "standard"
	var figure: Array = FIGURES[role]
	var kind := str(figure[0])
	var variant := int(figure[1])
	var mat := ShaderMaterial.new()
	mat.shader = FLAG_SHADER
	var skinned := BattleSkinned.has_figure(kind, variant)
	mat.set_shader_parameter("skinned", skinned)
	if skinned:
		BattleSkinned.setup_material(mat, kind, variant)
		var config := _custom_config(kind, variant, DEATH_SETS[role] if ground else role_set(role))
		mat.set_shader_parameter("clip_set", BattleSkinned._ivec(config["set"]))
		mat.set_shader_parameter("clip_set_size", (config["set"] as Array).size())
		var bones: Array = BattleSkinned.rig(kind, variant).get("bones", [])
		mat.set_shader_parameter("prop_bone", maxi(bones.find("R:Prop" if mounted else "Prop"), 0))
		var fig := BattleSkinned.figure(kind, variant)
		var top: Array = fig.get("pole_top", [0.0, 4.0, 0.0])
		var axis: Array = fig.get("pole_axis", [0.0, 1.0, 0.0])
		mat.set_shader_parameter("pole_top", Vector3(float(top[0]), float(top[1]), float(top[2])))
		mat.set_shader_parameter("pole_axis", Vector3(float(axis[0]), float(axis[1]), float(axis[2])))
	else:
		# Repli : hampe fixe tenue à droite d'une figurine ordinaire (comme BV3).
		var lift := float(_cfg.get("mounted_lift_m", 1.2)) if mounted else 0.35
		mat.set_shader_parameter("pole_top", Vector3(0.32, float(_cfg.get("pole_m", 4.2)) + lift, 0.15))
		mat.set_shader_parameter("pole_axis", Vector3.UP)
	mat.set_shader_parameter("cloths", _cloths)
	mat.set_shader_parameter("cell_info", _cell_info)
	mat.set_shader_parameter("wind_dir", wind_dir)
	mat.set_shader_parameter("far_scale", float(_render.get("far_scale", 2.2)))
	mat.set_shader_parameter("far_from", float(_render.get("far_scale_from_m", 140.0)))
	mat.set_shader_parameter("far_to", float(_render.get("far_scale_to_m", 600.0)))
	apply_wind(mat)
	BattleSecondaryMotion.setup_flag(mat)  # AN1a : onde et vent apparent de l'allure
	var mesh := BattleMeshes.flag(1.0, 1.0)
	mesh.custom_aabb = AABB(Vector3(-10, -1, -10), Vector3(20, 20, 20))
	var mmi := _new_mmi(mesh, mat, true)
	mmi.name = "EP5_flags_%s_%s" % [parts[0], parts[1]]
	var layer := {"mmi": mmi, "rows": [], "mat": mat, "skinned": skinned}
	_flag_layers[key] = layer
	return layer


static func _append(rows: Array, t: Transform3D, custom: Vector4) -> void:
	var b := t.basis
	rows.append_array(([b.x.x, b.y.x, b.z.x, t.origin.x, b.x.y, b.y.y, b.z.y, t.origin.y, b.x.z, b.y.z, b.z.z, t.origin.z, custom.x, custom.y, custom.z, custom.w]))


static func _flush(mmi: MultiMeshInstance3D, rows: Array) -> void:
	var mm := mmi.multimesh
	var n := rows.size() / 16
	if n > mm.instance_count:
		mm.instance_count = maxi(n, mm.instance_count * 2)
	mmi.visible = n > 0
	if n > 0:
		var padded := PackedFloat32Array(rows)
		padded.resize(mm.instance_count * 16)
		mm.buffer = padded
	mm.visible_instance_count = n


# --- Image ------------------------------------------------------------------------------------


## NT7 : états qui ont un clip propre par rôle (`data/fx/battle_animation.json`, `role_clips`),
## ajoutés après les quatre clips de `SETS` dans cet ordre quand le rig les a.
const ROLE_EXTRAS := ["charging", "victory", "idle_alt"]
static var _role_sets: Dictionary = {}  # rôle -> {names: [...], extra: {état: indice}}


## NT7 : jeu de clips d'un rôle (quatre clips EP5 + clips propres présents dans le rig).
static func role_set(role: String) -> Array:
	return _role_entry(role)["names"]


static func _role_entry(role: String) -> Dictionary:
	if _role_sets.has(role):
		return _role_sets[role]
	var names: Array = (SETS[role] as Array).duplicate()
	var extra := {}
	var figure: Array = FIGURES[role]
	var clips: Dictionary = {}
	if BattleSkinned.has_figure(str(figure[0]), int(figure[1])):
		clips = BattleSkinned.rig(str(figure[0]), int(figure[1])).get("clips", {})
	var wanted: Dictionary = (BattleSkinned.animation_settings().get("role_clips", {}) as Dictionary).get(role, {})
	for state in ROLE_EXTRAS:
		var clip := str(wanted.get(state, ""))
		if clip == "" or not clips.has(clip):
			continue
		var at := names.find(clip)
		if at < 0 and names.size() < BattleSkinned.MAX_SET:
			names.append(clip)
			at = names.size() - 1
		if at >= 0:
			extra[state] = at
	var entry := {"names": names, "extra": extra}
	_role_sets[role] = entry
	return entry


## Clip (indice dans le jeu du rôle) selon l'état du régiment. NT7 : clips propres de charge
## (`charging`), de victoire (`victory`, état de rendu du camp vainqueur) et variante d'attente
## (`idle_alt`, un porteur sur deux selon `phase`) quand le rig les a ; sinon jeu EP5.
static func clip_for(role: String, state: String, running: bool, phase: float = 0.0) -> int:
	var extra: Dictionary = _role_entry(role)["extra"]
	match state:
		"marching":
			return 2 if running else 1
		"charging":
			if extra.has("charging"):
				return int(extra["charging"])
			return 3 if role == "horn" or role == "drum" else 2
		"melee":
			return 3
		"routing":
			return 2
		"victory":
			return int(extra.get("victory", 3 if role == "horn" or role == "drum" else 0))
	if extra.has("idle_alt") and fmod(absf(phase) * 7.31, 1.0) < 0.5:
		return int(extra["idle_alt"])
	return 0


## Place porte-étendards, musiciens, étoffes et étendards à terre ; `soldiers` donne les
## figurines du tampon courant (et reçoit les rangs à masquer).
func update(units: Array, soldiers: BattleSoldiers, camera_pos: Vector3) -> void:
	_anim_time = soldiers.anim_time
	var max_d := float(_render.get("max_distance_m", 650.0))
	var lod0 := float(_render.get("lod0_distance_m", 24.0))
	var lod1 := float(_render.get("lod1_distance_m", 75.0))
	var trophy_offset := int(_render.get("trophy_slot_offset", 2))
	shown_count = 0
	figure_count = 0
	_shown.clear()
	for group in _groups.values():
		for k in 3:
			group["rows"][k] = []
	for key in _flag_layers:
		(_flag_layers[key] as Dictionary)["rows"] = []
	_poll_no_quarter(units)
	var by_id: Dictionary = {}
	for unit in units:
		by_id[int(unit["id"])] = unit
	# Trophées : étendards pris, portés par le régiment vainqueur.
	var trophies: Dictionary = {}  # captor id -> [couches]
	for unit in units:
		var id := int(unit["id"])
		if not _records.has(id):
			continue
		var rec: Dictionary = _records[id]
		var standard := str(unit.get("standard", "carried"))
		_track_standard(rec, unit, standard)
		if standard == "captured" and int(unit.get("standard_by", -1)) >= 0:
			var captor := int(unit["standard_by"])
			var list: Array = trophies.get(captor, [])
			list.append(int((rec["layers"] as Array)[0]))
			trophies[captor] = list
	for unit in units:
		var id := int(unit["id"])
		if not _records.has(id):
			continue
		var rec: Dictionary = _records[id]
		var reserved := PackedInt32Array()
		if bool(unit.get("present", false)) and camera_pos.distance_to(Vector3(float(unit["x"]), float(unit.get("y", 0.0)), float(unit["z"]))) < max_d:
			reserved = _place_unit(unit, rec, soldiers, camera_pos, lod0, lod1, trophies.get(id, []), trophy_offset)
		if reserved.is_empty():
			soldiers.reserved.erase(id)
		else:
			soldiers.reserved[id] = reserved
	_place_fallen(camera_pos, max_d)
	for group in _groups.values():
		for k in 3:
			var mmi: MultiMeshInstance3D = group["lods"][k]
			_flush(mmi, group["rows"][k])
		(group["mat"] as ShaderMaterial).set_shader_parameter("anim_time", _anim_time)
	for group in _dead_groups.values():
		_flush(group["mmi"], group["rows"])
		(group["mat"] as ShaderMaterial).set_shader_parameter("anim_time", _anim_time)
	for key in _flag_layers:
		var layer: Dictionary = _flag_layers[key]
		_flush(layer["mmi"], layer["rows"])
		(layer["mat"] as ShaderMaterial).set_shader_parameter("anim_time", _anim_time)


## Figurines dédiées d'un régiment présent ; renvoie les rangs du tampon qu'elles remplacent.
func _place_unit(unit: Dictionary, rec: Dictionary, soldiers: BattleSoldiers, camera_pos: Vector3, lod0: float, lod1: float, trophy_layers: Array, trophy_offset: int) -> PackedInt32Array:
	var id := int(unit["id"])
	var reserved := PackedInt32Array()
	var n := soldiers.figure_count(id)
	if n <= 0:
		return reserved
	var slots: Array = []
	if unit.has("bearer_slots"):
		for s in unit["bearer_slots"]:
			slots.append(int(s))
	else:
		slots.append(clampi(int(float(_cfg.get("bearer_rank", 0.5)) * float(n - 1)), 0, n - 1))
	if slots.is_empty():
		return reserved
	var state := str(unit.get("state", "idle"))
	# NT7 : acclamation du camp vainqueur (même règle de rendu que BattleSoldiers).
	if soldiers.victor_side != "" and str(unit.get("side", "")) == soldiers.victor_side and state != "routing" and state != "climbing":
		state = "victory"
	var running := bool(unit.get("running", false))
	var standard := str(unit.get("standard", "carried"))
	var role := str(rec["role"])
	var dedicated := bool(rec["dedicated"])
	var clip := clip_for(role, state, running, float(rec["phase"]))
	if not rec.has("fade"):
		rec["fade"] = {}
	var clip_y := _fade_y(role, clip, rec["fade"])
	var phase := float(rec["phase"])
	var rig := "cavalry" if bool(rec["mounted"]) else "human"
	var layers: Array = rec["layers"]
	var main_layer := int(rec.get("layer_now", layers[0]))
	var carried: Array = []  # [rang, couche, état de l'étoffe]
	for b in slots.size():
		if b == 0 and standard != "carried":
			continue
		carried.append([int(slots[b]), main_layer if b == 0 else int(layers[1]), FLAG_CARRIED])
	for k in trophy_layers.size():
		carried.append([int(slots[0]) + trophy_offset + k, int(trophy_layers[k]), FLAG_CAPTURED])
	for entry in carried:
		var slot := int(entry[0])
		if slot < 0 or slot >= n:
			continue
		var frame: Variant = soldiers.figure_at(id, slot)
		if frame == null:
			continue
		var xform: Transform3D = frame
		var custom := Vector4(phase, clip_y, float(entry[1]), float(entry[2]) + fmod(phase * 0.13, 1.0))
		if dedicated:
			var group := _group(str(rec["side"]), role)
			var d := camera_pos.distance_to(xform.origin)
			var level := 0 if d < lod0 else (1 if d < lod1 else 2)
			_append(group["rows"][level], xform, custom)
			reserved.append(slot)
			figure_count += 1
		_append((_flag_layer("carried/" + rig) as Dictionary)["rows"], xform, custom)
		if float(entry[2]) == FLAG_CARRIED and slot == int(slots[0]):
			shown_count += 1
			_shown[id] = true
	# Musiciens de part et d'autre du porte-étendard (à pied seulement).
	var musicians: Array = rec["musicians"]
	for k in musicians.size():
		var slot := int(slots[0]) + (-1 if k == 0 else 1)
		if slot < 0 or slot >= n or reserved.has(slot):
			continue
		var frame: Variant = soldiers.figure_at(id, slot)
		if frame == null:
			continue
		var xform: Transform3D = frame
		var instrument := str(musicians[k])
		var group := _group(str(rec["side"]), instrument)
		var d := camera_pos.distance_to(xform.origin)
		var level := 0 if d < lod0 else (1 if d < lod1 else 2)
		if not rec.has("fade_mus"):
			rec["fade_mus"] = [{}, {}]
		var mus_clip := clip_for(instrument, state, running, float(rec["phase"]) + float(k) * 0.37)
		var mus_y := _fade_y(instrument, mus_clip, (rec["fade_mus"] as Array)[mini(k, 1)])
		_append(group["rows"][level], xform, Vector4(phase + float(k) * 0.37, mus_y, 0.0, 0.0))
		reserved.append(slot)
		figure_count += 1
		_sound(rec, instrument, state, xform.origin, camera_pos)
	rec["state"] = state
	return reserved


## Suit l'état de l'étendard : chute (porte-étendard mort à l'endroit donné par le cœur),
## relève (busine), prise.
func _track_standard(rec: Dictionary, unit: Dictionary, standard: String) -> void:
	var previous := str(rec["standard"])
	if standard == previous:
		return
	rec["standard"] = standard
	var id := int(rec["id"])
	if (standard == "fallen" or standard == "lost") and previous == "carried":
		var pos := Vector3(float(unit.get("standard_x", unit["x"])), float(unit.get("standard_y", unit.get("y", 0.0))), float(unit.get("standard_z", unit["z"])))
		var yaw := float(unit.get("facing", 0.0))
		_fallen.append({"id": id, "xform": Transform3D(Basis(Vector3.UP, yaw), pos), "instant": _anim_time, "layer": int(rec.get("layer_now", (rec["layers"] as Array)[0])), "role": rec["role"], "side": rec["side"], "flag": true, "dedicated": rec["dedicated"]})
		var cap := int(_render.get("max_fallen_bearers", 96))
		while _fallen.size() > cap:
			_fallen.pop_front()
	elif standard == "carried" or standard == "captured":
		# Relevé ou pris : l'étoffe quitte le sol (le mort reste).
		for entry in _fallen:
			if int(entry["id"]) == id:
				entry["flag"] = false
		if standard == "carried" and previous == "fallen":
			_play("horn", Vector3(float(unit["x"]), float(unit.get("y", 0.0)), float(unit["z"])))
		elif standard == "captured":
			_play("horn", Vector3(float(unit.get("standard_x", unit["x"])), float(unit.get("standard_y", unit.get("y", 0.0))), float(unit.get("standard_z", unit["z"]))))


func _place_fallen(camera_pos: Vector3, max_d: float) -> void:
	fallen_count = 0
	for group in _dead_groups.values():
		group["rows"] = []
	for entry in _fallen:
		var xform: Transform3D = entry["xform"]
		if camera_pos.distance_to(xform.origin) > max_d:
			continue
		var role := str(entry["role"])
		var rig := "cavalry" if role == "standard_mounted" else "human"
		var custom := Vector4(float(entry["instant"]), 0.0, float(entry["layer"]), FLAG_GROUND + 0.5)
		if bool(entry["dedicated"]):
			var group := _dead_group(str(entry["side"]), role)
			# NT10 : le corps n'hérite pas de la couche d'étoffe (z) ni de l'état du drapeau (w),
			# lus par le shader skinné comme projection et membre tranché ; un peu de sang seul.
			_append(group["rows"], xform, Vector4(float(entry["instant"]), 0.0, 0.0, 0.5))
		if bool(entry["flag"]):
			_append((_flag_layer("ground/" + rig) as Dictionary)["rows"], xform, custom)
			fallen_count += 1


## « Pas de quartier » : le général lève l'oriflamme (France) ou le dragon (Angleterre).
func _poll_no_quarter(units: Array) -> void:
	_no_quarter_poll -= 1.0 / 30.0
	if _no_quarter_poll > 0.0 or _battle == null or not _battle.has_method("get_no_quarter"):
		return
	_no_quarter_poll = 1.0
	for unit in units:
		var id := int(unit["id"])
		if not _records.has(id) or not bool(unit.get("is_general", false)):
			continue
		var rec: Dictionary = _records[id]
		var no_quarter := bool(_battle.call("get_no_quarter", str(unit["side"])))
		rec["layer_now"] = int(rec["layer_no_quarter"]) if no_quarter and rec.has("layer_no_quarter") else int((rec["layers"] as Array)[0])


## Sons des musiciens : tambour qui bat la marche, busine qui sonne la charge.
func _sound(rec: Dictionary, instrument: String, state: String, pos: Vector3, camera_pos: Vector3) -> void:
	var musicians: Dictionary = _fx.get("musicians", {})
	if camera_pos.distance_to(pos) > float(musicians.get("sound_distance_m", 260.0)):
		return
	var previous := str(rec["state"])
	if instrument == "drum":
		var every := float(musicians.get("drum_every_s", 6.0))
		if (state == "marching" or state == "charging") and _anim_time - float(rec["drum_at"]) >= every:
			rec["drum_at"] = _anim_time
			_play("drum", pos)
	elif instrument == "horn" and state == "charging" and previous != "charging":
		_play("horn", pos)


func _play(event: String, pos: Vector3) -> void:
	if _audio != null:
		_audio.call("play_at", event, pos)


## Échelle du drapeau-repère en deçà de laquelle il s'efface devant l'étendard porté.
func hide_scale() -> float:
	return float(_cfg.get("marker_hide_scale", 1.05))


func is_shown(id: int) -> bool:
	return _shown.has(id)


## EP5 : le régiment a un étendard rendu ici (porté, tombé ou pris) : de près, son
## drapeau-repère s'efface.
func handles(id: int) -> bool:
	return _records.has(id) and (_shown.has(id) or str((_records[id] as Dictionary)["standard"]) != "carried")
