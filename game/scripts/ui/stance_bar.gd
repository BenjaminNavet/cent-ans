class_name StanceBar
extends HBoxContainer

## Lot CV3-4 : rangée de boutons de posture du sceau du chef (une icône à l'encre par posture :
## normale, chevauchée, siège, embuscade, marche forcée, camp retranché). La posture en cours
## est enfoncée ; une posture que le cœur refuse est grisée, sa raison en infobulle.
## Aucune règle : `set_state(stance, options)` reçoit la posture de `get_army` et
## `CampaignSim.get_stance_options(army)` (posture → "" si permise, sinon raison en français).
## Libellés et bulles d'aide : `RichTooltip.HUD_TEXTS["hud_stance_<clé>"]`.

## Posture choisie (bouton d'une autre posture, permise).
signal stance_selected(stance: String)

## Ordre d'affichage des clés d'ordre `set_stance` du cœur.
const STANCES := ["normal", "raid", "siege", "ambush", "forced_march", "entrenched"]
const BUTTON_SIZE := Vector2(30, 28)
const ICON_SIZE := 20

var current: String = "normal"
var options: Dictionary = {}
var _buttons: Dictionary = {}  # posture → RichButton


func _ready() -> void:
	name = "StanceBar"
	add_theme_constant_override("separation", 2)
	mouse_filter = Control.MOUSE_FILTER_PASS
	for stance: String in STANCES:
		var button := RichButton.new()
		button.name = "Stance_" + stance
		button.toggle_mode = true
		button.focus_mode = Control.FOCUS_NONE
		button.custom_minimum_size = BUTTON_SIZE
		button.mouse_default_cursor_shape = Control.CURSOR_POINTING_HAND
		button.set_meta("stance", stance)
		var library := RichTooltip.icons()
		if library != null:
			library.call("decorate_button", button, icon_id(stance), ICON_SIZE)
		button.icon_alignment = HORIZONTAL_ALIGNMENT_CENTER
		button.pressed.connect(_on_pressed.bind(stance))
		add_child(button)
		_buttons[stance] = button
	_apply()


## Identifiant d'icône (et de bulle) d'une posture.
static func icon_id(stance: String) -> String:
	return "hud_stance_" + stance


## Nom français d'une posture (bulle d'aide `RichTooltip.HUD_TEXTS`).
static func stance_name(stance: String) -> String:
	var spec: Array = RichTooltip.HUD_TEXTS.get(icon_id(stance), [stance.capitalize(), ""])
	return str(spec[0])


## Posture en cours et disponibilités du cœur (vide : toutes proposées, le cœur tranchera).
func set_state(stance: String, stance_options: Dictionary) -> void:
	current = stance if stance != "" else "normal"
	options = stance_options.duplicate()
	_apply()


func button(stance: String) -> Button:
	return _buttons.get(stance)


## Raison du refus du cœur ("" si la posture est permise ou inconnue du cœur).
func reason(stance: String) -> String:
	return str(options.get(stance, ""))


func _apply() -> void:
	for stance: String in _buttons:
		var node: RichButton = _buttons[stance]
		var is_current := stance == current
		var refusal := "" if is_current else reason(stance)
		node.set_pressed_no_signal(is_current)
		node.disabled = refusal != ""
		var extra := ""
		if is_current:
			extra = "[color=#2f5a1c]Posture actuelle.[/color]"
		elif refusal != "":
			extra = "[color=#9e2114]Impossible : %s[/color]" % refusal
		else:
			extra = "[i]Clic : adopter cette posture.[/i]"
		node.tooltip_text = RichTooltip.hud(icon_id(stance), extra)


func _on_pressed(stance: String) -> void:
	if stance == current:
		_buttons[stance].set_pressed_no_signal(true)
		return
	_buttons[stance].set_pressed_no_signal(false)  # enfoncé au retour du cœur (rafraîchissement)
	if reason(stance) == "":
		stance_selected.emit(stance)
