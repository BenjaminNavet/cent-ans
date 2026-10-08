class_name BattlesMenu
extends ListMenu

## A6-L11 (U1) — sous-menu « Batailles » du menu principal : regroupe la bataille personnalisée,
## les batailles historiques, les batailles de démonstration et les rejeux. Le choix est émis par
## le signal `chosen(mode)` (`custom`, `historical`, `demos`, `replays`) ; le menu principal ferme
## ce sous-menu et ouvre l'écran voulu. Échap ou « Fermer » : `closed`. Aucune règle ici.

signal chosen(mode: String)

const ENTRIES := [
	["custom", "Bataille personnalisée"],
	["historical", "Batailles historiques"],
	["demos", "Batailles de démonstration"],
	["replays", "Rejeux"],
]



func _menu_title() -> String:
	return "Batailles"


func _menu_width() -> float:
	return 480.0


func _build_entries(box: VBoxContainer) -> void:
	for entry: Array in ENTRIES:
		var mode := str(entry[0])
		var button := UiBuild.button(str(entry[1]), func() -> void: chosen.emit(mode), box)
		button.name = "Battles_" + mode
		button.custom_minimum_size = Vector2(0, 40)
		buttons[mode] = button
