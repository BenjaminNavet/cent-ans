class_name BuildCompanion
extends VBoxContainer

## RG5 : vue des bâtiments d'une colonie à côté d'une autre fenêtre (panneau de province, liste
## « Colonies »), pour bâtir sans quitter la fenêtre courante. Un sélecteur liste les colonies du
## joueur ; changer de colonie recharge la vue. Les données viennent du `provider`
## (`SettlementController.fill_companion_view`), les ordres sortent en signaux. Aucune règle ici.

signal build_requested(settlement_id: String, building_id: String)
signal raze_requested(settlement_id: String, building_id: String, preview: Dictionary)
signal cancel_requested(settlement_id: String, index: int)
signal settlement_changed(settlement_id: String)
signal closed

var settlement_id: String = ""
var provider: Callable = Callable()
var title_label: Label
var selector: OptionButton
var view: SettlementBuildingsView
var _ids: Array = []


func _init() -> void:
	name = "BuildCompanion"
	size_flags_horizontal = Control.SIZE_EXPAND_FILL
	add_theme_constant_override("separation", 4)
	var head := HBoxContainer.new()
	add_child(head)
	title_label = HudStyle.label("Bâtir", HudStyle.FONT_BODY, HudStyle.INK)
	title_label.name = "Title"
	head.add_child(title_label)
	selector = OptionButton.new()
	selector.name = "Selector"
	selector.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	selector.fit_to_longest_item = false
	selector.item_selected.connect(func(index: int) -> void: show_settlement(str(_ids[index])))
	head.add_child(selector)
	var close := Button.new()
	close.name = "CloseCompanion"
	close.text = "✕"
	close.focus_mode = Control.FOCUS_NONE
	close.pressed.connect(func() -> void:
		hide()
		closed.emit())
	head.add_child(close)
	view = SettlementBuildingsView.new()
	view.show_cancel = true
	view.build_requested.connect(func(id: String, building_id: String) -> void: build_requested.emit(id, building_id))
	view.raze_requested.connect(func(id: String, building_id: String, preview: Dictionary) -> void: raze_requested.emit(id, building_id, preview))
	view.cancel_requested.connect(func(id: String, index: int) -> void: cancel_requested.emit(id, index))
	add_child(view)
	hide()


## `rows` : colonies proposées (`id`, `name`, `can_build`, `can_build_reason`) ; seules celles du
## joueur doivent y figurer. Garde la colonie courante si elle y est encore.
func set_choices(rows: Array) -> void:
	_ids.clear()
	selector.clear()
	for row in rows:
		var mark := "⚒ " if bool(row.get("can_build", false)) else ""
		selector.add_item(mark + str(row.get("name", row.get("id", ""))))
		selector.set_item_tooltip(selector.item_count - 1, "" if bool(row.get("can_build", false)) else str(row.get("can_build_reason", "")))
		_ids.append(str(row.get("id", "")))
	var index := _ids.find(settlement_id)
	if index >= 0:
		selector.select(index)
	elif not _ids.is_empty():
		selector.select(0)
		settlement_id = str(_ids[0])
	else:
		settlement_id = ""
		hide()


## Passe la vue sur la colonie `id` (parmi les choix) et la montre.
func show_settlement(id: String) -> void:
	var index := _ids.find(id)
	if index >= 0:
		selector.select(index)
	settlement_id = id
	title_label.text = "Bâtir à"
	if provider.is_valid():
		provider.call(view, id)
	show()
	settlement_changed.emit(id)


## Recharge la colonie courante (après un ordre ou un tour).
func refresh() -> void:
	if settlement_id != "" and visible and provider.is_valid():
		provider.call(view, settlement_id)
