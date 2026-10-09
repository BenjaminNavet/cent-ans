class_name CampaignAmbience
extends Node

## Ambiances de la carte de campagne, en boucles 2D sur le bus « Ambiance » :
## mer, campagne (oiseaux), grillons, forêt, ville, vent d'altitude, vent d'hiver, pluie.
##
## Toutes les 0,25 s, le terrain sous la caméra est échantillonné (grille 3 × 3 autour du point
## visé, rayon proportionnel au zoom) : part de mer (`land_mask`), de forêt et de champs
## (`splat` : B forêt, R prairie + G cultures), proximité d'une colonie ou d'une capitale.
## `layer_levels` (fonction pure, testée seule) en tire un niveau 0..1 par couche selon le zoom
## (au ras de la carte : les sons du lieu ; en vue haute : le vent d'altitude et la mer au loin),
## la saison (oiseaux au printemps, grillons l'été, vent l'hiver) et la météo.
##
## Météo : celle de la province sous la caméra, tirée par le cœur (
## `CampaignWeatherView.weather_at`, `clear`, `fog`, `rain`, `snow`, `storm`). Sans campagne
## (maquette de simulation), repli sur un tirage déterministe par saison et par date
## (`seasonal_weather`). `weather_override` permet de l'imposer (tests).
##
## Enfant d'`AudioDirector` (toujours actif) : les couches s'éteignent en fondu quand la carte est
## masquée (bataille) ou détachée (menu), et reprennent au retour.

const LAYERS := ["sea", "countryside", "crickets", "forest", "town", "wind", "wind_strong", "rain"]
const UPDATE_PERIOD := 0.25
## Vitesse des fondus (dB par seconde).
const FADE_DB_PER_S := 10.0
const SILENT_DB := -80.0

## Probabilité de pluie (ou de neige l'hiver) par saison.
const RAIN_CHANCE := {"spring": 0.3, "summer": 0.15, "autumn": 0.5, "winter": 0.35}
const BIRDS := {"spring": 1.0, "summer": 0.8, "autumn": 0.45, "winter": 0.12}
const CRICKETS := {"spring": 0.2, "summer": 1.0, "autumn": 0.3, "winter": 0.0}
const WINTER_WIND := {"spring": 0.1, "summer": 0.0, "autumn": 0.35, "winter": 0.8}

var silent: bool = false
var bank: SoundBank = null
## Clé météo imposée (`clear`, `rain`, `snow`, `storm`) ; vide = tirage saisonnier.
var weather_override: String = ""
## Dernier environnement échantillonné et niveaux visés (tests, débogage).
var environment: Dictionary = {}
var levels: Dictionary = {}

var _campaign: Node = null
var _players: Dictionary = {}
var _timer: float = 0.0
var _town_points: PackedVector2Array = PackedVector2Array()


func _ready() -> void:
	process_mode = Node.PROCESS_MODE_ALWAYS
	silent = DisplayServer.get_name() == "headless"
	if bank == null:
		bank = SoundBank.load_default()
	for layer in LAYERS:
		var player := AudioStreamPlayer.new()
		player.name = "Layer_" + layer
		player.bus = AudioBuses.AMBIENCE
		player.stream = bank.ambience_stream(layer)
		player.volume_db = SILENT_DB
		add_child(player)
		_players[layer] = player
		levels[layer] = 0.0


## `campaign` : la carte (`map_data`, `camera_rig`, `sim`) ; null = tout s'éteint.
func setup(campaign: Node) -> void:
	_campaign = campaign
	_town_points = PackedVector2Array()
	if campaign == null:
		silence()
		return
	var map_data: Variant = campaign.get("map_data")
	if map_data == null:
		return
	for province in (map_data.get("provinces") as Dictionary).values():
		var capital: Variant = province.get("capital_px")
		if capital is Vector2:
			_town_points.append(capital)
	var settlements: Variant = map_data.call("_read_json", "settlements_px.json")
	if settlements is Dictionary:
		for entry in (settlements as Dictionary).values():
			if entry is Array and (entry as Array).size() >= 2:
				_town_points.append(Vector2(float(entry[0]), float(entry[1])))
	_timer = 0.0


## Coupe immédiatement les niveaux visés (entrée en bataille) ; les fondus font le reste.
func silence() -> void:
	for layer in LAYERS:
		levels[layer] = 0.0


func _exit_tree() -> void:
	for player in _players.values():
		(player as AudioStreamPlayer).stop()
		(player as AudioStreamPlayer).stream = null


func _process(delta: float) -> void:
	_timer -= delta
	if _timer <= 0.0:
		_timer = UPDATE_PERIOD
		refresh()
	for layer in _players:
		var player: AudioStreamPlayer = _players[layer]
		var level := float(levels.get(layer, 0.0))
		var target := SILENT_DB
		if level > 0.005:
			target = float(bank.ambience.get(layer, {}).get("volume_db", 0.0)) + linear_to_db(level)
		player.volume_db = move_toward(player.volume_db, target, FADE_DB_PER_S * delta)
		if silent or player.stream == null:
			continue
		if player.volume_db > -60.0 and not player.playing:
			player.play(randf() * maxf(player.stream.get_length() - 1.0, 0.0))
		elif player.volume_db <= SILENT_DB + 1.0 and player.playing:
			player.stop()


## Recalcule l'environnement sous la caméra et les niveaux visés.
func refresh() -> void:
	if not _campaign_visible():
		silence()
		return
	var rig: Node = _campaign.get("camera_rig")
	var map_data: Variant = _campaign.get("map_data")
	if rig == null or map_data == null:
		silence()
		return
	var focus: Vector3 = rig.get("focus")
	var distance := float(rig.get("distance"))
	environment = sample_environment(map_data, Vector2(focus.x, focus.z), distance, _town_points)
	var season := current_season()
	environment["season"] = season
	environment["weather"] = weather_override if weather_override != "" else map_weather(Vector2(focus.x, focus.z), season)
	environment["distance"] = distance
	levels = layer_levels(environment)


