class_name CouncilChamber
extends PanelContainer

## Fenêtre « Conseil du royaume » (RG13c, ADR 0341 § 3-4) : la salle 3D (`CouncilRoom` dans un
## `SubViewport`) à gauche, la fiche de la charge choisie à droite. Rendu et entrées seulement :
## les charges, candidats et ordres viennent du pont (`get_council`, `get_council_candidates`,
## ordres `appoint_councillor` / `dismiss_councillor`). Sans pont (ancien cœur), des données de
## démonstration montrent la fenêtre. Les conseils sont les alertes / indices / missions déjà
## produits (`advice_provider`), rangés par `kind` dans les `advice_domains` de chaque charge.

signal closed
## Clic sur une ligne de conseil : même chemin que la pastille de la cloche.
signal advice_activated(item: Dictionary)
## Un ordre vient d'être accepté (la carte rafraîchit son état).
signal order_done(order: Dictionary)

const SHEET_WIDTH := 340.0
const SEVERITY_RANK := {"danger": 0, "warning": 1, "info": 2}
const ROLE_LABELS := {
	"chancellor": "Chancelier", "constable": "Connétable", "treasurer": "Grand trésorier",
	"chaplain": "Aumônier", "chamberlain": "Chambellan",
}

var sim_source: Object = null
var faction_id: String = ""
var faction_label: String = ""
var faction_color: Color = Color(0.6, 0.15, 0.15)
## `Callable() -> Array` d'éléments `{id, kind, text, severity, ...}` (alertes, indice, missions).
var advice_provider: Callable = Callable()
## `Callable(order: Dictionary) -> Dictionary` ; défaut : `sim_source.submit_order`.
var order_submitter: Callable = Callable()

var rows: Array[Dictionary] = []
var using_demo: bool = false
var selected_key: String = ""  # "" = rien, "ruler" ou identifiant de charge
var room: CouncilRoom
var viewport: SubViewport
var viewport_container: SubViewportContainer
var sheet_box: VBoxContainer
var card: PanelContainer
var title_label: Label
var _card_portrait: TextureRect
var _card_name: Label
var _card_office: Label
var _card_skill: Label
var _built: bool = false


func _ready() -> void:
	_build()


func _build() -> void:
	if _built:
		return
	_built = true
	theme = load("res://scenes/ui/parchment_theme.tres")
	theme_type_variation = &"IlluminatedPanel"
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 6)
	add_child(box)
	var header := HBoxContainer.new()
	title_label = Label.new()
	title_label.text = "Conseil du royaume"
	title_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	UiType.apply(title_label, UiType.HEADING)
	header.add_child(title_label)
	var close := Button.new()
	close.name = "CloseButton"
	close.text = "×"
	close.pressed.connect(func() -> void:
		UiMotion.fade_out(self)
		closed.emit())
	header.add_child(close)
	box.add_child(header)
	var body := HBoxContainer.new()
	body.size_flags_vertical = Control.SIZE_EXPAND_FILL
	body.add_theme_constant_override("separation", 8)
	box.add_child(body)
	var stage := Control.new()
	stage.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	stage.size_flags_vertical = Control.SIZE_EXPAND_FILL
	stage.custom_minimum_size = Vector2(520, 360)
	stage.clip_contents = true
	body.add_child(stage)
	viewport_container = SubViewportContainer.new()
	viewport_container.stretch = true
	viewport_container.set_anchors_preset(Control.PRESET_FULL_RECT)
	viewport_container.mouse_filter = Control.MOUSE_FILTER_STOP
	viewport_container.gui_input.connect(_on_stage_input)
	viewport_container.mouse_exited.connect(_on_stage_exit)
	stage.add_child(viewport_container)
	viewport = SubViewport.new()
	viewport.own_world_3d = true
	viewport.size = Vector2i(760, 480)
	viewport.msaa_3d = Viewport.MSAA_2X
	viewport.render_target_update_mode = SubViewport.UPDATE_WHEN_VISIBLE
	viewport_container.add_child(viewport)
	room = CouncilRoom.new()
	room.name = "Room"
	viewport.add_child(room)
	room.build(faction_color, faction_id)
	_build_card(stage)
	var scroll := ScrollContainer.new()
	scroll.custom_minimum_size = Vector2(SHEET_WIDTH, 0)
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	body.add_child(scroll)
	sheet_box = VBoxContainer.new()
	sheet_box.name = "Sheet"
	sheet_box.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	sheet_box.add_theme_constant_override("separation", 6)
	scroll.add_child(sheet_box)


