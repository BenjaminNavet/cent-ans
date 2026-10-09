class_name BattleCavalryGaits
extends RefCounted

## Lot AS3 : allures de la cavalerie de bataille (rendu seulement, aucune règle de jeu).
## La simulation ne connaît que « marche » et « course » ; le rendu choisit, d'après la vitesse
## lissée du régiment et la variation de son orientation, entre le pas (`c_walk`), le trot
## (`c_trot`, bande intermédiaire), le galop (`c_gallop`) et les virages (`c_turn_l/r`,
## `c_trot_turn_l/r` : corps incliné, encolure fléchie vers l'intérieur). Seuils dans
## `data/fx/battle_animation.json` (`cavalry_gaits`).

const BAND_WALK := 0
const BAND_TROT := 1
const BAND_GALLOP := 2

static var _override: Dictionary = {}


## Réglages (surchargeables par les tests).
static func settings() -> Dictionary:
	if not _override.is_empty():
		return _override
	return BattleSkinned.animation_settings().get("cavalry_gaits", {})


static func set_override(values: Dictionary) -> void:
	_override = values


static func enabled() -> bool:
	var cfg := settings()
	return not cfg.is_empty() and bool(cfg.get("enabled", true))


## Allure et virage d'un régiment : `mem` = mémoire du régiment ({band, turn}, modifiée) pour
## l'hystérésis ; `speed` en m/s ; `turn_rate` en rad/s (> 0 = vers la gauche, sens de
## `facing` : x = sin f, z = cos f). Renvoie la clé d'état de `BattleSkinned.state_config` :
## marching, running, trotting, turn_l, turn_r, trot_turn_l, trot_turn_r.
static func pick(mem: Dictionary, speed: float, running: bool, turn_rate: float) -> String:
	var cfg := settings()
	var trot_min := float(cfg.get("trot_min_speed", 3.2))
	var gallop_min := float(cfg.get("gallop_min_speed", 5.6))
	var hyst := float(cfg.get("hysteresis_mps", 0.4))
	var band: int = int(mem.get("band", BAND_WALK))
	# Seuils décalés dans le sens de l'allure en cours : on ne la quitte qu'au-delà de l'hystérésis.
	var up_trot := trot_min + (0.0 if band >= BAND_TROT else hyst)
	var up_gallop := gallop_min + (0.0 if band >= BAND_GALLOP else hyst)
	var down_trot := trot_min - (hyst if band >= BAND_TROT else 0.0)
	var down_gallop := gallop_min - (hyst if band >= BAND_GALLOP else 0.0)
	var next := BAND_WALK
	if running and speed >= (down_gallop if band == BAND_GALLOP else up_gallop):
		next = BAND_GALLOP
	elif speed >= (down_trot if band >= BAND_TROT else up_trot):
		next = BAND_TROT
	if not running and next == BAND_GALLOP:
		next = BAND_TROT
	mem["band"] = next
	# Virage : seuil abaissé par l'hystérésis tant que le virage dure.
	var turn_min := float(cfg.get("turn_min_rate", 0.16))
	var turning := int(mem.get("turn", 0))
	var limit := turn_min - (float(cfg.get("turn_hysteresis", 0.05)) if turning != 0 else 0.0)
	if absf(turn_rate) >= limit and next != BAND_GALLOP:
		turning = 1 if turn_rate > 0.0 else -1
	else:
		turning = 0
	mem["turn"] = turning
	if next == BAND_GALLOP:
		return "running"
	if turning != 0:
		var side := "l" if turning > 0 else "r"
		return ("trot_turn_" if next == BAND_TROT else "turn_") + side
	return "trotting" if next == BAND_TROT else "marching"


## Variation d'orientation (rad/s, entre -π et π par pas) entre deux orientations.
static func turn_rate(previous: float, current: float, dt: float) -> float:
	if dt <= 0.0:
		return 0.0
	return wrapf(current - previous, -PI, PI) / dt
