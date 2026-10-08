class_name AudioBuses
extends RefCounted

## Disposition des bus audio, créée en code (idempotent) au démarrage d'`AudioDirector` :
##
##   Master (limiteur dur)
##   ├─ Musique (amplification de ducking, compresseur côté-chaîne sur « Voix »)
##   │   └─ BatailleMusique (passe-bas d'intensité, créé par `BattleMusicDirector`)
##   ├─ Ambiance (compresseur doux)
##   ├─ Bataille (compresseur)
##   │   └─ BatailleLointain (passe-bas + réverbération réglés selon le zoom)
##   ├─ Interface
##   └─ Voix
##
## Les volumes réglables par le joueur sont ceux de `PLAYER_BUSES` (Master compris). Le ducking
## passe par l'effet d'amplification de « Musique » (`DUCK_EFFECT`), jamais par le volume du bus,
## pour ne pas écraser le réglage du joueur.

const MASTER := "Master"
const MUSIC := "Musique"
const AMBIENCE := "Ambiance"
const BATTLE := "Bataille"
const BATTLE_FAR := "BatailleLointain"
const INTERFACE := "Interface"
const VOICE := "Voix"

## Bus réglables : [nom du bus, libellé du curseur].
const PLAYER_BUSES := [
	[MASTER, "Général"],
	[MUSIC, "Musique"],
	[AMBIENCE, "Ambiance"],
	[BATTLE, "Bataille"],
	[INTERFACE, "Interface"],
	[VOICE, "Voix"],
]

## Index des effets sur les bus (ordre d'insertion ci-dessous).
const DUCK_EFFECT := 0  # Musique : AudioEffectAmplify
const FAR_LOWPASS_EFFECT := 0  # BatailleLointain : AudioEffectLowPassFilter
const FAR_REVERB_EFFECT := 1  # BatailleLointain : AudioEffectReverb


## Crée les bus manquants et leurs effets. Sans effet si la disposition existe déjà.
static func ensure_layout() -> void:
	_ensure_master()
	_ensure(MUSIC, MASTER, _music_effects)
	_ensure(AMBIENCE, MASTER, func() -> Array: return [_compressor(-14.0, 3.0, 20.0, 250.0)])
	_ensure(BATTLE, MASTER, func() -> Array: return [_compressor(-16.0, 4.0, 10.0, 180.0)])
	_ensure(BATTLE_FAR, BATTLE, _far_effects)
	_ensure(INTERFACE, MASTER, func() -> Array: return [])
	_ensure(VOICE, MASTER, func() -> Array: return [_compressor(-12.0, 3.0, 5.0, 150.0)])


static func bus_index(bus_name: String) -> int:
	return AudioServer.get_bus_index(bus_name)


## Volume du joueur (linéaire 0..1) sur un bus ; muet sous 0,001.
static func set_linear_volume(bus_name: String, linear: float) -> void:
	var index := bus_index(bus_name)
	if index == -1:
		return
	AudioServer.set_bus_volume_db(index, linear_to_db(maxf(linear, 0.0001)))
	AudioServer.set_bus_mute(index, linear <= 0.001)


## Atténuation de ducking de la musique (dB, ≤ 0).
static func set_music_duck_db(db: float) -> void:
	var amplify := _effect(MUSIC, DUCK_EFFECT) as AudioEffectAmplify
	if amplify != null:
		amplify.volume_db = db


static func music_duck_db() -> float:
	var amplify := _effect(MUSIC, DUCK_EFFECT) as AudioEffectAmplify
	return amplify.volume_db if amplify != null else 0.0


## Bus lointain de bataille selon le zoom : `far01` 0 (caméra au ras du sol) → 1 (vue haute).
## Plus la caméra est haute, plus le son est filtré et réverbéré (distance, écho du champ).
static func set_battle_distance(far01: float) -> void:
	var t := clampf(far01, 0.0, 1.0)
	var lowpass := _effect(BATTLE_FAR, FAR_LOWPASS_EFFECT) as AudioEffectLowPassFilter
	if lowpass != null:
		lowpass.cutoff_hz = lerpf(7000.0, 1800.0, t)
	var reverb := _effect(BATTLE_FAR, FAR_REVERB_EFFECT) as AudioEffectReverb
	if reverb != null:
		reverb.wet = lerpf(0.12, 0.38, t)
		reverb.dry = lerpf(0.95, 0.75, t)
		reverb.room_size = lerpf(0.45, 0.8, t)


# --- Construction ------------------------------------------------------------------


static func _ensure_master() -> void:
	var index := bus_index(MASTER)
	if index == -1:
		return
	for i in AudioServer.get_bus_effect_count(index):
		if AudioServer.get_bus_effect(index, i) is AudioEffectHardLimiter:
			return
	var limiter := AudioEffectHardLimiter.new()
	limiter.ceiling_db = -0.5
	limiter.pre_gain_db = 0.0
	AudioServer.add_bus_effect(index, limiter)


static func _ensure(bus_name: String, parent: String, effects: Callable) -> void:
	var index := bus_index(bus_name)
	if index == -1:
		AudioServer.add_bus()
		index = AudioServer.bus_count - 1
		AudioServer.set_bus_name(index, bus_name)
	AudioServer.set_bus_send(index, parent)
	if AudioServer.get_bus_effect_count(index) == 0:
		for effect in effects.call():
			AudioServer.add_bus_effect(index, effect)


static func _music_effects() -> Array:
	var amplify := AudioEffectAmplify.new()
	amplify.volume_db = 0.0
	var sidechain := _compressor(-20.0, 3.0, 20.0, 400.0)
	sidechain.sidechain = VOICE
	return [amplify, sidechain]


static func _far_effects() -> Array:
	var lowpass := AudioEffectLowPassFilter.new()
	lowpass.cutoff_hz = 6000.0
	var reverb := AudioEffectReverb.new()
	reverb.room_size = 0.6
	reverb.damping = 0.6
	reverb.spread = 0.8
	reverb.hipass = 0.15
	reverb.wet = 0.2
	reverb.dry = 0.9
	reverb.predelay_msec = 60.0
	return [lowpass, reverb]


static func _compressor(threshold: float, ratio: float, attack_us: float, release_ms: float) -> AudioEffectCompressor:
	var compressor := AudioEffectCompressor.new()
	compressor.threshold = threshold
	compressor.ratio = ratio
	compressor.attack_us = attack_us
	compressor.release_ms = release_ms
	compressor.gain = 0.0
	return compressor


static func _effect(bus_name: String, effect_index: int) -> AudioEffect:
	var index := bus_index(bus_name)
	if index == -1 or AudioServer.get_bus_effect_count(index) <= effect_index:
		return null
	return AudioServer.get_bus_effect(index, effect_index)
