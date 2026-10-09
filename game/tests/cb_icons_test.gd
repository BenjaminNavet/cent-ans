extends TestCase

## Lot CB (icônes des contrôles de bataille, pipeline DA5) :
## 1. chaque clé lue par les scripts de bataille (`battle_mode_<mode>`, `battle_state_<pastille>`,
##    `battle_alert_<kind>`, `battle_ability_<kind>`, `battle_lock`) a son PNG à l'encre, que
##    `IconLibrary` charge ;
## 2. les six curseurs `res://assets/ui/cursors/<contexte>.png` existent, font 32 px et sont repris
##    par `BattleCursor` ;
## 3. repli : sans l'icône (clé retirée de l'index), `draw_ink_icon` renvoie faux et les glyphes
##    dessinés en code prennent le relais sans erreur ; le substitut de curseur reste constructible.
##
## Usage : godot --headless --path game --script res://tests/cb_icons_test.gd

const MODES := ["run", "guard", "skirmish", "melee", "breach"]
const STATES := ["rout", "wavering", "under_fire", "charge", "melee", "shoot", "tired"]
const ALERTS := ["rout", "general_down", "flanked", "reinforcements", "ammo_out", "wall_breached", "gate_destroyed"]
const ABILITIES := ["aimed_shot", "pavise", "banner_rally", "close_ranks", "planted_pikes"]

var _drawn := {}


func _init() -> void:
	await process_frame
	_check_keys()
	_check_cursors()
	await _check_fallback()
	finish()


static func expected_keys() -> Array[String]:
	var keys: Array[String] = []
	for mode in MODES:
		keys.append("battle_mode_" + mode)
	for state in STATES:
		keys.append("battle_state_" + state)
	for alert in ALERTS:
		keys.append("battle_alert_" + alert)
	for ability in ABILITIES:
		keys.append("battle_ability_" + ability)
	keys.append("battle_lock")
	return keys


func _check_keys() -> void:
	var library: Node = root.get_node_or_null("/root/IconLibrary")
	if not check(library != null, "IconLibrary autoload missing"):
		return
	for key in expected_keys():
		check(bool(library.call("is_ink", key)), "%s has no ink icon in ink/index.json" % key)
		var texture := HudStyle.icon(key)
		if check(texture != null, "%s does not load" % key):
			check(texture.get_width() == 128 and texture.get_height() == 128, "%s is not 128 px" % key)
	var ink: Dictionary = library.get("ink")
	check(not ink.has("order_pavise"), "order_pavise still claimed by an ink icon")


func _check_cursors() -> void:
	for context in BattleCursor.CONTEXTS:
		check(BattleCursor.has_final_art(context), "cursor %s has no PNG" % context)
		var image := BattleCursor.image(context)
		if check(image != null, "cursor %s does not load" % context):
			check(image.get_size() == Vector2i(BattleCursor.SIZE, BattleCursor.SIZE), "cursor %s is not 32 px" % context)
			check(image.get_pixel(16, 16).a > 0.5 or context == "move", "cursor %s is empty at its hotspot" % context)
	var placeholder := BattleCursor.placeholder("move")
	check(placeholder != null and placeholder.get_size() == Vector2i(32, 32), "cursor placeholder broken")


func _check_fallback() -> void:
	var library: Node = root.get_node_or_null("/root/IconLibrary")
	if library == null:
		return
	var ink: Dictionary = library.get("ink")
	var removed := {}
	for key in ["battle_mode_run", "battle_state_rout", "battle_alert_flanked", "battle_ability_pavise", "battle_lock"]:
		if ink.has(key):
			removed[key] = ink[key]
			ink.erase(key)
	var canvas := Control.new()
	canvas.size = Vector2(64, 64)
	canvas.draw.connect(func() -> void:
		_drawn["present"] = BattleModeIcons.draw_ink_icon(canvas, "battle_mode_guard", Vector2(16, 16), 16.0)
		_drawn["absent"] = BattleModeIcons.draw_ink_icon(canvas, "battle_mode_run", Vector2(16, 16), 16.0)
		BattleModeIcons.draw_mode(canvas, "run", Vector2(16, 16), 1.0)
		BattleUnitMarkers.draw_badge(canvas, "rout", Vector2(30, 30), true)
		BattleAbilityIcons.draw_ability(canvas, "pavise", Vector2(40, 16), 1.0)
		UnitCard.draw_padlock(canvas, Vector2(40, 40), 1.0)
		_drawn["done"] = true)
	root.add_child(canvas)
	canvas.queue_redraw()
	for i in 4:
		await process_frame
	check(bool(_drawn.get("done", false)), "fallback canvas never drew")
	check(bool(_drawn.get("present", false)), "present icon not drawn")
	check(not bool(_drawn.get("absent", true)), "missing icon should fall back to the glyph")
	for key in removed:
		ink[key] = removed[key]
	canvas.queue_free()
