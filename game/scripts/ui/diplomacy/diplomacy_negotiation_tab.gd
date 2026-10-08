class_name DiplomacyNegotiationTab
extends DiplomacyView

## Onglet « Négociation » : brouillon de traité (clauses communes, colonnes « Vous offrez » /
## « Vous demandez »), barre d'acceptation en direct, leur raisonnement ligne à ligne et la
## contre-offre quand un seul point bloque (DP2), « Que faudrait-il ? », puis les actions
## unilatérales (`DiplomacyActionsSection`). Aucune règle : `treaty_options`, `evaluate_treaty`,
## `explain_treaty`, `counter_treaty` viennent de `CampaignSim`.

const GOLD_STEPS := [500, 1000, 2500, 5000, 10000, 20000]
const TRIBUTE_STEPS := [100, 250, 500, 1000]
const TRIBUTE_SEASONS := 8
const TRUCE_TURNS := 8

## Clauses du brouillon ({kind, giver?, …}) et menu d'ajout des clauses communes.
var articles: Array = []
var clause_menu: MenuButton
var chance_label: Label
## Boutons « Que faudrait-il ? » / « Effacer » / « Proposer le traité » : le panneau les place
## hors de la page défilante pour qu'ils restent atteignables (Q6).
var treaty_buttons: HBoxContainer
var _draft_for: String = ""
var _options: Dictionary = {}
var _verdict: Dictionary = {}
var _clauses: HFlowContainer
var _offer_list: VBoxContainer
var _demand_list: VBoxContainer
var _offer_menu: MenuButton
var _demand_menu: MenuButton
var _chance_bar: ProgressBar
var _reasons: RichTextLabel
var _counter_box: HBoxContainer
var _counter_label: Label
var _counter_articles: Array = []
var _explanation: Dictionary = {}
var _actions: DiplomacyActionsSection


func _init() -> void:
	super("Negotiation", 6)
	_build()
	_actions = DiplomacyActionsSection.new()
	_actions.order_requested.connect(order_requested.emit)
	add_child(_actions)


## Nouvelle faction : brouillon de départ (paix si en guerre) ; puis brouillon et actions.
func _render() -> void:
	if _draft_for != faction_id:
		_draft_for = faction_id
		articles = [{"kind": "peace"}] if str(entry.get("status", "")) == "war" else []
	_options = sim.call("treaty_options", faction_id) if sim.has_method("treaty_options") else {}
	render_draft()
	_actions.show_for(sim, player_faction, faction_id, entry)


func _build() -> void:
	var clause_row := HBoxContainer.new()
	clause_row.add_child(_label("Clauses communes", UiType.HEADING, HudStyle.INK))
	var spacer := Control.new()
	spacer.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	clause_row.add_child(spacer)
	clause_menu = _add_menu("+ Clause")
	clause_row.add_child(clause_menu)
	add_child(clause_row)
	_clauses = HFlowContainer.new()
	_clauses.add_theme_constant_override("h_separation", 6)
	add_child(_clauses)
	var columns := UiBuild.hbox(10, self)
	var offer := _article_column("Vous offrez")
	_offer_menu = offer[1]
	_offer_list = offer[2]
	columns.add_child(offer[0])
	var demand := _article_column("Vous demandez")
	_demand_menu = demand[1]
	_demand_list = demand[2]
	columns.add_child(demand[0])
	add_child(_rule())
	var chance_row := UiBuild.hbox(8)
	chance_label = _label("", UiType.HEADING, HudStyle.INK)
	chance_label.custom_minimum_size = Vector2(150, 0)
	chance_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	chance_row.add_child(chance_label)
	_chance_bar = ProgressBar.new()
	_chance_bar.min_value = 0
	_chance_bar.max_value = 100
	_chance_bar.show_percentage = false
	_chance_bar.custom_minimum_size = Vector2(0, 18)
	_chance_bar.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_chance_bar.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	chance_row.add_child(_chance_bar)
	add_child(chance_row)
	_reasons = RichTextLabel.new()
	_reasons.bbcode_enabled = true
	_reasons.fit_content = true
	_reasons.scroll_active = false
	_counter_box = UiBuild.hbox(8)
	_counter_box.name = "CounterOffer"
	_counter_label = _label("", UiType.BODY, HudStyle.INK)
	_counter_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_counter_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_counter_box.add_child(_counter_label)
	var adopt := UiBuild.button("Reprendre leur contre-offre")
	adopt.name = "AdoptCounter"
	TooltipHost.attach_plain(adopt, "treaty_adopt_counter")
	adopt.focus_mode = Control.FOCUS_NONE
	adopt.pressed.connect(_adopt_counter)
	_counter_box.add_child(adopt)
	_counter_box.hide()
	add_child(_counter_box)
	add_child(_reasons)
	_codex_labels.append(_reasons)  # BP1 : mots du Codex cliquables dans les motifs
	var buttons := UiBuild.hbox(6)
	var counter := UiBuild.button("Que faudrait-il ?")
	TooltipHost.attach_plain(counter, "treaty_ask_counter")
	counter.pressed.connect(ask_counter)
	buttons.add_child(counter)
	var clear := UiBuild.button("Effacer")
	clear.pressed.connect(func() -> void:
		articles = []
		render_draft())
	buttons.add_child(clear)
	var send := UiBuild.button("Proposer le traité")
	send.name = "SendTreaty"
	send.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	# FA5 : sceau de cire réel sur le bouton qui engage la parole du prince.
	send.icon = FaUi.seal("treaty")
	send.expand_icon = false
	send.add_theme_constant_override("icon_max_width", TREATY_SEAL_SIZE)
	send.pressed.connect(_send_treaty)
	buttons.add_child(send)
	treaty_buttons = buttons  # Q6 : hors de la page défilante (placé par le panneau)


