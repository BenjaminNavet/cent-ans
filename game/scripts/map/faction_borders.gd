class_name FactionBorders
extends Node

## Lot FR1 (ADR 0070) : frontières de faction lumineuses de la carte de campagne, façon Total War.
##
## Rendu dans le fragment du terrain (`faction_borders.gdshaderinc`, crochet `fr1_borders` de
## terrain.gdshader et terrain_parchment.gdshader) : les frontières sont peintes sur le relief
## lui-même, donc drapées à tous les zooms (morceaux E0 et patchs du quadtree ZG2), sans maillage
## ni passe en plus. Ce nœud pose les uniformes `fr1_*` sur le matériau partagé du terrain :
## - deux petites textures construites depuis la simulation (propriétaire, contrôleur et drapeau
##   joueur par province ; couleur par faction) à chaque `refresh` (appelé par `refresh_all`) :
##   un changement de propriétaire se voit à l'image suivante, sans reconstruction de géométrie ;
## - opacité et largeur selon le zoom, le filtre de carte (MF1) et la qualité (PF1) ;
## - respiration du halo du joueur (un uniforme par image).
## Réglages : `data/map/faction_borders.json` (schéma `faction_borders.schema.json`).
## Option (après `--`) : `--no-faction-borders` (A/B de perf). Purement visuel.

const MAP_PATHS_SCRIPT := preload("res://scripts/map/map_paths.gd")
const TUNING_PATH := "map/faction_borders.json"
const PALETTE_SIZE := 256

var map: Node = null  # CampaignMap (non typé : tests et maquettes)
var terrain: TerrainBuilder = null
var tuning: Dictionary = {}
## Interrupteur (A/B, qualité) ; faux : `fr1_alpha` = 0, le crochet sort au premier test.
var enabled: bool = true
## Mode de carte courant (MF1) et opacité effective (mode × zoom), exposés pour les tests.
var mode: String = "political"
var effective_alpha: float = 0.0
var quality_level: String = "high"

var _quality_on: bool = true
var _faction_index: Dictionary = {}  # id de faction → index de palette (1..255)
var _info_image: Image
var _info_texture: ImageTexture
var _palette_texture: ImageTexture
var _owners: PackedStringArray = PackedStringArray()  # par index raster - 1
var _controllers: PackedStringArray = PackedStringArray()
var _player: String = ""
var _band_saved: Variant = null
var _last_distance: float = -1.0
var _last_mode: String = ""
var _cli_disabled: bool = false


func setup(campaign_map: Node, terrain_builder: TerrainBuilder = null) -> void:
	map = campaign_map
	terrain = terrain_builder if terrain_builder != null else campaign_map.get("terrain") as TerrainBuilder
	_cli_disabled = OS.get_cmdline_user_args().has("--no-faction-borders")
	tuning = load_tuning()
	_apply_tuning()
	add_to_group(RenderQuality.CLIENT_GROUP)
	apply_render_quality(RenderQuality.preset())
	_apply_enabled()


## Réglages de `data/map/faction_borders.json` (dictionnaire vide si absent ou illisible).
static func load_tuning() -> Dictionary:
	var path := MAP_PATHS_SCRIPT.default_data_dir().path_join(TUNING_PATH)
	if not FileAccess.file_exists(path):
		push_warning("FactionBorders: %s missing" % path)
		return {}
	var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
	return parsed if parsed is Dictionary else {}


## Matériau partagé du terrain (porte les uniformes `fr1_*`), ou null.
func material() -> ShaderMaterial:
	return terrain.material if terrain != null else null


func _set_param(param: String, value: Variant) -> void:
	var target := material()
	if target != null:
		target.set_shader_parameter(param, value)


func get_param(param: String) -> Variant:
	var target := material()
	return target.get_shader_parameter(param) if target != null else null


## Vrai si le shader courant du terrain porte le crochet FR1 (terrain 3D ou parchemin seul).
func has_hook() -> bool:
	var target := material()
	if target == null or target.shader == null:
		return false
	for entry in target.shader.get_shader_uniform_list():
		if str(entry["name"]) == "fr1_alpha":
			return true
	return false


func set_enabled(value: bool) -> void:
	enabled = value
	_apply_enabled()


func _active() -> bool:
	return enabled and _quality_on and not _cli_disabled