func _build_card(stage: Control) -> void:
	card = PanelContainer.new()
	card.name = "HoverCard"
	card.mouse_filter = Control.MOUSE_FILTER_IGNORE
	card.hide()
	var row := HBoxContainer.new()
	row.add_theme_constant_override("separation", 8)
	card.add_child(row)
	_card_portrait = TextureRect.new()
	_card_portrait.custom_minimum_size = Vector2(64, 80)
	_card_portrait.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	_card_portrait.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_COVERED
	row.add_child(_card_portrait)
	var column := VBoxContainer.new()
	row.add_child(column)
	_card_name = Label.new()
	_card_office = Label.new()
	_card_skill = Label.new()
	UiType.apply(_card_name, UiType.BODY)
	UiType.apply(_card_office, UiType.CAPTION)
	UiType.apply(_card_skill, UiType.CAPTION)
	for label in [_card_name, _card_office, _card_skill]:
		column.add_child(label)
	stage.add_child(card)


# --- Données ----------------------------------------------------------------------------------


func _sim() -> Object:
	if sim_source != null:
		return sim_source
	var facade := get_node_or_null("/root/SimFacade")
	return facade.get("sim") if facade != null else null


## Ouvre la fenêtre pour `faction` (rechargement complet).
func open_for(faction: String, label: String, color: Color) -> void:
	_build()
	faction_id = faction
	faction_label = label
	faction_color = color
	title_label.text = "Conseil du royaume — %s" % label if label != "" else "Conseil du royaume"
	refresh()
	var was_visible := visible
	show()
	if not was_visible:
		UiMotion.fade_in(self)
	_fit_to_view()


func _fit_to_view() -> void:
	if not is_inside_tree():
		return
	var view := get_viewport_rect().size
	var width := minf(1180.0, view.x - 80.0)
	var height := minf(700.0, view.y - 120.0)
	set_anchors_preset(Control.PRESET_CENTER)
	offset_left = -width * 0.5
	offset_right = width * 0.5
	offset_top = -height * 0.5 + 20.0
	offset_bottom = height * 0.5 + 20.0


## Relit le conseil depuis le pont (ou la démonstration), met à jour la salle et la fiche.
func refresh() -> void:
	_build()
	rows = fetch_council()
	var occupied := {}
	for row in rows:
		occupied[str(row.get("office_id", ""))] = str(row.get("holder_id", "")) != ""
	room.faction_color = faction_color
	room.faction_id = faction_id
	room.populate(occupied)
	_show_selection()


func fetch_council() -> Array[Dictionary]:
	var sim := _sim()
	var result: Array[Dictionary] = []
	if sim != null and sim.has_method("get_council"):
		using_demo = false
		for entry in sim.call("get_council", faction_id):
			if entry is Dictionary:
				result.append(entry)
		return result
	using_demo = true
	return demo_council()


func fetch_candidates(office_id: String) -> Array[Dictionary]:
	var sim := _sim()
	var result: Array[Dictionary] = []
	if not using_demo and sim != null and sim.has_method("get_council_candidates"):
		for entry in sim.call("get_council_candidates", faction_id, office_id):
			if entry is Dictionary:
				result.append(entry)
	else:
		result = demo_candidates()
	result.sort_custom(func(a: Dictionary, b: Dictionary) -> bool:
		return int(a.get("skill_value", 0)) > int(b.get("skill_value", 0)))
	return result


