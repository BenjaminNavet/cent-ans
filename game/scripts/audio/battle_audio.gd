class_name BattleAudio
extends Node3D

## AU1 / T3 — audio de bataille spatialisé, dérivé de l'état des régiments
## (`BattleSim.get_units()` / `get_siege()`), comme les effets visuels de B4.
##
## - **Pool de voix** : `max_voices` `AudioStreamPlayer3D` (banque `data/audio/sound_bank.json`).
##   Chaque événement a une priorité, un nombre d'instances simultanées maximal et un délai de
##   recharge ; pool plein → la voix la moins prioritaire (puis la plus ancienne) est volée si
##   elle ne l'est pas davantage que le nouveau son. Hauteur aléatoire par événement.
## - **Nappes** : pas un son par soldat, mais des boucles 3D (mêlée, clameur, marche, galop, feu)
##   placées au barycentre des régiments concernés proches de la caméra, dont le volume suit le
##   nombre de soldats engagés (pondéré par la distance) ; une nappe 2D « bataille lointaine »
##   prend le relais quand la caméra est haute.
## - **Distance et zoom** : atténuation 3D en distance inverse + filtre d'absorption de l'air ;
##   au-delà de `near_distance_m`, les sons passent par le bus « BatailleLointain » dont le
##   passe-bas et la réverbération suivent la hauteur de la caméra (`AudioBuses`).
## - **Événements** déduits des transitions d'état : charge (cri, galop, hennissement), contact
##   (chocs de boucliers et d'épées), volée (décoche, sifflement, impacts à l'arrivée), pertes en
##   mêlée (râles), déroute, mort de général (cor, ducking de la musique), cri de guerre au premier
##   engagement d'un camp ; siège : bélier, impacts de pierres, effondrement, cloche d'alarme, feu.
## - Météo : pluie, vent (fort sous la neige), tonnerre occasionnel sous la pluie.
##
## API pour les autres modules (BV1…) : `BattleAudio.play_at(événement, position)`.
## Headless : les voix sont créées et les décisions prises (testables), rien n'est joué.

## Instance courante (une seule bataille à la fois).
static var active: BattleAudio = null
## Faux si un autre module joue lui-même les sons de volée (évite les doublons).
static var auto_volley: bool = true

const ENGAGED_STATES := {"melee": true}
## Rayon (m) autour du point visé par la caméra où les régiments nourrissent les nappes.
const BED_RADIUS := 260.0
## Soldats engagés (pondérés par la distance) pour une nappe de mêlée à pleine intensité.
const MELEE_FULL := 420.0
const MARCH_FULL := 500.0
const CAVALRY_FULL := 120.0
const BED_SMOOTHING := 2.5
## Vitesse des traits (m/s) pour caler les impacts à l'arrivée (cf. `BattleEffects.SPEED`).
const MISSILE_SPEED := {"arrow": 48.0, "bolt": 62.0, "ball": 110.0, "stone": 34.0}
const DEATH_GROAN_CHANCE := 0.25
const BELL_PERIOD := 22.0
const BELL_UNTIL := 150.0

var bank: SoundBank
var silent: bool = false
var camera: Camera3D = null
## Hauteur relative de la caméra 0 (au sol) → 1 (vue haute), recalculée à chaque `update`.
var far01: float = 0.0
## Derniers événements joués (tests, débogage) : [{event, position, time}] ; 32 au plus.
var history: Array = []
## Intensités courantes des nappes (0..1).
var bed_levels: Dictionary = {}

var _voices: Array = []  # [{player, event, priority, started}]
var _last_played: Dictionary = {}  # événement → temps
var _scheduled: Array = []  # [{time, event, position, gain}]
var _beds: Dictionary = {}  # nom → AudioStreamPlayer3D
var _ambience: Dictionary = {}  # nom → AudioStreamPlayer (2D)
var _ambience_targets: Dictionary = {}  # nom → volume_db visé
var _track: Dictionary = {}  # id → {state, ammo, soldiers, present}
var _side_cried: Dictionary = {}
var _siege_track: Dictionary = {}
var _weather: String = "clear"
var _time: float = 0.0
var _battle_time: float = 0.0
var _next_bell: float = 4.0
var _next_thunder: float = 20.0
var _rng := RandomNumberGenerator.new()


