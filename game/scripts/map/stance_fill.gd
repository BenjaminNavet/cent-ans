class_name StanceFill
extends Node

## Lot RJ-d (ADR 0175) : lavis translucide de chaque province selon la position diplomatique de
## son CONTRÔLEUR envers le joueur — or : nous, vert : alliés et vassaux, rouge : ennemis en
## guerre, gris très léger : les autres. Le lavis suit le contrôle (une cité prise passe à l'or :
## la progression se voit) ; la possession de droit reste dite par le trait de frontière et les
## hachures de l'occupant (FR1), peintes par-dessus le lavis.
##
## Rendu dans le fragment du terrain (`stance_fill.gdshaderinc`, crochet `sf_fill` de
## terrain.gdshader et terrain_parchment.gdshader) : une petite texture par province (index
## raster), refaite seulement quand contrôleurs ou positions changent ; aucune géométrie.
## Classification des positions : `StanceCues` (ADR 0155), non dupliquée ici.
## Réglages : `data/map/stance_fill.json` (schéma `stance_fill.schema.json`).
## Option du joueur : `map/stance_fill` (Réglages › Carte). Option (après `--`) :
## `--no-stance-fill` (A/B de perf). Purement visuel.

const MAP_PATHS_SCRIPT := preload("res://scripts/map/map_paths.gd")
const TUNING_PATH := "map/stance_fill.json"
const SETTING_KEY := "map/stance_fill"

var map: Node = null  # CampaignMap (non typé : tests et maquettes)
var terrain: TerrainBuilder = null
var tuning: Dictionary = {}
## Interrupteur du joueur (Réglages) ; faux : `sf_alpha` = 0, le crochet sort au premier test.
var enabled: bool = true
## Mode de carte courant et opacité globale effective (mode × zoom × option), pour les tests.
var mode: String = "political"
var effective_alpha: float = 0.0

var _cli_disabled: bool = false
var _texture: ImageTexture = null
var _image: Image = null
var _colors := PackedColorArray()
var _last_distance: float = -1.0


func setup(campaign_map: Node, terrain_builder: TerrainBuilder = null) -> void:
	map = campaign_map
	terrain = terrain_builder if terrain_builder != null else campaign_map.get("terrain") as TerrainBuilder
	_cli_disabled = OS.get_cmdline_user_args().has("--no-stance-fill")
	tuning = load_tuning()
	_set_param("sf_saturation", float(tuning.get("saturation", 0.6)))
	_set_param("sf_parchment_scale", float(tuning.get("parchment_scale", 0.55)))
	var settings: Node = get_node_or_null("/root/Settings") if is_inside_tree() else null
	if settings != null:
		enabled = bool(settings.call("get_value", SETTING_KEY))
		settings.connect("changed", func(key: String) -> void:
			if key == SETTING_KEY:
				set_enabled(bool(settings.call("get_value", SETTING_KEY))))
	_update_alpha()


## Réglages de `data/map/stance_fill.json` (dictionnaire vide si absent ou illisible).
static func load_tuning() -> Dictionary:
	var path := MAP_PATHS_SCRIPT.default_data_dir().path_join(TUNING_PATH)
	if not FileAccess.file_exists(path):
		push_warning("StanceFill: %s missing" % path)
		return {}
	var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
	return parsed if parsed is Dictionary else {}


## Couleur de lavis d'une catégorie de position (`StanceCues` : self, enemy, friend, other) :
## rgb sRGB, a = opacité de la catégorie. Nous, ennemis et amis : couleur de frontière de
## `stance_cues.json` (une seule source) ; autres : `colors.other`. Fonction pure (tests).
static func fill_color(cue: String, fill_tuning: Dictionary, cue_tuning: Dictionary) -> Color:
	var alpha := float(fill_tuning.get("alpha", {}).get(cue, 0.0))
	if alpha <= 0.0:
		return Color(0, 0, 0, 0)
	var border: Dictionary = cue_tuning.get("border", {})
	var hex := ""
	if cue != StanceCues.OTHER and border.get(cue) is String:
		hex = str(border[cue])
	else:
		hex = str(fill_tuning.get("colors", {}).get(StanceCues.OTHER, ""))
	if hex == "":
		return Color(0, 0, 0, 0)
	var color := Color.html(hex)
	color.a = alpha
	return color


