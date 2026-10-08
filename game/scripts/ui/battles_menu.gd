class_name BattlesMenu
extends PanelContainer

## A6-L11 (U1) — sous-menu « Batailles » du menu principal : regroupe la bataille personnalisée,
## les batailles historiques, les batailles de démonstration et les rejeux. Le choix est émis par
## le signal `chosen(mode)` (`custom`, `historical`, `demos`, `replays`) ; le menu principal ferme
## ce sous-menu et ouvre l'écran voulu. Échap ou « Fermer » : `closed`. Aucune règle ici.

signal closed
signal chosen(mode: String)

const ENTRIES := [
	["custom", "Bataille personnalisée"],
	["historical", "Batailles historiques"],
	["demos", "Batailles de démonstration"],
	["replays", "Rejeux"],
]

var buttons: Dictionary = {}  # mode -> Button (tests)


func _ready() -> void:
	theme = load("res://scenes/ui/parchment_theme.tres")
	mouse_filter = Control.MOUSE_FILTER_STOP
	custom_minimum_size = Vector2(480, 0)
	var box := UiBuild.vbox(8)
	add_child(box)
	var title := UiBuild.label("Batailles")
	UiType.apply(title, UiType.TITLE)
	box.add_child(title)
	for entry: Array in ENTRIES:
		var mode := str(entry[0])
		var button := UiBuild.button(str(entry[1]), func() -> void: chosen.emit(mode))
		button.name = "Battles_" + mode
		button.custom_minimum_size = Vector2(0, 40)
		box.add_child(button)
		buttons[mode] = button
	var close_button := UiBuild.button("Fermer", close)
	close_button.name = "CloseButton"
	box.add_child(close_button)
	(buttons["custom"] as Button).grab_focus.call_deferred()
	UiZones.put(UiZones.Zone.MODAL, self)
	UiMotion.fade_in(self)


func _unhandled_input(event: InputEvent) -> void:
	if visible and event.is_action_pressed("ui_cancel"):
		close()
		get_viewport().set_input_as_handled()


func close() -> void:
	closed.emit()
	UiMotion.fade_out(self, UiMotion.DURATION, true)