## [colonne, menu d'ajout, liste des articles].
func _article_column(title: String) -> Array:
	var box := PanelContainer.new()
	box.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	box.add_theme_stylebox_override("panel", HudStyle.card_box(HudStyle.PARCHMENT_LIGHT, HudStyle.GOLD, 1))
	var column := UiBuild.vbox(4, box)
	var row := HBoxContainer.new()
	var label := _label(title, UiType.HEADING, HudStyle.RUBRIC)
	label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	# U15 : le titre passe à la ligne plutôt que d'élargir la colonne (le menu sortait du cadre).
	label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	label.custom_minimum_size = Vector2(40, 0)
	row.add_child(label)
	var menu := _add_menu("+ Ajouter")
	row.add_child(menu)
	column.add_child(row)
	var list := UiBuild.vbox(3)
	list.custom_minimum_size = Vector2(0, 90)
	column.add_child(list)
	return [box, menu, list]


func _add_menu(text: String) -> MenuButton:
	var menu := MenuButton.new()
	menu.text = text
	menu.flat = false
	UiType.apply(menu, UiType.BODY)  # PO phase 2 (P2b) : plus de taille ad hoc, variation UiType
	menu.about_to_popup.connect(func() -> void: fill_menu(menu))
	return menu


func render_draft() -> void:
	for box in [_clauses, _offer_list, _demand_list]:
		UiBuild.clear_children(box)
	_verdict = {}
	if sim != null and sim.has_method("evaluate_treaty") and not articles.is_empty():
		_verdict = sim.call("evaluate_treaty", faction_id, articles)
	var values: Array = _verdict.get("articles", [])
	for index in articles.size():
		var article: Dictionary = articles[index]
		var value: Dictionary = values[index] if index < values.size() else {}
		var giver := str(article.get("giver", ""))
		var target: Container = _clauses if giver == "" else (_offer_list if giver == "proposer" else _demand_list)
		target.add_child(_article_row(index, article, value))
	for list in [_offer_list, _demand_list]:
		if list.get_child_count() == 0:
			list.add_child(_label("—", UiType.BODY, HudStyle.INK_FADED))
	if _clauses.get_child_count() == 0:
		_clauses.add_child(_label("Aucune clause commune.", UiType.CAPTION, HudStyle.INK_FADED))
	_render_chance()


