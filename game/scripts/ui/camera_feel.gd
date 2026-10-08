class_name CameraFeel
extends RefCounted

## Chantier PO5 (ADR 0097) : ressenti des caméras, lu dans `data/ui/camera_feel.json` (schéma
## `data/schemas/camera_feel.schema.json`). `get_value(block, key)` rend la valeur du bloc
## `campaign` ou `battle`, remplacée par celle de `reduce_motion` quand « Réduire les
## animations » est actif. Les valeurs de repli ne servent que si le fichier manque.

const DATA_FILE := "ui/camera_feel.json"
const FALLBACK := {
	"campaign": {
		"pan_accel": 14.0, "pan_friction": 5.5, "follow_damping": 12.0, "zoom_damping": 9.0,
		"rotate_damping": 10.0, "focus_glide_s": 0.4, "drag_release_inertia": 0.6,
	},
	"battle": {
		"zoom_damping": 9.0, "focus_glide_s": 0.4, "opening_distance_m": 155.0, "opening_ahead_m": 10.0,
		"marker_shrink_start_m": 260.0, "marker_shrink_end_m": 700.0, "marker_min_scale": 0.5,
	},
	"reduce_motion": {},
}

static var _lookup := JsonLookup.new(DATA_FILE, FALLBACK)


## Valeur `key` du bloc `block` (`"campaign"` ou `"battle"`), selon « Réduire les animations ».
static func get_value(block: String, key: String) -> float:
	var feel := settings()
	if Accessibility.reduce_motion():
		var reduced: Dictionary = feel.get("reduce_motion", {})
		if reduced.has(key):
			return float(reduced[key])
	var values: Dictionary = feel.get(block, {})
	return float(values.get(key, (FALLBACK.get(block, {}) as Dictionary).get(key, 0.0)))


## Réglages complets (lus une fois) : repli complété par le fichier.
static func settings() -> Dictionary:
	return _lookup.data()


## Vrai si les valeurs viennent du fichier de données (tests).
static func loaded_from_data() -> bool:
	settings()
	return _lookup.found


## Relit le fichier au prochain accès (tests).
static func reload() -> void:
	_lookup.reload()


