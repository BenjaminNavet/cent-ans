class_name ExpectedErrors
extends RefCounted

## Erreurs attendues d'un test (chemin de repli exercé volontairement, ex. fixtures sans relief ni
## données de simulation dans le smoke). Pendant `begin()` … `end()`, les sites qui connaissent ce
## repli rendent leur message en avertissement préfixé « [attendu] » au lieu d'une erreur, pour
## qu'un `grep '^ERROR'` d'un journal de test ne trouve que de vraies pannes. Le drapeau est une
## métadonnée du singleton `Engine`, lue aussi par le pont natif (`godot-bridge`).

const META := "expected_errors"
const PREFIX := "[attendu] "


static func begin() -> void:
	Engine.set_meta(META, true)


static func end() -> void:
	if Engine.has_meta(META):
		Engine.remove_meta(META)


static func active() -> bool:
	return Engine.has_meta(META)


## `push_error(message)`, ou `push_warning("[attendu] " + message)` quand le repli est attendu.
static func report(message: String) -> void:
	if active():
		push_warning(PREFIX + message)
	else:
		push_error(message)
