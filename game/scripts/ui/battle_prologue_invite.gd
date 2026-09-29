class_name BattlePrologueInvite
extends PanelContainer

## NT4 — invite au premier lancement d'une campagne ou d'une bataille depuis le menu :
## « Jouer le didacticiel de bataille ? » Oui (lance `BattlePrologue`), Non (poursuit le
## lancement demandé, l'invite reviendra), Ne plus demander (poursuit et mémorise le choix,
## réglage `battle_prologue/never_ask`). Plus d'invite non plus une fois le didacticiel terminé
## (`battle_prologue/done`). Zone `MODAL` de `UiLayout`, comme les autres fenêtres du menu.

signal chosen(choice: String)  # "yes", "no", "never"

const NEVER_KEY := "battle_prologue/never_ask"
const DONE_KEY := "battle_prologue/done"

var yes_button: Button
var no_button: Button
var never_button: Button
## Suite du lancement interrompu (appelée sur « Non » et « Ne plus demander »).
var proceed: Callable = Callable()


## L'invite doit-elle s'afficher (didacticiel présent, ni terminé ni refusé pour de bon) ?
static func should_ask() -> bool:
	var tree := Engine.get_main_loop() as SceneTree
	var settings: Node = tree.root.get_node_or_null("/root/Settings") if tree != null else null
	if settings == null or not ClassDB.class_exists("BattleSim"):
		return false
	if bool(settings.call("get_value", NEVER_KEY)) or bool(settings.call("get_value", DONE_KEY)):
		return false
	return not BattlePrologue.load_data().is_empty()


## Appelle `p_proceed` tout de suite, ou après l'invite si elle doit s'afficher ; renvoie
## l'invite ouverte (null sinon).
static func gate(host: Node, p_proceed: Callable) -> BattlePrologueInvite:
	if not should_ask():
		p_proceed.call()
		return null
	var invite := BattlePrologueInvite.new()
	invite.proceed = p_proceed
	host.add_child(invite)
	return invite


func _ready() -> void:
	name = "BattlePrologueInvite"
	theme = load("res://scenes/ui/parchment_theme.tres")
	mouse_filter = Control.MOUSE_FILTER_STOP
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 12)
	add_child(box)
	var title := Label.new()
	title.text = "Jouer le didacticiel de bataille ?"
	UiType.apply(title, UiType.HEADING)
	box.add_child(title)
	var text := Label.new()
	text.text = "Une courte escarmouche guidée de 1337 apprend à mener une bataille : caméra, ordres, formation, charge, tir et pause tactique. On la retrouve dans « Batailles historiques »."
	text.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	text.custom_minimum_size = Vector2(520, 0)
	UiType.apply(text, UiType.BODY)
	box.add_child(text)
	var row := HBoxContainer.new()
	row.alignment = BoxContainer.ALIGNMENT_END
	row.add_theme_constant_override("separation", 10)
	box.add_child(row)
	never_button = _button(row, "NeverButton", "Ne plus demander", "never")
	no_button = _button(row, "NoButton", "Non", "no")
	yes_button = _button(row, "YesButton", "Oui", "yes")
	yes_button.grab_focus.call_deferred()
	UiZones.put(UiZones.Zone.MODAL, self)
	UiMotion.fade_in(self)


func _button(row: HBoxContainer, node_name: String, text: String, choice: String) -> Button:
	var button := Button.new()
	button.name = node_name
	button.text = text
	button.pressed.connect(choose.bind(choice))
	row.add_child(button)
	return button


func _unhandled_input(event: InputEvent) -> void:
	if visible and event.is_action_pressed("ui_cancel"):
		get_viewport().set_input_as_handled()
		queue_free()  # Échap : ni lancement ni choix mémorisé


## Applique le choix : « yes » lance le didacticiel, sinon on poursuit le lancement demandé.
func choose(choice: String) -> void:
	if choice == "never":
		var settings := get_node_or_null("/root/Settings")
		if settings != null:
			settings.call("set_value", NEVER_KEY, true)
	chosen.emit(choice)
	queue_free()
	if choice == "yes":
		BattlePrologue.launch(get_tree())
	elif proceed.is_valid():
		proceed.call()