func _campaign_visible() -> bool:
	if _campaign == null or not is_instance_valid(_campaign) or not _campaign.is_inside_tree():
		return false
	return not (_campaign is Node3D) or (_campaign as Node3D).is_visible_in_tree()


func _date_label() -> String:
	var sim: Variant = _campaign.get("sim") if _campaign != null else null
	if sim is Object and (sim as Object).has_method("get_date_label"):
		return str((sim as Object).call("get_date_label"))
	return ""


func current_season() -> String:
	var season := RichTooltip.season_of(_date_label())
	return season if season != "" else "spring"


## Météo de la carte au point visé (source : le cœur) ; repli saisonnier sans elle.
func map_weather(point: Vector2, season: String) -> String:
	var view: Variant = _campaign.get("weather_view") if _campaign != null else null
	if view is CampaignWeatherView and not (view as CampaignWeatherView).weather.is_empty():
		return (view as CampaignWeatherView).weather_at(point)
	return seasonal_weather(season, _date_label())


## Repli (sans météo du cœur) : tirage déterministe (même date → même temps) : `clear`, `rain` ou `snow` (hiver).
static func seasonal_weather(season: String, date_label: String) -> String:
	if date_label == "":
		return "clear"
	var rng := RandomNumberGenerator.new()
	rng.seed = hash(date_label)
	var roll := rng.randf()
	if roll >= float(RAIN_CHANCE.get(season, 0.2)):
		return "clear"
	return "snow" if season == "winter" else "rain"


## Parts de mer, forêt, champs et ville autour de `center` (coordonnées carte = pixels).
static func sample_environment(map_data: Object, center: Vector2, distance: float, towns: PackedVector2Array) -> Dictionary:
	var radius := clampf(distance * 0.4, 6.0, 320.0)
	var size: Vector2i = map_data.get("size")
	var splat: Image = map_data.get("splat_image")
	var sea := 0.0
	var forest := 0.0
	var fields := 0.0
	var count := 0
	for iy in 3:
		for ix in 3:
			var p := center + Vector2((ix - 1) * radius, (iy - 1) * radius)
			count += 1
			var px := int(clampf(p.x, 0.0, size.x - 1.0))
			var py := int(clampf(p.y, 0.0, size.y - 1.0))
			if not bool(map_data.call("is_land_px", px, py)):
				sea += 1.0
				continue
			if splat != null:
				var sx := int(float(px) * splat.get_width() / maxf(size.x, 1.0))
				var sy := int(float(py) * splat.get_height() / maxf(size.y, 1.0))
				var c := splat.get_pixel(clampi(sx, 0, splat.get_width() - 1), clampi(sy, 0, splat.get_height() - 1))
				forest += c.b
				fields += minf(c.r + c.g, 1.0)
			else:
				fields += 0.7
	var nearest := INF
	for town in towns:
		nearest = minf(nearest, town.distance_squared_to(center))
	var town_level := 0.0
	if nearest < INF:
		town_level = clampf(1.0 - sqrt(nearest) / (radius * 0.9 + 4.0), 0.0, 1.0)
	return {"sea": sea / count, "forest": forest / count, "fields": fields / count, "town": town_level}


static func _smoothstep(edge0: float, edge1: float, x: float) -> float:
	var t := clampf((x - edge0) / (edge1 - edge0), 0.0, 1.0)
	return t * t * (3.0 - 2.0 * t)


## Niveaux 0..1 par couche. `env` : {sea, forest, fields, town (0..1), distance (zoom),
## season (`spring`…), weather (`clear`, `rain`, `snow`, `storm`)}.
static func layer_levels(env: Dictionary) -> Dictionary:
	var distance := float(env.get("distance", 500.0))
	var season := str(env.get("season", "spring"))
	var weather := str(env.get("weather", "clear"))
	var near := 1.0 - _smoothstep(80.0, 700.0, distance)
	var far := _smoothstep(250.0, 1100.0, distance)
	var sea := float(env.get("sea", 0.0))
	var forest := float(env.get("forest", 0.0))
	var fields := float(env.get("fields", 0.0))
	var town := float(env.get("town", 0.0))
	var wet := weather == "rain" or weather == "storm"
	# La pluie couvre les oiseaux ; la ville couvre en partie la campagne.
	var birds := float(BIRDS.get(season, 0.5)) * (0.35 if wet else 1.0)
	var result := {
		"sea": clampf(sea * (0.35 + 0.65 * near), 0.0, 1.0),
		"countryside": clampf(fields * near * birds * (1.0 - 0.6 * town), 0.0, 1.0),
		"crickets": clampf(fields * near * float(CRICKETS.get(season, 0.0)) * (0.0 if wet else 1.0) * (1.0 - town), 0.0, 1.0),
		"forest": clampf(forest * 1.4 * near * (0.45 if season == "winter" else 1.0), 0.0, 1.0),
		"town": clampf(town * near, 0.0, 1.0),
		"wind": clampf(0.2 + 0.7 * far + (0.2 if wet else 0.0), 0.0, 1.0),
		"wind_strong": clampf(float(WINTER_WIND.get(season, 0.0)) * (0.35 + 0.65 * far) + (0.5 if weather == "snow" or weather == "storm" else 0.0), 0.0, 1.0),
		"rain": (0.5 + 0.5 * near) if wet else 0.0,
	}
	return result