func _article_row(index: int, article: Dictionary, value: Dictionary) -> Control:
	var row := UiBuild.hbox(4)
	var label_text := str(value.get("label", _fallback_label(article)))
	var label := _label(label_text, UiType.BODY, HudStyle.INK)
	label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	# U15 : largeur plancher = le plus long mot (jamais de coupure au milieu d'un mot) ; une clause
	# commune, dans un conteneur à retour automatique, garde sa phrase entière jusqu'à 220 px.
	var floor_width := _longest_word_width(label, label_text)
	if str(article.get("giver", "")) == "":
		floor_width = maxf(floor_width, minf(_text_width(label, label_text), 220.0))
	label.custom_minimum_size = Vector2(floor_width, 0)
	var tips := PackedStringArray()
	for reason in value.get("reasons", []):
		tips.append("%+d  %s" % [int(reason["value"]), str(reason["text"])])
	if str(value.get("blocked", "")) != "":
		tips.append("Impossible : %s" % value["blocked"])
	TooltipHost.attach_plain(label, "treaty_clause_detail", {"body": "\n".join(tips)})
	label.mouse_filter = Control.MOUSE_FILTER_STOP
	row.add_child(label)
	if not value.is_empty():
		var points := int(value.get("value", 0))
		var points_label := _label("%+d" % points, UiType.BODY, HudStyle.GOOD if points >= 0 else HudStyle.POOR)
		TooltipHost.attach_plain(points_label, "treaty_clause_value_for_them")
		points_label.mouse_filter = Control.MOUSE_FILTER_STOP
		row.add_child(points_label)
	var remove := UiBuild.button("×")
	TooltipHost.attach_plain(remove, "treaty_clause_remove")
	remove.focus_mode = Control.FOCUS_NONE
	remove.pressed.connect(func() -> void:
		articles.remove_at(index)
		render_draft())
	row.add_child(remove)
	return row


func _text_width(label: Label, text: String) -> float:
	var font := label.get_theme_font("font")
	var font_size := label.get_theme_font_size("font_size")
	return font.get_string_size(text, HORIZONTAL_ALIGNMENT_LEFT, -1, font_size).x + 4.0


func _longest_word_width(label: Label, text: String) -> float:
	var widest := 40.0
	for word in text.split(" ", false):
		widest = maxf(widest, _text_width(label, word))
	return widest


func _fallback_label(article: Dictionary) -> String:
	return str(article.get("kind", "?")).replace("_", " ")


func _render_chance() -> void:
	var fill := StyleBoxFlat.new()
	_counter_box.hide()
	_counter_articles = []
	_explanation = {}
	if articles.is_empty():
		chance_label.text = "Ajoutez des clauses"
		_chance_bar.value = 0
		_reasons.text = ""
		return
	var chance := int(_verdict.get("chance", 0))
	var blocked := str(_verdict.get("blocked", ""))
	_chance_bar.value = chance
	fill.bg_color = HudStyle.gauge_color(chance / 100.0)
	_chance_bar.add_theme_stylebox_override("fill", fill)
	# M6 (ADR 0182) : acceptation déterministe, score signé (accepté ssi score >= 0).
	var accepts := bool(_verdict.get("accept", false))
	chance_label.text = "%s (%+d)" % ["Accepterait" if accepts else "Refuserait", int(_verdict.get("score", 0))]
	if _render_explanation():
		return
	var text := ""
	if blocked != "":
		text += "[color=#8b1a1a][b]Impossible :[/b] %s[/color]\n" % CodexText.format(blocked, true)
	var context: Array = _verdict.get("context", [])
	var parts := PackedStringArray()
	for reason in context:
		var v := int(reason["value"])
		parts.append("[color=%s]%+d[/color] %s" % ["#2a6a2a" if v >= 0 else "#8b1a1a", v, CodexText.format(str(reason["text"]), true)])
	if not parts.is_empty():
		text += "[b]Considérations :[/b] " + " · ".join(parts)
	_reasons.text = text