## `weather` : clé météo du rendu ; `cam` : caméra de la bataille (écouteur).
func setup(weather: String, cam: Camera3D, sound_bank: SoundBank = null) -> void:
	name = "BattleAudio"
	_rng.seed = 4242
	silent = DisplayServer.get_name() == "headless"
	AudioBuses.ensure_layout()
	bank = sound_bank if sound_bank != null else SoundBank.load_default()
	camera = cam
	_weather = weather
	for i in int(bank.voices.get("max_voices", 28)):
		var player := AudioStreamPlayer3D.new()
		player.name = "Voice%d" % i
		player.attenuation_model = AudioStreamPlayer3D.ATTENUATION_INVERSE_DISTANCE
		player.attenuation_filter_cutoff_hz = 6000.0
		player.attenuation_filter_db = -18.0
		player.doppler_tracking = AudioStreamPlayer3D.DOPPLER_TRACKING_DISABLED
		player.bus = AudioBuses.BATTLE
		add_child(player)
		_voices.append({"player": player, "event": "", "priority": -1, "started": -1.0})
	for bed_name in bank.beds:
		var entry: Dictionary = bank.beds[bed_name]
		var bed := AudioStreamPlayer3D.new()
		bed.name = "Bed_" + str(bed_name)
		bed.stream = bank.bed_stream(bed_name)
		bed.bus = str(entry.get("bus", AudioBuses.BATTLE))
		bed.unit_size = float(entry.get("unit_size_m", 40.0))
		bed.max_distance = float(entry.get("max_distance_m", 600.0))
		bed.attenuation_filter_cutoff_hz = 8000.0
		bed.attenuation_filter_db = -12.0
		bed.volume_db = -80.0
		add_child(bed)
		_beds[bed_name] = bed
		bed_levels[bed_name] = 0.0
	for layer in ["battle_distant", "wind", "wind_strong", "rain"]:
		if not bank.ambience.has(layer):
			continue
		var player := AudioStreamPlayer.new()
		player.name = "Ambience_" + layer
		player.stream = bank.ambience_stream(layer)
		player.bus = str(bank.ambience[layer].get("bus", AudioBuses.AMBIENCE))
		player.volume_db = -80.0
		add_child(player)
		_ambience[layer] = player
		_ambience_targets[layer] = -80.0
	_ambience_targets["wind"] = _ambience_db("wind", -6.0 if weather in ["clear", "fog"] else 0.0)
	if weather == "snow":
		_ambience_targets["wind_strong"] = _ambience_db("wind_strong", -3.0)
	if weather == "rain":
		_ambience_targets["rain"] = _ambience_db("rain", 0.0)
	active = self


func _exit_tree() -> void:
	if active == self:
		active = null
	for voice in _voices:
		(voice["player"] as AudioStreamPlayer3D).stop()
		(voice["player"] as AudioStreamPlayer3D).stream = null
	for bed in _beds.values():
		(bed as AudioStreamPlayer3D).stop()
		(bed as AudioStreamPlayer3D).stream = null
	for player in _ambience.values():
		(player as AudioStreamPlayer).stop()
		(player as AudioStreamPlayer).stream = null


# --- API ---------------------------------------------------------------------------


## Joue un événement de la banque à une position monde ; false si rien n'est joué.
static func play_at(event_name: String, position: Vector3, gain_db: float = 0.0) -> bool:
	if active == null or not is_instance_valid(active):
		return false
	return active.play_event(event_name, position, gain_db)


## Comme `play_at`, après `delay` secondes (temps de bataille, figé en pause).
static func play_at_delayed(event_name: String, position: Vector3, delay: float, gain_db: float = 0.0) -> void:
	if active == null or not is_instance_valid(active):
		return
	active.schedule(event_name, position, delay, gain_db)


func schedule(event_name: String, position: Vector3, delay: float, gain_db: float = 0.0) -> void:
	_scheduled.append({"time": _battle_time + maxf(delay, 0.0), "event": event_name, "position": position, "gain": gain_db})