static func demo_council() -> Array[Dictionary]:
	var data: Array[Dictionary] = [
		{"office_id": "chancellor", "office_name": "Chancelier", "skill": "court", "holder_id": "demo_a",
			"holder_name": "Guillaume de Nogaret", "holder_skill": 6,
			"effect_text": "+6 points d'acceptation diplomatique.", "advice_domains": PackedStringArray(["diplomacy_offer", "mission_due"])},
		{"office_id": "constable", "office_name": "Connétable", "skill": "command", "holder_id": "demo_b",
			"holder_name": "Gautier de Brienne", "holder_skill": 7,
			"effect_text": "+1,4 expérience des recrues, +3,5 % de moral d'armée.", "advice_domains": PackedStringArray(["enemy_army", "siege", "idle_army"])},
		{"office_id": "treasurer", "office_name": "Grand trésorier", "skill": "governance", "holder_id": "demo_c",
			"holder_name": "Enguerrand de Marigny", "holder_skill": 5,
			"effect_text": "+2,5 % des recettes fiscales.", "advice_domains": PackedStringArray(["debt", "free_slot", "construction_done"])},
		{"office_id": "chaplain", "office_name": "Aumônier", "skill": "court", "holder_id": "",
			"holder_name": "", "holder_skill": 0,
			"effect_text": "Charge vacante : aucun effet.", "advice_domains": PackedStringArray(["ransom", "herbarium"])},
		{"office_id": "chamberlain", "office_name": "Chambellan", "skill": "court", "holder_id": "demo_e",
			"holder_name": "Mahaut d'Artois", "holder_skill": 4,
			"effect_text": "+0,4 prestige par saison.", "advice_domains": PackedStringArray(["research_idle", "chronicle_decision"])},
	]
	return data


static func demo_candidates() -> Array[Dictionary]:
	var data: Array[Dictionary] = [
		{"character_id": "demo_x", "name": "Pierre de Chalon", "skill_value": 5, "current_office": ""},
		{"character_id": "demo_y", "name": "Jeanne de Navarre", "skill_value": 8, "current_office": "chancellor"},
		{"character_id": "demo_z", "name": "Robert le Bossu", "skill_value": 3, "current_office": ""},
	]
	return data


## Ligne de la charge `office_id`, ou `{}`.
func row_of(office_id: String) -> Dictionary:
	for row in rows:
		if str(row.get("office_id", "")) == office_id:
			return row
	return {}


## Éléments de conseil dont le `kind` est dans `domains`, du plus urgent au moins urgent.
static func filter_advice(items: Array, domains: PackedStringArray) -> Array[Dictionary]:
	var result: Array[Dictionary] = []
	for item in items:
		if item is Dictionary and str(item.get("kind", "")) in domains:
			result.append(item)
	result.sort_custom(func(a: Dictionary, b: Dictionary) -> bool:
		return int(SEVERITY_RANK.get(str(a.get("severity", "info")), 3)) < int(SEVERITY_RANK.get(str(b.get("severity", "info")), 3)))
	return result


func all_advice() -> Array:
	if advice_provider.is_valid():
		return advice_provider.call()
	return demo_advice() if using_demo else []


static func demo_advice() -> Array:
	return [
		{"id": "debt", "kind": "debt", "severity": "danger", "text": "Trésor endetté : -120 livres"},
		{"id": "army:1", "kind": "enemy_army", "severity": "danger", "text": "Armée ennemie aux portes de Rouen (3 200)"},
		{"id": "offers", "kind": "diplomacy_offer", "severity": "warning", "text": "1 proposition diplomatique en attente"},
		{"id": "research_idle", "kind": "research_idle", "severity": "warning", "text": "Aucune recherche en cours"},
		{"id": "slot", "kind": "free_slot", "severity": "info", "text": "2 emplacements de construction libres"},
	]


func advice_for_office(office_id: String) -> Array[Dictionary]:
	return filter_advice(all_advice(), row_of(office_id).get("advice_domains", PackedStringArray()))


## Les trois conseils les plus urgents toutes charges confondues (`item.office_name` ajouté).
func top_advice(limit: int = 3) -> Array[Dictionary]:
	var merged: Array[Dictionary] = []
	var seen := {}
	for row in rows:
		for item in advice_for_office(str(row.get("office_id", ""))):
			var key := str(item.get("id", item.get("text", "")))
			if seen.has(key):
				continue
			seen[key] = true
			var copy := item.duplicate()
			copy["office_name"] = str(row.get("office_name", ""))
			merged.append(copy)
	merged.sort_custom(func(a: Dictionary, b: Dictionary) -> bool:
		return int(SEVERITY_RANK.get(str(a.get("severity", "info")), 3)) < int(SEVERITY_RANK.get(str(b.get("severity", "info")), 3)))
	return merged.slice(0, limit)


func seat_count() -> int:
	var count := 0
	for key in room.figures:
		if key != "ruler":
			count += 1
	return count


func vacant_count() -> int:
	return room.vacant_labels.size()