## Lot DP2 : leur raisonnement ligne à ligne, chaque raison avec son poids (à la Warhammer III :
## « Ils se méfient de vous −12 », « Accord commercial — routes communes +8 »), puis la
## contre-offre quand un seul point bloque. Faux si la simulation ne l'explique pas.
func _render_explanation() -> bool:
	if sim == null or not sim.has_method("explain_treaty"):
		return false
	_explanation = sim.call("explain_treaty", faction_id, articles)
	if not bool(_explanation.get("ok", false)):
		return false
	var blocker: Dictionary = _explanation.get("blocker", {})
	var blocker_text := str(blocker.get("text", ""))
	var accept := bool(_explanation.get("accept", false))
	var text := "[b][color=%s]%s[/color][/b]\n" % ["#2a6a2a" if accept else "#8b1a1a", _explanation.get("summary", "")]
	var lines: Array = _explanation.get("lines", [])
	var shown := 0
	for line in lines:
		var v := int(line["value"])
		var line_text := str(line["text"])
		var entry := "[color=%s]%+d[/color]  %s" % ["#2a6a2a" if v >= 0 else "#8b1a1a", v, line_text]
		if blocker_text != "" and (line_text == blocker_text or line_text.begins_with(blocker_text + " — ")):
			entry = "[b]%s[/b]  ◄" % entry
		text += entry + "\n"
		shown += 1
		if shown >= 14 and lines.size() > 15:
			text += "[color=#6b5a45]… %d autres raisons de moindre poids[/color]\n" % (lines.size() - shown)
			break
	_reasons.text = text.strip_edges()
	var counter: Array = _explanation.get("counter", [])
	if not accept and not counter.is_empty():
		_counter_articles = counter
		_counter_label.text = "Leur contre-offre (ils accepteraient) : %s." % _explanation.get("counter_text", "")
		_counter_box.show()
	return true


func _adopt_counter() -> void:
	if _counter_articles.is_empty():
		return
	articles = _counter_articles.duplicate(true)
	render_draft()


func fill_menu(menu: MenuButton) -> void:
	var popup := menu.get_popup()
	popup.clear()
	for child in popup.get_children():
		if child is PopupMenu:
			child.queue_free()
	if popup.id_pressed.is_connected(_on_menu_id):
		popup.id_pressed.disconnect(_on_menu_id)
	var items: Array = []  # [label, article | null, children[]]
	var ours: Dictionary = _options.get("ours", {})
	var theirs: Dictionary = _options.get("theirs", {})
	var at_war := bool(_options.get("at_war", false))
	if menu == clause_menu:
		if at_war:
			items.append(["Paix", {"kind": "peace"}])
			items.append(["Trêve de deux ans", {"kind": "truce", "turns": TRUCE_TURNS}])
		if not bool(_options.get("allied", false)):
			items.append(["Alliance", {"kind": "alliance"}])
		if not bool(_options.get("trade", false)):
			items.append(["Accord commercial", {"kind": "trade_agreement"}])
		var pairs: Array = []
		for a in ours.get("marriageable", []):
			for b in theirs.get("marriageable", []):
				if bool(a.get("female", false)) != bool(b.get("female", false)) and pairs.size() < 10:
					pairs.append(["%s et %s" % [a["name"], b["name"]], {"kind": "marriage", "character": a["id"], "spouse": b["id"]}])
		if not pairs.is_empty():
			items.append(["Mariage", null, pairs])
	else:
		var giver := "proposer" if menu == _offer_menu else "recipient"
		var side: Dictionary = ours if giver == "proposer" else theirs
		var other_occupies := "occupée par eux" if giver == "proposer" else "occupée par nous"
		var gold: Array = []
		for amount in GOLD_STEPS:
			if amount <= int(side.get("treasury", 0)):
				gold.append([Money.amount(amount), {"kind": "gold", "giver": giver, "amount": amount}])
		if not gold.is_empty():
			items.append(["Or", null, gold])
		var tribute: Array = []
		for amount in TRIBUTE_STEPS:
			tribute.append(["%s par saison, %d saisons" % [Money.amount(amount), TRIBUTE_SEASONS], {"kind": "tribute", "giver": giver, "per_season": amount, "seasons": TRIBUTE_SEASONS}])
		items.append(["Tribut", null, tribute])
		var access_key := "access_given" if giver == "proposer" else "access_received"
		if not bool(_options.get(access_key, false)):
			items.append(["Accès militaire", {"kind": "military_access", "giver": giver}])
		var provinces: Array = []
		for province in side.get("provinces", []):
			var label := str(province["name"])
			if bool(province.get("capital", false)):
				label += " (capitale)"
			if bool(province.get("occupied", false)):
				label += " — %s" % other_occupies
			if bool(province.get("war_goal", false)):
				label = "★ " + label
			provinces.append([label, {"kind": "cede_province", "giver": giver, "province": province["id"]}])
		if not provinces.is_empty():
			items.append(["Province", null, provinces])
		var places: Array = []
		for place in side.get("settlements", []):
			if places.size() >= 24:
				break
			places.append(["%s (%s)" % [place["name"], place["province"]], {"kind": "cede_settlement", "giver": giver, "settlement": place["id"]}])
		if not places.is_empty():
			items.append(["Place forte ou colonie", null, places])
		var captives: Array = []
		for captive in side.get("captives", []):
			captives.append([str(captive["name"]), {"kind": "release_captive", "giver": giver, "character": captive["id"]}])
		if not captives.is_empty():
			items.append(["Libérer un captif", null, captives])
		var hostages: Array = []
		for hostage in side.get("hostages", []):
			hostages.append([str(hostage["name"]), {"kind": "hostage", "giver": giver, "character": hostage["id"]}])
		if not hostages.is_empty():
			items.append(["Otage", null, hostages])
		items.append(["Devenir vassal" if giver == "proposer" else "Vassalité", {"kind": "vassalage", "giver": giver}])
	var id := 0
	var lookup := {}
	for item in items:
		if item.size() > 2:
			var sub := PopupMenu.new()
			for child in item[2]:
				sub.add_item(str(child[0]), id)
				lookup[id] = child[1]
				id += 1
			sub.id_pressed.connect(func(pressed: int) -> void: _add_article(lookup.get(pressed, {})))
			popup.add_submenu_node_item(str(item[0]), sub)
		else:
			popup.add_item(str(item[0]), id)
			lookup[id] = item[1]
			id += 1
	popup.set_meta("lookup", lookup)
	popup.id_pressed.connect(_on_menu_id.bind(popup))