## Joue `event_name` (et ses couches `layers`) ; false si inconnu, trop loin, en recharge ou si
## le pool est saturé par des sons plus prioritaires.
func play_event(event_name: String, position: Vector3, gain_db: float = 0.0) -> bool:
	if bank == null or not bank.has_event(event_name):
		return false
	var entry: Dictionary = bank.event(event_name)
	var played := false
	for layer in entry.get("layers", []):
		played = play_event(str(layer), position, gain_db) or played
	if (entry.get("files", []) as Array).is_empty():
		if played:
			_duck(entry)
		return played
	var listener := _listener_position()
	var distance := listener.distance_to(position)
	if distance > float(entry["max_distance_m"]):
		return played
	var cooldown := float(entry["cooldown_s"])
	if cooldown > 0.0 and _time - float(_last_played.get(event_name, -INF)) < cooldown:
		return played
	var voice := _claim_voice(event_name, int(entry["priority"]), int(entry["max_instances"]))
	if voice.is_empty():
		return played
	var stream := bank.pick_stream(event_name)
	if stream == null:
		return played
	var player: AudioStreamPlayer3D = voice["player"]
	player.stop()
	player.stream = stream
	player.global_position = position
	player.unit_size = float(entry["unit_size_m"])
	player.max_distance = float(entry["max_distance_m"])
	player.volume_db = float(entry["volume_db"]) + gain_db
	player.pitch_scale = bank.random_pitch(event_name)
	var near_distance := float(bank.voices.get("near_distance_m", 70.0))
	var bus := str(entry["bus"])
	player.bus = AudioBuses.BATTLE_FAR if bus == AudioBuses.BATTLE and distance > near_distance else bus
	voice["event"] = event_name
	voice["priority"] = int(entry["priority"])
	voice["started"] = _time
	voice["length"] = stream.get_length() / maxf(player.pitch_scale, 0.01)
	_last_played[event_name] = _time
	if not silent:
		player.play()
	_duck(entry)
	history.append({"event": event_name, "position": position, "time": _battle_time})
	if history.size() > 32:
		history.pop_front()
	return true


## Nombre de voix occupées (tests).
func busy_voices() -> int:
	var count := 0
	for voice in _voices:
		if _is_busy(voice):
			count += 1
	return count


func voice_count() -> int:
	return _voices.size()


# --- Pool --------------------------------------------------------------------------


func _is_busy(voice: Dictionary) -> bool:
	if str(voice["event"]) == "":
		return false
	if not silent:
		return (voice["player"] as AudioStreamPlayer3D).playing
	return _time - float(voice["started"]) < float(voice.get("length", 0.0))


func _claim_voice(event_name: String, priority: int, max_instances: int) -> Dictionary:
	var same: Array = []
	var free: Dictionary = {}
	var victim: Dictionary = {}
	for voice in _voices:
		if not _is_busy(voice):
			if free.is_empty():
				free = voice
			continue
		if str(voice["event"]) == event_name:
			same.append(voice)
		if victim.is_empty() or int(voice["priority"]) < int(victim["priority"]) or (int(voice["priority"]) == int(victim["priority"]) and float(voice["started"]) < float(victim["started"])):
			victim = voice
	if same.size() >= max_instances:
		# Limite d'instances : on remplace la plus ancienne du même événement.
		var oldest: Dictionary = same[0]
		for voice in same:
			if float(voice["started"]) < float(oldest["started"]):
				oldest = voice
		return oldest
	if not free.is_empty():
		return free
	if not victim.is_empty() and int(victim["priority"]) <= priority:
		return victim
	return {}


func _listener_position() -> Vector3:
	if camera != null and is_instance_valid(camera) and camera.is_inside_tree():
		return camera.global_position
	return Vector3.ZERO


func _duck(entry: Dictionary) -> void:
	var duck_db := float(entry.get("duck_music_db", 0.0))
	if duck_db >= 0.0:
		return
	var director := get_node_or_null("/root/AudioDirector")
	if director != null and director.has_method("duck_music"):
		director.call("duck_music", duck_db, float(entry.get("duck_seconds", 3.0)))