# --- Entrées sur la scène ---------------------------------------------------------------------


func _on_stage_input(event: InputEvent) -> void:
	if event is InputEventMouseMotion:
		_hover_at((event as InputEventMouseMotion).position)
	elif event is InputEventMouseButton:
		var button := event as InputEventMouseButton
		if button.pressed and button.button_index == MOUSE_BUTTON_LEFT:
			click_at(button.position)


func _on_stage_exit() -> void:
	room.set_hover("")
	card.hide()


## Avec `stretch`, la fenêtre 3D a la taille du conteneur : coordonnées identiques.
func _to_viewport(position_: Vector2) -> Vector2:
	return position_


func _hover_at(position_: Vector2) -> void:
	var key := room.pick_screen(_to_viewport(position_))
	room.set_hover(key)
	if key == "":
		card.hide()
		return
	show_card(key)
	var stage := card.get_parent() as Control
	card.position = (position_ + Vector2(18, 18)).clamp(Vector2.ZERO, (stage.size - card.size).max(Vector2.ZERO))


func click_at(position_: Vector2) -> void:
	var key := room.pick_screen(_to_viewport(position_))
	if key != "":
		select(key)


## Remplit la carte flottante pour `key`.
func show_card(key: String) -> void:
	var name_text := ""
	var office_text := ""
	var skill_text := ""
	var holder_id := ""
	if key == "ruler":
		name_text = _ruler_name()
		office_text = "Souverain"
		skill_text = "Préside le conseil"
		holder_id = _ruler_id()
	else:
		var row := row_of(key)
		holder_id = str(row.get("holder_id", ""))
		office_text = str(row.get("office_name", ROLE_LABELS.get(key, key)))
		if holder_id == "":
			name_text = "Charge vacante"
			skill_text = "Aucun effet"
		else:
			name_text = str(row.get("holder_name", ""))
			skill_text = "Compétence %d (%s)" % [int(row.get("holder_skill", 0)), str(row.get("skill", ""))]
	_card_name.text = name_text
	_card_office.text = office_text
	_card_skill.text = skill_text
	_card_portrait.texture = PortraitLoader.portrait_texture(holder_id) if holder_id != "" and not holder_id.begins_with("demo_") else null
	_card_portrait.visible = _card_portrait.texture != null
	card.show()


func _ruler_summary() -> Dictionary:
	var sim := _sim()
	if sim != null and not using_demo and sim.has_method("get_faction_summary"):
		return sim.call("get_faction_summary", faction_id)
	return {}


func _ruler_name() -> String:
	var summary := _ruler_summary()
	for key in ["ruler_name", "leader_name", "ruler"]:
		if str(summary.get(key, "")) != "":
			return str(summary[key])
	return faction_label if faction_label != "" else "Le souverain"


func _ruler_id() -> String:
	var summary := _ruler_summary()
	return str(summary.get("ruler_id", summary.get("leader_id", "")))


# --- Fiche latérale ---------------------------------------------------------------------------


func select(key: String) -> void:
	selected_key = key
	_show_selection()


func _clear_sheet() -> void:
	for child in sheet_box.get_children():
		sheet_box.remove_child(child)
		child.queue_free()


func _label(text: String, size_kind: String = UiType.CAPTION, wrap: bool = true) -> Label:
	var label := Label.new()
	label.text = text
	UiType.apply(label, size_kind)
	if wrap:
		label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		label.custom_minimum_size = Vector2(SHEET_WIDTH - 24.0, 0)
	return label


func _show_selection() -> void:
	_clear_sheet()
	if selected_key == "" or selected_key == "ruler":
		_show_overview()
	else:
		_show_office(selected_key)


func _show_overview() -> void:
	sheet_box.add_child(_label("%s préside" % (_ruler_name() if _ruler_name() != "" else "Le souverain"), UiType.BODY))
	sheet_box.add_child(_label("Survolez un conseiller pour le connaître, cliquez pour sa charge."))
	sheet_box.add_child(_label("Conseils les plus pressants", UiType.BODY))
	_add_advice_lines(top_advice(3), true)
	if using_demo:
		sheet_box.add_child(_label("(Données de démonstration : le cœur n'expose pas encore le conseil.)"))


