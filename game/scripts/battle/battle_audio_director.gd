class_name BattleAudioDirector
extends RefCounted

## Son de la bataille : musique dynamique (B3), sons spatialisés (AU1), répliques des
## régiments (VO1) et mise en veille de la musique de campagne pendant le combat. Extrait de
## `BattleScene`, qui expose `music`, `battle_audio` et `voices` (tests, alertes du HUD).

const BATTLE_MUSIC := preload("res://scripts/battle/battle_music.gd")
const SIEGE_AUDIO_PERIOD := 0.25  # le siège se lit 4 fois par seconde

var music: BattleMusicDirector = null
var battle_audio: BattleAudio = null
var voices: BattleVoices = null

var _scene: BattleScene = null
var _campaign_director: Node = null  # autoload de la carte, mis en veille pendant la bataille
var _siege_timer: float = 0.0


func _init(scene: BattleScene) -> void:
	_scene = scene


## La musique de campagne cède la place à la musique de bataille (réveillée au retour).
func silence_campaign() -> void:
	_campaign_director = _scene.get_node_or_null("/root/AudioDirector")
	if _campaign_director != null:
		_campaign_director.call("stop_all")


## La carte retrouve sa musique de contexte.
func restore_campaign() -> void:
	if _campaign_director != null:
		_campaign_director.call("refresh_context")


## Crée musique, sons spatialisés et répliques ; `weather_key` règle l'ambiance météo.
func build(weather_key: String) -> void:
	var scene := _scene
	music = BATTLE_MUSIC.new()
	music.name = "Music"
	scene.add_child(music)
	music.setup(scene)
	battle_audio = BattleAudio.new()
	scene.add_child(battle_audio)
	battle_audio.setup(weather_key, scene.camera_rig.camera)
	scene.hud.alerts_column.battle_audio = battle_audio  # Cris (déroute, général tombé)
	voices = BattleVoices.new()  # VO1
	scene.add_child(voices)
	voices.setup(scene)


## Un événement sonore des effets (volée, impact) : tout de suite ou après `delay` s.
func on_sound_event(event: StringName, position: Vector3, delay: float) -> void:
	if delay > 0.0:
		BattleAudio.play_at_delayed(str(event), position, delay)
	else:
		BattleAudio.play_at(str(event), position)


## Musique d'intensité, puis sons spatialisés d'après les régiments (et le siège, 4 fois par
## seconde) et répliques.
func update(delta: float, units: Array, running: bool, speed: float, elapsed: float, siege: Dictionary, has_siege: bool) -> void:
	if music != null:
		music.update(delta, units)
	if battle_audio == null:
		return
	var camera_rig := _scene.camera_rig
	var height := camera_rig.camera.global_position.y - camera_rig.target.y
	battle_audio.update(units, camera_rig.target, height, delta * speed if running else 0.0, delta, elapsed)
	if has_siege and running:
		_siege_timer -= delta
		if _siege_timer <= 0.0:
			_siege_timer = SIEGE_AUDIO_PERIOD
			battle_audio.update_siege(siege, elapsed)
	if voices != null:
		voices.update(delta)