# --- Mise à jour -------------------------------------------------------------------


## `units` : `BattleSim.get_units()` ; `focus` : point visé au sol ; `camera_height` : hauteur
## de la caméra au-dessus du sol (zoom) ; `dt` : temps de bataille écoulé (0 en pause) ;
## `elapsed` : temps de bataille.
func update(units: Array, focus: Vector3, camera_height: float, dt: float, real_dt: float, elapsed: float) -> void:
	_time += real_dt
	_battle_time = elapsed
	far01 = clampf((camera_height - 25.0) / 260.0, 0.0, 1.0)
	AudioBuses.set_battle_distance(far01)
	_run_scheduled()
	if dt > 0.0:
		_detect_events(units, elapsed)
	_update_beds(units, focus, real_dt)
	_update_weather(real_dt)


## Siège (`BattleSim.get_siege()`) : bélier, impacts, effondrements, feu, cloche d'alarme.
func update_siege(siege: Dictionary, elapsed: float) -> void:
	if siege.is_empty():
		return
	var center_v: Vector2 = siege.get("center", Vector2.ZERO)
	var center := Vector3(center_v.x, 0.0, center_v.y)
	if elapsed < BELL_UNTIL and elapsed >= _next_bell:
		_next_bell = elapsed + BELL_PERIOD
		play_event("bell_toll", center + Vector3(0, 25, 0))
	for piece in siege.get("pieces", []):
		var index := int(piece.get("index", -1))
		var hp := float(piece.get("hp", 0.0))
		var intact := bool(piece.get("intact", true))
		var a: Vector2 = piece.get("a", Vector2.ZERO)
		var b: Vector2 = piece.get("b", Vector2.ZERO)
		var mid := Vector3((a.x + b.x) * 0.5, 4.0, (a.y + b.y) * 0.5)
		var key := "piece%d" % index
		if _siege_track.has(key):
			var prev: Dictionary = _siege_track[key]
			if hp < float(prev["hp"]) - 0.001:
				play_event("ram_hit" if str(piece.get("kind", "")) == "gate" else "stone_impact", mid)
			if bool(prev["intact"]) and not intact:
				play_event("wall_collapse", mid)
		_siege_track[key] = {"hp": hp, "intact": intact}
	# Feu : la nappe se place sur la maison en flammes la plus proche du point visé.
	var burning: Array = []
	for house in siege.get("houses", []):
		var fire: Dictionary = house.get("fire", {})
		if str(fire.get("state", "")) == "burning":
			burning.append({"pos": Vector3(float(house["x"]), 3.0, float(house["z"])), "intensity": float(fire.get("intensity", 1.0))})
	var gate_fire: Dictionary = siege.get("gate_fire", {})
	if str(gate_fire.get("state", "")) == "burning":
		burning.append({"pos": center, "intensity": float(gate_fire.get("intensity", 1.0))})
	_siege_track["fires"] = burning


func _run_scheduled() -> void:
	if _scheduled.is_empty():
		return
	var remaining: Array = []
	for item in _scheduled:
		if float(item["time"]) <= _battle_time:
			play_event(str(item["event"]), item["position"], float(item["gain"]))
		else:
			remaining.append(item)
	_scheduled = remaining


static func _pos(unit: Dictionary) -> Vector3:
	return Vector3(float(unit["x"]), float(unit.get("y", 0.0)) + 1.5, float(unit["z"]))


static func _forward(unit: Dictionary) -> Vector3:
	var facing := float(unit.get("facing", 0.0))
	return Vector3(sin(facing), 0.0, cos(facing))


static func missile_kind(unit: Dictionary) -> String:
	var type := str(unit.get("type", ""))
	if str(unit.get("render", "")) == "siege":
		return "ball" if type == "unit_bombard" else "stone"
	if type.contains("crossbow"):
		return "bolt"
	return "arrow"