func _apply_enabled() -> void:
	var band: Variant = tuning.get("terrain_realm_band_alpha", null)
	if band != null and material() != null:
		if _band_saved == null:
			_band_saved = get_param("realm_band_alpha")
			if _band_saved == null:
				_band_saved = RenderingServer.shader_get_parameter_default(TerrainBuilder.TERRAIN_SHADER.get_rid(), "realm_band_alpha")
		if _active():
			_set_param("realm_band_alpha", float(band))
		elif _band_saved != null:
			_set_param("realm_band_alpha", _band_saved)
	_update_alpha()


## PF1 : qualité (halo et hachures coupés en Basse ; tout coupé si `enabled` faux).
func apply_render_quality(_preset: Dictionary) -> void:
	quality_level = RenderQuality.current()
	var levels: Dictionary = tuning.get("quality", {})
	var q: Dictionary = levels.get(quality_level, levels.get("high", {}))
	_set_param("fr1_glow", bool(q.get("glow", true)))
	_set_param("fr1_hatch", bool(q.get("hatch", true)))
	var on := bool(q.get("enabled", true))
	if on != _quality_on:
		_quality_on = on
		_apply_enabled()


func _apply_tuning() -> void:
	var sections := {
		"realm": ["core_px", "core_alpha", "core_brightness", "outline_px", "outline_alpha", "glow_px", "glow_alpha", "glow_exponent"],
		"province": ["core_px", "core_alpha", "core_brightness"],
		"player": ["width_scale", "glow_scale", "brightness"],
		"parchment": ["width_scale", "alpha_scale", "glow_scale", "ink_mix"],
	}
	for section: String in sections:
		var values: Dictionary = tuning.get(section, {})
		for key: String in sections[section]:
			if values.has(key):
				_set_param("fr1_%s_%s" % [section, key], float(values[key]))
	var occupied: Dictionary = tuning.get("occupied", {})
	for key: String in ["period_px", "duty", "band_px", "alpha"]:
		var source := "hatch_" + key if key in ["period_px", "duty"] else key
		if occupied.has(source):
			_set_param("fr1_hatch_" + key, float(occupied[source]))
	if tuning.has("neutral_color"):
		var ink := Color.html(str(tuning["neutral_color"]))
		_set_param("fr1_neutral_color", Vector3(pow(ink.r, 2.2), pow(ink.g, 2.2), pow(ink.b, 2.2)))
	if tuning.has("emission"):
		_set_param("fr1_emission", float(tuning["emission"]))


## Propriétaires et contrôleurs depuis la simulation ; textures reconstruites seulement si
## quelque chose a changé. Renvoie vrai si les frontières ont changé.
func refresh() -> bool:
	if map == null or terrain == null or terrain.map_data == null:
		return false
	var sim: Object = map.get("sim")
	var data: MapData = terrain.map_data
	var owners := PackedStringArray()
	var controllers := PackedStringArray()
	owners.resize(data.province_count)
	controllers.resize(data.province_count)
	for index in range(1, data.province_count + 1):
		var province: Dictionary = data.get_province(index)
		var owner := str(province.get("owner", ""))
		var controller := owner
		if sim != null and sim.has_method("get_province_state"):
			var state: Dictionary = sim.call("get_province_state", str(province.get("id", "")))
			owner = str(state.get("owner", owner))
			controller = str(state.get("controller", owner))
			if controller == "":
				controller = owner
		owners[index - 1] = owner
		controllers[index - 1] = controller
	return set_ownership(owners, controllers, str(map.get("player_faction")))


## Pose directement propriétaires et contrôleurs (par index raster - 1) : `refresh` et tests.
func set_ownership(owners: PackedStringArray, controllers: PackedStringArray, player: String) -> bool:
	if owners == _owners and controllers == _controllers and player == _player and _info_texture != null:
		return false
	_owners = owners.duplicate()  # copies : l'appelant peut modifier ses tableaux ensuite
	_controllers = controllers.duplicate()
	_player = player
	var palette_dirty := _palette_texture == null
	for list in [owners, controllers]:
		for faction: String in list:
			if faction != "" and not _faction_index.has(faction) and _faction_index.size() < PALETTE_SIZE - 1:
				_faction_index[faction] = _faction_index.size() + 1
				palette_dirty = true
	if palette_dirty:
		_build_palette()
	var width := maxi(owners.size() + 1, 1)
	if _info_image == null or _info_image.get_width() != width:
		_info_image = Image.create(width, 1, false, Image.FORMAT_RGBA8)
	_info_image.fill(Color(0, 0, 0, 0))
	for i in owners.size():
		var o := int(_faction_index.get(owners[i], 0))
		var c := int(_faction_index.get(controllers[i], o)) if i < controllers.size() else o
		var mine := player != "" and (owners[i] == player or (i < controllers.size() and controllers[i] == player))
		_info_image.set_pixel(i + 1, 0, Color8(o, c, 255 if mine else 0, 255))
	if _info_texture == null or _info_texture.get_width() != width:
		_info_texture = ImageTexture.create_from_image(_info_image)
	else:
		_info_texture.update(_info_image)
	_set_param("fr1_info", _info_texture)
	return true


