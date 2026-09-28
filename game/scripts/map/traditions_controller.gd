class_name TraditionsController
extends Node

## TW2-T5 : traditions d'armée. Un bouton « Traditions » dans l'en-tête du bandeau d'armée (armée
## du joueur) ouvre un panneau latéral : rang, expérience, traditions choisies et leurs effets, puis
## une ligne par branche (marche, intendance, tir, assaut, tenue) avec la prochaine tradition de la
## branche ; un clic la choisit (`choose_army_tradition`). Le cœur décide de tout (xp, rangs,
## paliers, refus) : ce contrôleur affiche `CampaignSim.get_army_traditions(armée)` et, après une
## bataille ou une fin de tour, signale par un toast chaque armée qui a un rang à dépenser
## (`get_armies_with_pending_traditions`). `campaign_map` appelle `setup`, `refresh` et
## `after_end_turn` ; `hud_controller.show_army` appelle `on_army_shown`.

const PANEL_WIDTH := 380.0

var map: Node = null  # CampaignMap
var panel: PanelContainer
var button: Button
## Armée affichée dans le panneau (vide = fermé).
var army_id: String = ""
## Armée du joueur sélectionnée dans le bandeau (bouton visible).
var _selected: String = ""
## Rang déjà signalé par armée (un toast par rang atteint).
var _notified: Dictionary = {}
var _box: VBoxContainer


func setup(campaign_map: Node) -> void:
	map = campaign_map
	panel = PanelContainer.new()
	panel.name = "TraditionsPanel"
	panel.add_theme_stylebox_override("panel", HudStyle.panel_box(10))
	panel.custom_minimum_size = Vector2(PANEL_WIDTH, 0)
	panel.hide()
	UiZones.put(UiZones.Zone.SIDE_PANEL, panel)
	map.ui.register_panel(panel, PanelStack.Kind.CENTRAL)
	var scroll := ScrollContainer.new()
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	panel.add_child(scroll)
	_box = VBoxContainer.new()
	_box.add_theme_constant_override("separation", 6)
	_box.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	scroll.add_child(_box)
	panel.visibility_changed.connect(func() -> void:
		if not panel.visible:
			army_id = "")
	button = RichButton.new()
	button.name = "TraditionsButton"
	button.text = "Traditions"
	UiType.apply(button, UiType.CAPTION)
	button.tooltip_text = "Traditions de l'armée : expérience, rangs et choix"
	button.visible = false
	button.pressed.connect(func() -> void: open_for(_selected))
	var strip: Node = map.ui.get("army_strip")
	if strip != null and strip.has_method("add_header_button"):
		strip.call("add_header_button", button)


func available() -> bool:
	return map != null and map.sim != null and map.sim.has_method("get_army_traditions")


func traditions_of(id: String) -> Dictionary:
	return map.sim.call("get_army_traditions", id) if available() and id != "" else {}


## Le bandeau montre `id` : bouton pour une armée du joueur (« Traditions (1) » s'il y a un choix).
func on_army_shown(id: String, is_player: bool) -> void:
	_selected = id if is_player else ""
	if button == null:
		return
	var info := traditions_of(_selected)
	button.visible = not info.is_empty()
	var pending := int(info.get("pending", 0))
	button.text = "Traditions (%d)" % pending if pending > 0 else "Traditions"
	if panel.visible and army_id != "" and army_id != id:
		if is_player:
			open_for(id)
		else:
			panel.hide()


## Titre du bandeau : « Ost d'Édouard III » gardé quand le chef change ; vide sinon.
func kept_title(id: String) -> String:
	var info := traditions_of(id)
	if not bool(info.get("named", false)):
		return ""
	var name := str(info.get("name", ""))
	return "Ost" + name.substr(5) if name.begins_with("L'ost") else name


func open_for(id: String) -> void:
	var info := traditions_of(id)
	if info.is_empty():
		panel.hide()
		return
	army_id = id
	_fill(info)
	panel.show()
	panel.reset_size()


## Prochaine tradition de chaque branche (le palier le plus bas non choisi), dans l'ordre des données.
static func next_by_branch(options: Array) -> Array:
	var best: Dictionary = {}
	var order: Array = []
	for option in options:
		var entry := option as Dictionary
		if bool(entry.get("chosen", false)):
			continue
		var branch := str(entry.get("branch", ""))
		if not best.has(branch):
			order.append(branch)
			best[branch] = entry
		elif int(entry.get("tier", 0)) < int((best[branch] as Dictionary).get("tier", 0)):
			best[branch] = entry
	var result: Array = []
	for branch in order:
		result.append(best[branch])
	return result


## « Rang 1 sur 4 — 25 / 60 d'expérience ».
static func rank_line(info: Dictionary) -> String:
	var text := "Rang %d sur %d — %d" % [int(info.get("rank", 0)), int(info.get("max_rank", 0)), int(info.get("xp", 0))]
	var next := int(info.get("next_threshold", -1))
	text += " / %d d'expérience" % next if next >= 0 else " d'expérience (rang maximal)"
	return text