func _detect_events(units: Array, elapsed: float) -> void:
	var by_id := {}
	for unit in units:
		by_id[int(unit["id"])] = unit
	for unit in units:
		var id := int(unit["id"])
		var state := str(unit["state"])
		var present := bool(unit["present"])
		var soldiers := int(unit.get("soldiers", 0))
		var ammo := int(unit.get("ammo", 0))
		var prev: Dictionary = _track.get(id, {})
		_track[id] = {"state": state, "ammo": ammo, "soldiers": soldiers, "present": present}
		if prev.is_empty():
			continue
		var pos := _pos(unit)
		var mounted := str(unit.get("render", "")) == "cavalry"
		var prev_state := str(prev["state"])
		if bool(unit.get("is_general", false)) and bool(prev["present"]) and (not present or soldiers <= 0):
			play_event("general_death", pos)
			continue
		if not present:
			continue
		if state == "charging" and prev_state != "charging":
			var side := str(unit.get("side", ""))
			if not _side_cried.has(side):
				_side_cried[side] = elapsed
				play_event("war_cry", pos + _forward(unit) * 4.0)
			play_event("charge_cry", pos)
			if mounted:
				play_event("horse_neigh", pos)
		if state == "melee" and prev_state != "melee":
			var front := pos + _forward(unit) * float(unit.get("depth", 6.0)) * 0.5
			play_event("contact", front, 2.0 if prev_state == "charging" else 0.0)
		if state == "routing" and prev_state != "routing":
			play_event("rout_cry", pos)
		if auto_volley and ammo < int(prev["ammo"]):
			_on_volley(unit, by_id)
		var lost := int(prev["soldiers"]) - soldiers
		if lost > 0 and state == "melee" and _rng.randf() < DEATH_GROAN_CHANCE * minf(float(lost), 3.0):
			play_event("death_groan", pos + Vector3(_rng.randf_range(-5, 5), 0, _rng.randf_range(-5, 5)))


func _on_volley(unit: Dictionary, by_id: Dictionary) -> void:
	var pos := _pos(unit)
	var target_id := int(unit.get("target", -1))
	var kind := missile_kind(unit)
	match kind:
		"ball":
			play_event("bombard", pos)
		"stone":
			play_event("trebuchet_release", pos)
		"bolt":
			play_event("crossbow_release", pos)
		_:
			play_event("bow_release", pos)
	if not by_id.has(target_id):
		return
	var target: Dictionary = by_id[target_id]
	var aim := _pos(target)
	var flight := pos.distance_to(aim) / float(MISSILE_SPEED[kind])
	if kind == "arrow" or kind == "bolt":
		schedule("arrow_whistle", pos.lerp(aim, 0.55) + Vector3(0, 12, 0), minf(flight * 0.35, 1.5))
		schedule("arrow_impact", aim, flight)
	else:
		schedule("stone_impact", aim, flight)


