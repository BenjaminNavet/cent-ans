class_name ListMenu
extends PanelContainer

## Base des menus de liste du menu principal (Batailles, historiques, démonstrations, rejeux) :
## fenêtre parchemin de la zone `MODAL`, titre, aide facultative, entrées fournies par la classe
## fille, bouton « Fermer », Échap pour fermer. Patron de méthode : la fille surcharge
## `_menu_title`, `_menu_hint`, `_build_entries` et réglages de taille ; elle ne garde que la
## source de ses entrées et l'action associée. `buttons` recense les boutons d'entrée (tests).

signal closed

const BATTLE_SCENE := "res://scenes/battle/battle.tscn"

## Boutons d'entrée, par clé propre à la fille.
var buttons: Dictionary = {}
var close_button: Button = null


func _ready() -> void:
	theme = load("res://scenes/ui/parchment_theme.tres")
	mouse_filter = Control.MOUSE_FILTER_STOP
	custom_minimum_size = Vector2(_menu_width(), 0)
	_load_entries()
	var box: VBoxContainer
	if _scroll_height() > 0.0:
		var scroll := ScrollContainer.new()
		scroll.custom_minimum_size = Vector2(_menu_width() - 20.0, _scroll_height())
		scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
		add_child(scroll)
		box = UiBuild.vbox(8, scroll)
		box.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	else:
		box = UiBuild.vbox(8, self)
	var title := UiBuild.label(_menu_title())
	UiType.apply(title, UiType.TITLE)
	box.add_child(title)
	if _menu_hint() != "":
		var hint := UiBuild.label(_menu_hint(), 0, null, true, _menu_width() - 60.0)
		UiType.apply(hint, UiType.CAPTION)
		hint.modulate = Color(1, 1, 1, 0.75)
		box.add_child(hint)
	_build_entries(box)
	var row := UiBuild.hbox(8, box)
	row.alignment = BoxContainer.ALIGNMENT_END
	close_button = UiBuild.button("Fermer", close, row)
	close_button.name = "CloseButton"
	if buttons.is_empty():
		close_button.grab_focus.call_deferred()
	else:
		(buttons.values()[0] as Button).grab_focus.call_deferred()
	UiZones.put(UiZones.Zone.MODAL, self)
	UiMotion.fade_in(self)


# --- Points d'extension ----------------------------------------------------------------------


func _menu_title() -> String:
	return ""


## Texte d'aide sous le titre (`""` : aucun).
func _menu_hint() -> String:
	return ""


func _menu_width() -> float:
	return 680.0


## Hauteur de la zone défilante (0 : pas de défilement).
func _scroll_height() -> float:
	return 0.0


## Lit la source des entrées ; appelé avant la construction.
func _load_entries() -> void:
	pass


## Ajoute les entrées à `box` et enregistre leurs boutons dans `buttons`.
func _build_entries(_box: VBoxContainer) -> void:
	pass


# --- Comportement commun ---------------------------------------------------------------------


func _unhandled_input(event: InputEvent) -> void:
	if visible and event.is_action_pressed("ui_cancel"):
		close()
		get_viewport().set_input_as_handled()


func close() -> void:
	closed.emit()
	UiMotion.fade_out(self, UiMotion.DURATION, true)


## Lance la scène de bataille avec les options `args` (musique du menu coupée).
func _launch_battle(args: PackedStringArray) -> void:
	BattleScene.demo_args = args
	var audio := get_node_or_null("/root/AudioDirector")
	if audio != null and audio.has_method("stop_all"):
		audio.call("stop_all")
	SceneFader.go(BATTLE_SCENE)