## Couleurs par province (index raster - 1) d'après les contrôleurs (vide : rien). Fonction pure.
static func colors_for(controllers: PackedStringArray, player: String, stances: Dictionary, fill_tuning: Dictionary, cue_tuning: Dictionary) -> PackedColorArray:
	var by_cue := {}
	for cue in [StanceCues.SELF, StanceCues.ENEMY, StanceCues.FRIEND, StanceCues.OTHER]:
		by_cue[cue] = fill_color(cue, fill_tuning, cue_tuning)
	var colors := PackedColorArray()
	colors.resize(controllers.size())
	for i in controllers.size():
		var controller := controllers[i]
		if controller == "":
			colors[i] = Color(0, 0, 0, 0)
			continue
		var cue := StanceCues.SELF if controller == player else _category(controller, stances, cue_tuning)
		colors[i] = by_cue[cue]
	return colors


## Catégorie d'une faction d'après la table `categories` donnée (même règle que
## `StanceCues.category`, mais sans le cache global : testable avec des réglages fournis).
static func _category(faction: String, stances: Dictionary, cue_tuning: Dictionary) -> String:
	return str(cue_tuning.get("categories", {}).get(str(stances.get(faction, "")), StanceCues.OTHER))


func material() -> ShaderMaterial:
	return terrain.material if terrain != null else null


func _set_param(param: String, value: Variant) -> void:
	var target := material()
	if target != null:
		target.set_shader_parameter(param, value)


func get_param(param: String) -> Variant:
	var target := material()
	return target.get_shader_parameter(param) if target != null else null


## Couleurs posées en dernier (index raster - 1), pour les tests.
func province_colors() -> PackedColorArray:
	return _colors


## Contrôleurs et positions depuis la simulation ; texture refaite seulement si changée.
## Renvoie vrai si le lavis a changé.
func refresh() -> bool:
	if map == null or terrain == null or terrain.map_data == null:
		return false
	var sim: Object = map.get("sim")
	var data: MapData = terrain.map_data
	var controllers := PackedStringArray()
	controllers.resize(data.province_count)
	var snapshot: ProvinceSnapshot = ProvinceSnapshot.of(sim, data) if sim != null else null
	for index in range(1, data.province_count + 1):
		var controller := str(data.get_province(index).get("owner", ""))
		if snapshot != null and snapshot.has(index - 1):
			controller = snapshot.controller[index - 1]
			if controller == "":
				controller = snapshot.owner[index - 1]
		controllers[index - 1] = controller
	var player := str(map.get("player_faction"))
	return set_colors(colors_for(controllers, player, StanceCues.stances(sim, player), tuning, StanceCues.tuning()))


## Pose les couleurs par province (index raster - 1) ; texture mise à jour seulement si changée.
func set_colors(colors: PackedColorArray) -> bool:
	if colors == _colors and _texture != null:
		return false
	_colors = colors.duplicate()
	var width := maxi(colors.size() + 1, 1)
	if _image == null or _image.get_width() != width:
		_image = Image.create(width, 1, false, Image.FORMAT_RGBA8)
	_image.fill(Color(0, 0, 0, 0))
	for i in colors.size():
		_image.set_pixel(i + 1, 0, colors[i])
	if _texture == null or _texture.get_width() != width:
		_texture = ImageTexture.create_from_image(_image)
	else:
		_texture.update(_image)
	_set_param("sf_colors", _texture)
	return true


## Chaque image : distance caméra (unités carte) ; atténuation de près et mode de carte.
func update_view(distance: float) -> void:
	var modes: Node = map.get("map_modes") if map != null else null
	var current := str(modes.get("mode")) if modes != null else "political"
	if is_equal_approx(distance, _last_distance) and current == mode:
		return
	_last_distance = distance
	mode = current
	_update_alpha()


## Facteur d'atténuation de près (1 au-delà de `far_distance`, `near_scale` sous `near_distance`).
func zoom_factor(distance: float) -> float:
	var zoom: Dictionary = tuning.get("zoom", {})
	var near := float(zoom.get("near_distance", 60.0))
	var far := maxf(float(zoom.get("far_distance", 260.0)), near + 1.0)
	return lerpf(float(zoom.get("near_scale", 1.0)), 1.0, smoothstep(near, far, distance))


func set_enabled(value: bool) -> void:
	enabled = value
	_update_alpha()


func _update_alpha() -> void:
	var alpha := 0.0
	if enabled and not _cli_disabled:
		alpha = float(tuning.get("modes", {}).get(mode, 0.0))
		if _last_distance >= 0.0:
			alpha *= zoom_factor(_last_distance)
	if is_equal_approx(alpha, effective_alpha) and _last_distance >= 0.0:
		return
	effective_alpha = alpha
	_set_param("sf_alpha", alpha)
