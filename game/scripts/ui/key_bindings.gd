class_name KeyBindings
extends RefCounted

## Lot NT6d : réaffectation libre des touches. Les actions listées sont celles de la fiche des
## raccourcis (`ShortcutSheet.CAMPAIGN_SECTIONS`, libellés français). Les touches modifiées sont
## enregistrées dans le réglage `input/bindings` (action → liste de touches physiques, modificateurs
## compris) et réappliquées à l'`InputMap` au démarrage (`Settings._ready`). Les défauts sont ceux
## de `project.godot`. Aucune règle de jeu.

const SETTING_KEY := "input/bindings"
## Actions qui n'agissent que pendant une infobulle ouverte : elles peuvent partager une touche
## avec une action de la carte (T par défaut) sans conflit.
const CONTEXTUAL: Array[String] = ["codex_pin_tooltip", "tooltip_explore"]


## Actions réaffectables : `[{title, actions: [[action, libellé]]}]`, celles présentes dans l'`InputMap`.
static func sections() -> Array:
	var result: Array = []
	for section in ShortcutSheet.CAMPAIGN_SECTIONS:
		var rows: Array = []
		for pair in section["actions"]:
			if InputMap.has_action(str(pair[0])):
				rows.append([str(pair[0]), str(pair[1])])
		if not rows.is_empty():
			result.append({"title": str(section["title"]), "actions": rows})
	return result


static func all_actions() -> Array[String]:
	var actions: Array[String] = []
	for section in sections():
		for row in section["actions"]:
			actions.append(str(row[0]))
	return actions


## Code d'une touche (physique) avec modificateurs ; 0 pour un évènement qui n'est pas au clavier.
static func encode(event: InputEvent) -> int:
	var key := event as InputEventKey
	if key == null:
		return 0
	var code: int = key.physical_keycode if key.physical_keycode != KEY_NONE else key.keycode
	if code == KEY_NONE:
		return 0
	if key.shift_pressed:
		code |= KEY_MASK_SHIFT
	if key.ctrl_pressed:
		code |= KEY_MASK_CTRL
	if key.alt_pressed:
		code |= KEY_MASK_ALT
	return code


static func decode(code: int) -> InputEventKey:
	var key := InputEventKey.new()
	key.physical_keycode = (code & KEY_CODE_MASK) as Key
	key.shift_pressed = (code & KEY_MASK_SHIFT) != 0
	key.ctrl_pressed = (code & KEY_MASK_CTRL) != 0
	key.alt_pressed = (code & KEY_MASK_ALT) != 0
	return key


## Touches actuelles d'une action (clavier seulement), dans l'ordre de l'`InputMap`.
static func codes(action: String) -> Array:
	var result: Array = []
	if not InputMap.has_action(action):
		return result
	for event in InputMap.action_get_events(action):
		var code := encode(event)
		if code != 0:
			result.append(code)
	return result


## Touches par défaut (project.godot).
static func default_codes(action: String) -> Array:
	var result: Array = []
	var entry: Variant = ProjectSettings.get_setting("input/" + action, null)
	if entry is Dictionary:
		for event in (entry as Dictionary).get("events", []):
			var code := encode(event)
			if code != 0:
				result.append(code)
	return result


static func _set_codes(action: String, new_codes: Array) -> void:
	var kept: Array[InputEvent] = []
	for event in InputMap.action_get_events(action):
		if not event is InputEventKey:
			kept.append(event)  # souris, manette : conservés
	InputMap.action_erase_events(action)
	for code in new_codes:
		InputMap.action_add_event(action, decode(int(code)))
	for event in kept:
		InputMap.action_add_event(action, event)


## Action (parmi les listées) qui utilise `code`, hors `except` ; "" sinon.
static func action_using(code: int, except: String) -> String:
	for action in all_actions():
		if action != except and not CONTEXTUAL.has(action) and codes(action).has(code):
			return action
	return ""


## Libellé français d'une action listée.
static func label_of(action: String) -> String:
	for section in sections():
		for row in section["actions"]:
			if row[0] == action:
				return str(row[1])
	return action


## Réaffecte la touche `slot` (0 = principale) de `action` à `code`. Si une autre action
## utilisait déjà `code`, elle reçoit l'ancienne touche de `action` (échange) ; renvoie l'action
## touchée ("" sans conflit). Persiste dans `settings` (autoload Settings) quand il est fourni.
static func rebind(action: String, slot: int, code: int, settings: Object = null) -> String:
	var mine := codes(action)
	if mine.is_empty() or code == 0:
		return ""
	slot = clampi(slot, 0, mine.size() - 1)
	var old_code: int = int(mine[slot])
	if old_code == code:
		return ""
	var other := action_using(code, action)
	var touched: Array[String] = [action]
	if other != "":
		var theirs := codes(other)
		theirs[theirs.find(code)] = old_code
		_set_codes(other, theirs)
		touched.append(other)
	elif mine.has(code):
		# Déjà sur une autre touche de la même action : on échange les deux.
		mine[mine.find(code)] = old_code
	mine[slot] = code
	_set_codes(action, mine)
	_save(touched, settings)
	return other


## Rétablit les touches par défaut (toutes les actions, ou `action` seule).
static func reset(action: String = "", settings: Object = null) -> void:
	var targets: Array[String] = all_actions()
	if action != "":
		targets = [action]
	for target in targets:
		_set_codes(target, default_codes(target))
	_save(targets, settings)


## Enregistre l'état des actions `touched` (retire celles revenues aux défauts).
static func _save(touched: Array[String], settings: Object) -> void:
	if settings == null:
		return
	var stored: Dictionary = (settings.call("get_value", SETTING_KEY) as Dictionary).duplicate(true)
	for action in touched:
		var current := codes(action)
		if current == default_codes(action):
			stored.erase(action)
		else:
			stored[action] = current
	settings.call("set_value", SETTING_KEY, stored)


## Applique à l'`InputMap` les touches enregistrées (au démarrage ; actions inconnues ignorées).
static func apply_saved(stored: Variant) -> void:
	if not stored is Dictionary:
		return
	for action in (stored as Dictionary):
		if InputMap.has_action(str(action)) and stored[action] is Array:
			var valid: Array = []
			for code in stored[action]:
				if int(code) != 0:
					valid.append(int(code))
			if not valid.is_empty():
				_set_codes(str(action), valid)