func _update_beds(units: Array, focus: Vector3, real_dt: float) -> void:
	var sums := {"melee": [0.0, Vector3.ZERO, 0.0], "march": [0.0, Vector3.ZERO, 0.0], "cavalry": [0.0, Vector3.ZERO, 0.0]}
	var engaged_total := 0.0
	for unit in units:
		if not bool(unit["present"]):
			continue
		var state := str(unit["state"])
		var soldiers := float(unit.get("soldiers", 0))
		var pos := _pos(unit)
		var key := ""
		if ENGAGED_STATES.has(state):
			key = "melee"
			engaged_total += soldiers
		elif str(unit.get("render", "")) == "cavalry" and (state == "charging" or state == "marching" or state == "routing"):
			key = "cavalry"
		elif state == "marching" or state == "charging" or state == "routing":
			key = "march"
		if key == "":
			continue
		var d := Vector2(pos.x - focus.x, pos.z - focus.z).length()
		if d > BED_RADIUS:
			continue
		var weight := soldiers * (1.0 - d / BED_RADIUS)
		var acc: Array = sums[key]
		acc[0] += weight
		acc[1] += pos * weight
		acc[2] += soldiers
	var melee_level := _level(float(sums["melee"][0]), MELEE_FULL)
	var near := 1.0 - far01 * 0.7
	_drive_bed("melee_bed_1", melee_level * near, sums["melee"], focus, real_dt)
	_drive_bed("melee_bed_2", clampf(melee_level * 1.6 - 0.6, 0.0, 1.0) * near, sums["melee"], focus, real_dt)
	_drive_bed("clamor_bed", melee_level * (0.6 + 0.4 * near), sums["melee"], focus, real_dt)
	_drive_bed("march_bed", _level(float(sums["march"][0]), MARCH_FULL) * near, sums["march"], focus, real_dt)
	_drive_bed("cavalry_bed", _level(float(sums["cavalry"][0]), CAVALRY_FULL) * near, sums["cavalry"], focus, real_dt)
	var fires: Array = _siege_track.get("fires", [])
	var fire_acc := [0.0, Vector3.ZERO, 0.0]
	var fire_best := INF
	for fire in fires:
		var d := (fire["pos"] as Vector3).distance_to(focus)
		if d < fire_best:
			fire_best = d
			fire_acc = [1.0, fire["pos"], 1.0]
	var fire_level := clampf(float(fires.size()) / 4.0, 0.0, 1.0) if not fires.is_empty() else 0.0
	_drive_bed("fire_bed", fire_level, fire_acc, focus, real_dt)
	# Vue haute : la bataille entière en 2D, étouffée ; d'autant plus que la mêlée est large.
	var distant := _level(engaged_total, MELEE_FULL * 3.0) * clampf(far01 * 1.4, 0.0, 1.0)
	_ambience_targets["battle_distant"] = _ambience_db("battle_distant", 0.0) + linear_to_db(maxf(distant, 0.0001)) if distant > 0.01 else -80.0


## Intensité 0..1 d'une nappe : racine (plusieurs régiments ne sonnent pas n fois plus fort).
static func _level(weight: float, full: float) -> float:
	return clampf(sqrt(maxf(weight, 0.0) / full), 0.0, 1.0)


func _drive_bed(bed_name: String, target: float, acc: Array, focus: Vector3, real_dt: float) -> void:
	if not _beds.has(bed_name):
		return
	var bed: AudioStreamPlayer3D = _beds[bed_name]
	var level := float(bed_levels.get(bed_name, 0.0))
	level = lerpf(level, target, clampf(real_dt * BED_SMOOTHING, 0.0, 1.0))
	bed_levels[bed_name] = level
	if float(acc[0]) > 0.0:
		var center: Vector3 = acc[1] / float(acc[0])
		# La nappe glisse vers son barycentre (pas de saut quand un régiment entre ou sort).
		bed.global_position = bed.global_position.lerp(center, clampf(real_dt * 2.0, 0.0, 1.0)) if bed.playing or silent else center
	elif not bed.playing:
		bed.global_position = focus
	var base := float(bank.beds[bed_name].get("volume_db", 0.0))
	bed.volume_db = base + linear_to_db(maxf(level, 0.0001))
	if silent or bed.stream == null:
		return
	if level > 0.02 and not bed.playing:
		bed.play(_rng.randf_range(0.0, maxf(bed.stream.get_length() - 1.0, 0.0)))
	elif level <= 0.01 and bed.playing:
		bed.stop()


func _ambience_db(layer: String, offset: float) -> float:
	return float(bank.ambience.get(layer, {}).get("volume_db", 0.0)) + offset


func _update_weather(real_dt: float) -> void:
	for layer in _ambience:
		var player: AudioStreamPlayer = _ambience[layer]
		var target := float(_ambience_targets.get(layer, -80.0))
		# Le vent monte avec la hauteur de la caméra.
		if layer == "wind":
			target += lerpf(-6.0, 2.0, far01)
		player.volume_db = move_toward(player.volume_db, target, real_dt * 12.0)
		if silent or player.stream == null:
			continue
		if player.volume_db > -60.0 and not player.playing:
			player.play()
		elif player.volume_db <= -79.0 and player.playing:
			player.stop()
	if _weather == "rain" and _battle_time >= _next_thunder:
		_next_thunder = _battle_time + _rng.randf_range(35.0, 80.0)
		var angle := _rng.randf_range(0.0, TAU)
		play_event("thunder", _listener_position() + Vector3(cos(angle) * 250.0, 120.0, sin(angle) * 250.0))