func _on_menu_id(pressed: int, popup: PopupMenu) -> void:
	var lookup: Dictionary = popup.get_meta("lookup", {})
	_add_article(lookup.get(pressed, {}))


func _add_article(article: Dictionary) -> void:
	if article.is_empty():
		return
	for existing in articles:
		if JSON.stringify(existing) == JSON.stringify(article):
			return
	# Un seul versement d'or et un seul tribut par camp : le nouveau remplace l'ancien.
	if str(article["kind"]) in ["gold", "tribute"]:
		for index in range(articles.size() - 1, -1, -1):
			var old: Dictionary = articles[index]
			if old.get("kind") == article["kind"] and old.get("giver") == article.get("giver"):
				articles.remove_at(index)
	articles.append(article)
	render_draft()


func ask_counter() -> void:
	if sim == null or not sim.has_method("counter_treaty"):
		return
	var answer: Dictionary = sim.call("counter_treaty", faction_id, articles)
	if bool(answer.get("ok", false)):
		articles = answer.get("articles", [])
		render_draft()
	else:
		chance_label.text = str(answer.get("error", "Aucune contre-proposition."))


func _send_treaty() -> void:
	if articles.is_empty():
		return
	order_requested.emit({"type": "propose_treaty", "target": faction_id, "articles": articles}, "Le traité est signé.")


## Captures et smoke : un brouillon de paix type (province exigée, or offert).
func stage_example() -> void:
	var theirs: Array = (_options.get("theirs", {}) as Dictionary).get("provinces", [])
	articles = [{"kind": "peace"}, {"kind": "gold", "giver": "proposer", "amount": 5000}]
	for province in theirs:
		if not bool(province.get("capital", false)):
			articles.append({"kind": "cede_province", "giver": "recipient", "province": province["id"]})
			break
	articles.append({"kind": "trade_agreement"})
	render_draft()


## DP2 (captures) : une offre généreuse gâchée par une seule exigence d'or excessive.
func stage_counter_example() -> void:
	articles = [{"kind": "trade_agreement"}, {"kind": "gold", "giver": "proposer", "amount": 2500},
		{"kind": "gold", "giver": "recipient", "amount": 10000}]
	render_draft()