func _build_palette() -> void:
	var image := Image.create(PALETTE_SIZE, 1, false, Image.FORMAT_RGBA8)
	image.fill(Color(0, 0, 0, 0))
	for faction: String in _faction_index:
		image.set_pixel(int(_faction_index[faction]), 0, faction_color(faction))
	_palette_texture = ImageTexture.create_from_image(image)
	_set_param("fr1_palette", _palette_texture)


## Couleur héraldique (SimFacade), palette de repli du terrain sans store.
func faction_color(faction: String) -> Color:
	var facade: Node = get_node_or_null("/root/SimFacade") if is_inside_tree() else null
	if facade != null and bool(facade.call("store_loaded")):
		var color: Color = facade.call("faction_color", faction)
		color.a = 1.0
		return color
	var index := int(_faction_index.get(faction, 1)) - 1
	return TerrainBuilder.FALLBACK_PALETTE[index % TerrainBuilder.FALLBACK_PALETTE.size()]


## Index de palette du propriétaire et du contrôleur d'une province (index raster), pour les tests.
func ownership_of(province_index: int) -> Vector2i:
	if _info_image == null or province_index <= 0 or province_index >= _info_image.get_width():
		return Vector2i.ZERO
	var c := _info_image.get_pixel(province_index, 0)
	return Vector2i(roundi(c.r * 255.0), roundi(c.g * 255.0))


func faction_index(faction: String) -> int:
	return int(_faction_index.get(faction, 0))


## Opacité [0, 1] et encre neutre d'un filtre de carte (MF1).
func mode_style(map_mode: String) -> Dictionary:
	var modes: Dictionary = tuning.get("modes", {})
	var style: Dictionary = modes.get(map_mode, modes.get("political", {"alpha": 1.0, "neutral": false}))
	return {"alpha": float(style.get("alpha", 1.0)), "neutral": bool(style.get("neutral", false))}


## Fondu de zoom : 0 sous `fade_out_m` (distance caméra en mètres), 1 au-delà de `fade_in_m`.
func zoom_alpha(distance: float) -> float:
	var zoom: Dictionary = tuning.get("zoom", {})
	var mpp := terrain.map_data.meters_per_px if terrain != null and terrain.map_data != null else 719.0
	return smoothstep(float(zoom.get("fade_out_m", 200.0)), float(zoom.get("fade_in_m", 1400.0)), distance * mpp)


## Chaque image (CampaignMap._process) : distance caméra (unités carte).
func update_view(distance: float) -> void:
	if material() == null or not _active():
		return
	var player: Dictionary = tuning.get("player", {})
	var amount := float(player.get("pulse_amount", 0.0))
	if amount > 0.0:
		var period := maxf(float(player.get("pulse_period_s", 3.0)), 0.1)
		_set_param("fr1_pulse", 1.0 + amount * sin(Time.get_ticks_msec() * 0.001 * TAU / period))
	var modes: Node = map.get("map_modes") if map != null else null
	var current := str(modes.get("mode")) if modes != null else "political"
	if is_equal_approx(distance, _last_distance) and current == _last_mode:
		return
	_last_distance = distance
	_last_mode = current
	var zoom: Dictionary = tuning.get("zoom", {})
	var far := float(zoom.get("far_distance", 900.0))
	_set_param("fr1_width_scale", lerpf(1.0, float(zoom.get("far_width_scale", 1.0)), smoothstep(far * 0.4, far, distance)))
	set_mode(current)


func set_mode(map_mode: String) -> void:
	mode = map_mode
	_set_param("fr1_neutral", bool(mode_style(mode)["neutral"]))
	_update_alpha()


func _update_alpha() -> void:
	var alpha := float(mode_style(mode)["alpha"])
	if _last_distance >= 0.0:
		alpha *= zoom_alpha(_last_distance)
	effective_alpha = alpha if _active() else 0.0
	_set_param("fr1_alpha", effective_alpha)


## Visible : crochet présent dans le shader du terrain et opacité effective non nulle.
func visible_now() -> bool:
	var alpha: Variant = get_param("fr1_alpha")
	return has_hook() and effective_alpha > 0.001 and alpha != null and float(alpha) > 0.001