func _show_office(office_id: String) -> void:
	var row := row_of(office_id)
	if row.is_empty():
		_show_overview()
		return
	sheet_box.add_child(_label(str(row.get("office_name", "")), UiType.HEADING, false))
	var holder_id := str(row.get("holder_id", ""))
	var holder := HBoxContainer.new()
	var portrait := TextureRect.new()
	portrait.custom_minimum_size = Vector2(72, 90)
	portrait.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	portrait.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_COVERED
	if holder_id != "" and not holder_id.begins_with("demo_"):
		portrait.texture = PortraitLoader.portrait_texture(holder_id)
	portrait.visible = portrait.texture != null
	holder.add_child(portrait)
	var holder_text := VBoxContainer.new()
	if holder_id == "":
		holder_text.add_child(_label("Charge vacante", UiType.BODY))
	else:
		holder_text.add_child(_label(str(row.get("holder_name", "")), UiType.BODY))
		holder_text.add_child(_label("Compétence %s : %d" % [str(row.get("skill", "")), int(row.get("holder_skill", 0))]))
	holder.add_child(holder_text)
	sheet_box.add_child(holder)
	sheet_box.add_child(_label(str(row.get("effect_text", ""))))
	sheet_box.add_child(_label("Conseils", UiType.BODY))
	_add_advice_lines(advice_for_office(office_id), false)
	var buttons := HBoxContainer.new()
	var appoint := Button.new()
	appoint.name = "AppointButton"
	appoint.text = "Nommer…"
	appoint.pressed.connect(show_candidates.bind(office_id))
	buttons.add_child(appoint)
	var dismiss := Button.new()
	dismiss.name = "DismissButton"
	dismiss.text = "Révoquer"
	dismiss.disabled = holder_id == ""
	dismiss.pressed.connect(func() -> void: submit({"type": "dismiss_councillor", "office": office_id}))
	buttons.add_child(dismiss)
	sheet_box.add_child(buttons)


func _add_advice_lines(items: Array, with_office: bool) -> void:
	if items.is_empty():
		sheet_box.add_child(_label("Rien à signaler."))
		return
	for item: Dictionary in items:
		var button := Button.new()
		button.name = "Advice"
		button.alignment = HORIZONTAL_ALIGNMENT_LEFT
		button.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		button.custom_minimum_size = Vector2(SHEET_WIDTH - 24.0, 0)
		button.text = ("%s : %s" % [item.get("office_name", ""), item.get("text", "")]) if with_office else str(item.get("text", ""))
		button.tooltip_text = str(item.get("tooltip", ""))
		button.pressed.connect(func() -> void: advice_activated.emit(item))
		sheet_box.add_child(button)


## Liste des candidats à `office_id`, du meilleur au moins bon ; un clic nomme.
func show_candidates(office_id: String) -> void:
	_clear_sheet()
	sheet_box.add_child(_label("Nommer — %s" % str(row_of(office_id).get("office_name", "")), UiType.HEADING, false))
	var candidates := fetch_candidates(office_id)
	if candidates.is_empty():
		sheet_box.add_child(_label("Aucun candidat disponible."))
	for candidate in candidates:
		var button := Button.new()
		button.name = "Candidate"
		button.alignment = HORIZONTAL_ALIGNMENT_LEFT
		var current := str(candidate.get("current_office", ""))
		button.text = "%s — compétence %d%s" % [candidate.get("name", ""), int(candidate.get("skill_value", 0)),
			" (siège déjà : %s)" % ROLE_LABELS.get(current, current) if current != "" else ""]
		var candidate_id := str(candidate.get("character_id", ""))
		button.pressed.connect(func() -> void:
			submit({"type": "appoint_councillor", "office": office_id, "character": candidate_id}))
		sheet_box.add_child(button)
	var back := Button.new()
	back.name = "BackButton"
	back.text = "Retour"
	back.pressed.connect(_show_selection)
	sheet_box.add_child(back)


## Soumet un ordre (via `order_submitter` ou le pont) ; relit le conseil si accepté.
func submit(order: Dictionary) -> Dictionary:
	var result := {"ok": false, "error": "Simulation absente"}
	if using_demo:
		result = {"ok": true}
	elif order_submitter.is_valid():
		result = order_submitter.call(order)
	else:
		var sim := _sim()
		if sim != null:
			result = sim.call("submit_order", order)
	if result.get("ok", false):
		order_done.emit(order)
		refresh()
	return result