func _fill(info: Dictionary) -> void:
	for child in _box.get_children():
		_box.remove_child(child)
		child.queue_free()
	var header := HBoxContainer.new()
	header.add_theme_constant_override("separation", 8)
	_box.add_child(header)
	var arms := HouseArms.texture(str(info.get("banner_house", "")))
	if arms != null:
		var banner := TextureRect.new()
		banner.name = "Banner"
		banner.texture = arms
		banner.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
		banner.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
		banner.custom_minimum_size = Vector2(36, 42)
		banner.tooltip_text = HouseArms.tooltip(str(info.get("banner_house", "")))
		header.add_child(banner)
	var titles := VBoxContainer.new()
	titles.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	header.add_child(titles)
	titles.add_child(HudStyle.label("Traditions de l'armée", UiType.size(UiType.CAPTION), HudStyle.INK_SOFT))
	var heading := HudStyle.label(str(info.get("name", "")), UiType.size(UiType.HEADING), HudStyle.RUBRIC)
	heading.name = "Heading"
	heading.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	titles.add_child(heading)
	var close := Button.new()
	close.text = "×"
	close.tooltip_text = "Fermer (Échap)"
	close.pressed.connect(panel.hide)
	header.add_child(close)

	var rank := HudStyle.label(rank_line(info), UiType.size(UiType.BODY), HudStyle.INK)
	rank.name = "RankLine"
	_box.add_child(rank)
	var bar := ProgressBar.new()
	bar.show_percentage = false
	bar.custom_minimum_size = Vector2(0, 8)
	var next := int(info.get("next_threshold", -1))
	bar.max_value = maxf(1.0, float(next if next > 0 else int(info.get("xp", 0))))
	bar.value = float(info.get("xp", 0))
	_box.add_child(bar)
	var chosen: PackedStringArray = info.get("chosen", PackedStringArray())
	var chosen_text := "Aucune tradition encore : l'armée en gagne une par rang, en livrant bataille." if chosen.is_empty() \
		else "Traditions : %s." % ", ".join(chosen)
	var chosen_label := HudStyle.label(chosen_text, UiType.size(UiType.CAPTION), HudStyle.INK_SOFT)
	chosen_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_box.add_child(chosen_label)
	var effects := str(info.get("effects_text", ""))
	if effects != "":
		var effects_label := HudStyle.label("Effets : %s." % effects, UiType.size(UiType.CAPTION), HudStyle.GOOD)
		effects_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		_box.add_child(effects_label)

	var pending := int(info.get("pending", 0))
	var prompt := "Choisissez une tradition (%s) :" % FrText.count(pending, "choix", "choix") if pending > 0 \
		else "Prochaines traditions (au prochain rang) :"
	var prompt_label := HudStyle.label(prompt, UiType.size(UiType.BODY), HudStyle.RUBRIC if pending > 0 else HudStyle.INK_SOFT)
	prompt_label.name = "Prompt"
	_box.add_child(prompt_label)
	var rows := VBoxContainer.new()
	rows.name = "Options"
	rows.add_theme_constant_override("separation", 3)
	_box.add_child(rows)
	for option in next_by_branch(info.get("options", [])):
		rows.add_child(_option_row(option))


func _option_row(option: Dictionary) -> Control:
	var choice := RichButton.new()
	choice.name = str(option.get("id", ""))
	choice.alignment = HORIZONTAL_ALIGNMENT_LEFT
	choice.text = "%s — %s (palier %d) : %s" % [str(option.get("branch_name", "")), str(option.get("name", "")),
		int(option.get("tier", 1)), str(option.get("effects_text", ""))]
	choice.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	var tip := str(option.get("description", ""))
	if bool(option.get("allowed", false)):
		choice.tooltip_text = tip
		var id := str(option.get("id", ""))
		choice.pressed.connect(func() -> void: choose(id))
	else:
		choice.disabled = true
		var reason := str(option.get("reason", ""))
		choice.tooltip_text = "%s\nImpossible : %s." % [tip, reason] if reason != "" else tip
	return choice


## Envoie le choix au cœur ; toast du résultat, panneau et bandeau rafraîchis.
func choose(tradition: String) -> bool:
	if not available() or army_id == "":
		return false
	var result: Dictionary = map.sim.call("choose_army_tradition", army_id, tradition)
	var ok := bool(result.get("ok", false))
	if ok:
		var info := traditions_of(army_id)
		var name := tradition
		for option in info.get("options", []):
			if str((option as Dictionary).get("id", "")) == tradition:
				name = str(option.get("name", tradition))
		map.ui.show_toast("Tradition adoptée : %s." % name)
		_fill(info)
		on_army_shown(_selected, _selected != "")
	else:
		map.ui.show_toast(str(result.get("error", "Choix impossible")), true)
	return ok


## Armées du joueur qui viennent d'atteindre un rang (une seule fois par rang) : toast.
func notify_new_ranks() -> Array:
	var notified: Array = []
	if not available() or not map.sim.has_method("get_armies_with_pending_traditions"):
		return notified
	for entry in map.sim.call("get_armies_with_pending_traditions"):
		var id := str((entry as Dictionary).get("army", ""))
		var rank := int(traditions_of(id).get("rank", 0))
		if int(_notified.get(id, 0)) >= rank:
			continue
		_notified[id] = rank
		notified.append(id)
	if notified.size() == 1:
		var info := traditions_of(str(notified[0]))
		map.ui.show_toast("%s atteint le rang %d : une tradition d'armée est à choisir." % [str(info.get("name", "Une armée")), int(info.get("rank", 0))])
	elif notified.size() > 1:
		map.ui.show_toast("%d armées peuvent choisir une tradition d'armée." % notified.size())
	return notified


## Après tout changement d'état (bataille 3D ou auto, assaut) : panneau, bouton et toasts.
func refresh() -> void:
	if not available():
		return
	if panel.visible and army_id != "":
		var info := traditions_of(army_id)
		if info.is_empty():
			panel.hide()  # armée dissoute ou détruite : ses traditions avec elle
		else:
			_fill(info)
	if _selected != "":
		on_army_shown(_selected, true)
	notify_new_ranks()


func after_end_turn() -> void:
	refresh()
