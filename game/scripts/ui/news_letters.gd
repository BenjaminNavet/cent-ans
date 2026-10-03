class_name NewsLetters
extends Control

## Lettres scellées (HUD de campagne, haut à droite sous la minicarte ; lot F10a) : chaque
## nouvelle est une lettre pliée, scellée d'une cire à l'écu de la faction concernée, avec
## une rubrique (type) et un titre court. La plus récente est en haut.
##
## Aucune règle de jeu : `push_news({kind, title, text, faction_id?, province_id?})`. Types
## connus : `faction_met`, `war_declared`, `peace`, `birth`, `death`, `succession`,
## `city_captured` (autres : rubrique « Nouvelle »).
## Clic gauche = déplie le texte et émet `news_activated(item)` ; clic droit = écarte la lettre
## (`news_dismissed(item)`).

## Lettre ouverte (clic gauche) : l'appelant peut centrer la caméra, ouvrir la diplomatie…
signal news_activated(item: Dictionary)
## Lettre écartée (clic droit).
signal news_dismissed(item: Dictionary)

const KIND_LABELS := {
	"faction_met": "Rencontre",
	"war_declared": "Déclaration de guerre",
	"peace": "Traité de paix",
	"birth": "Naissance",
	"death": "Décès",
	"succession": "Succession",
	"city_captured": "Prise de ville",
	# Types du journal de la simulation (`CampaignSim.end_turn`, `EventKind` en snake_case).
	"peace_signed": "Traité de paix",
	"province_captured": "Prise de ville",
	"alliance_formed": "Alliance",
	"alliance_broken": "Rupture d'alliance",
	"marriage": "Mariage",
	"faction_destroyed": "Chute d'une maison",
	"vassalage": "Hommage",
	"vassal_rebellion": "Révolte d'un vassal",
	"excommunication": "Excommunication",
	"regency": "Régence",
	"general_captured": "Chef capturé",
	"revolt": "Révolte",
	"plague": "Peste",
	"table": "La Table",  # H9
	"edict": "Édit régional",  # lot C4
	"medicine": "Médecine",  # H9
	"coinage": "Monnaie",  # H11
	"ransom": "Rançon",  # H11
	"chivalry": "Chevalerie",  # H11
	"crusade": "Croisade",  # JR3
}
## Types d'événements de la simulation qui méritent une lettre (les autres restent au journal).
const NEWS_EVENT_KINDS := [
	"war_declared", "peace_signed", "alliance_formed", "alliance_broken", "marriage", "birth", "death",
	"succession", "province_captured", "faction_destroyed", "vassalage", "vassal_rebellion",
	"excommunication", "regency", "general_captured", "revolt", "plague",
	"coinage", "ransom", "chivalry",  # H11
	"crusade"]  # JR3 (les nouvelles privées d'une croisade étrangère sont filtrées par `MapUI.journal_keeps` ; JR5 : les publiques passent)
const LETTER_WIDTH := 300.0
const SEAL_RADIUS := 21.0
## Nombre maximal de lettres conservées (les plus anciennes sont oubliées).
const MAX_KEPT := 40

## Lettres visibles ; au-delà, une ligne « + n lettres plus anciennes ».
@export var max_visible: int = 5
## Lot U5 : plafond de `max_visible` et hauteur estimée d'une lettre repliée (`fit_height`).
const MAX_VISIBLE_DEFAULT := 5
const LETTER_HEIGHT_ESTIMATE := 98.0

var _items: Array = []  # plus récente en premier
var _expanded: Dictionary = {}  # instance id de l'item → vrai
var _box: VBoxContainer
var _more_label: Label
var _more_pill: PanelContainer


func _ready() -> void:
	custom_minimum_size = Vector2(LETTER_WIDTH, 0)
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	_box = VBoxContainer.new()
	_box.add_theme_constant_override("separation", 6)
	_box.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_box.custom_minimum_size = Vector2(LETTER_WIDTH, 0)
	add_child(_box)
	# « + n lettres plus anciennes » : petite étiquette parchemin alignée à droite.
	_more_pill = PanelContainer.new()
	var pill_box := HudStyle.card_box(HudStyle.PARCHMENT, HudStyle.INK_SOFT)
	pill_box.content_margin_left = 8
	pill_box.content_margin_right = 8
	pill_box.content_margin_top = 1
	pill_box.content_margin_bottom = 2
	_more_pill.add_theme_stylebox_override("panel", pill_box)
	_more_pill.size_flags_horizontal = Control.SIZE_SHRINK_END
	_more_pill.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_more_label = HudStyle.label("", HudStyle.FONT_SMALL, HudStyle.INK_SOFT)
	_more_pill.add_child(_more_label)
	_rebuild()


func _notification(what: int) -> void:
	# L'étiquette « + n » est hors de l'arbre quand elle est masquée : la libérer à la main.
	if what == NOTIFICATION_PREDELETE and _more_pill != null and not _more_pill.is_inside_tree():
		_more_pill.free()


## Lot U5 : nombre de lettres visibles ajusté à la hauteur disponible (au-dessus des pastilles
## d'alerte de la cloche) ; 0 = seule l'étiquette « + n lettres » reste.
func fit_height(max_height: float) -> void:
	var fitting := clampi(int((max_height - 26.0 + 6.0) / (LETTER_HEIGHT_ESTIMATE + 6.0)), 0, MAX_VISIBLE_DEFAULT)
	if fitting != max_visible:
		max_visible = fitting
		_rebuild()


## Ajoute une nouvelle en tête de pile.
func push_news(item: Dictionary) -> void:
	_items.push_front(item.duplicate(true))
	UiSounds.play("letter")  # UB1 / U13 : lettre reçue
	if _items.size() > MAX_KEPT:
		_items.resize(MAX_KEPT)
	_rebuild(true)


## Ajoute plusieurs nouvelles (ordre chronologique : la dernière finit en tête) avec une seule
## reconstruction et un seul son (fin de tour).
func push_news_batch(items: Array) -> void:
	if items.is_empty():
		return
	for item in items:
		_items.push_front((item as Dictionary).duplicate(true))
	UiSounds.play("letter")
	if _items.size() > MAX_KEPT:
		_items.resize(MAX_KEPT)
	_rebuild(true)


## Toutes les lettres conservées, plus récente en premier.
func get_items() -> Array:
	return _items


## Écarte la lettre d'index `index` (0 = plus récente).
func dismiss(index: int) -> void:
	if index < 0 or index >= _items.size():
		return
	var item: Dictionary = _items[index]
	_items.remove_at(index)
	news_dismissed.emit(item)
	_rebuild()


## Écarte tout.
func clear() -> void:
	_items.clear()
	_expanded.clear()
	_rebuild()


## Ouvre (déplie) la lettre d'index `index` et émet `news_activated`.
func activate(index: int) -> void:
	if index < 0 or index >= _items.size():
		return
	var item: Dictionary = _items[index]
	var key := _key(item)
	_expanded[key] = not bool(_expanded.get(key, false))
	news_activated.emit(item)
	_rebuild()


## Convertit un événement de `CampaignSim.end_turn()` / `get_events()`
## (`{kind, text_fr, province, army, faction}`) en lettre, ou `{}` s'il ne mérite pas de lettre.
static func news_from_event(event: Dictionary) -> Dictionary:
	var kind := str(event.get("kind", ""))
	if not NEWS_EVENT_KINDS.has(kind):
		return {}
	var text := str(event.get("text_fr", ""))
	return {"kind": kind, "title": text, "text": text, "public": bool(event.get("public", false)),
		"faction_id": str(event.get("faction", "")), "province_id": str(event.get("province", ""))}


## Rubrique affichée pour un type de nouvelle.
static func kind_label(kind: String) -> String:
	return str(KIND_LABELS.get(kind, "Nouvelle"))


## Couleur de cire selon le type : verte pour la paix (actes perpétuels), brune pour un décès.
static func wax_color(kind: String) -> Color:
	match kind:
		"peace", "peace_signed", "faction_met", "alliance_formed", "marriage", "vassalage":
			return HudStyle.WAX_GREEN
		"death":
			return Color(0.25, 0.14, 0.09, 1.0)
		"birth":
			return HudStyle.WAX_LIGHT
		_:
			return HudStyle.WAX


func _key(item: Dictionary) -> String:
	return "%s|%s|%s" % [item.get("kind", ""), item.get("title", ""), item.get("text", "")]


func _rebuild(animate_first := false) -> void:
	if _box == null:
		return
	for child in _box.get_children():
		_box.remove_child(child)
		if child != _more_pill:
			child.queue_free()
	var shown := mini(_items.size(), maxi(max_visible, 0))
	for i in shown:
		var letter := Letter.new()
		letter.owner_list = self
		letter.index = i
		letter.item = _items[i]
		letter.expanded = bool(_expanded.get(_key(_items[i]), false))
		_box.add_child(letter)
		if i == 0 and animate_first and is_inside_tree() and not Accessibility.reduce_motion():
			letter.modulate.a = 0.0
			create_tween().tween_property(letter, "modulate:a", 1.0, 0.35)
	var hidden := _items.size() - shown
	if hidden > 0:
		_more_label.text = "+ %d lettre%s plus ancienne%s" % [hidden, "s" if hidden > 1 else "", "s" if hidden > 1 else ""]
		_box.add_child(_more_pill)
	size = _box.get_combined_minimum_size()


## Une lettre pliée avec son sceau.
class Letter:
	extends PanelContainer

	var owner_list: NewsLetters
	var index: int = 0
	var item: Dictionary = {}
	var expanded := false
	var _heraldry: Texture2D
	var _hover := false

	func _ready() -> void:
		mouse_filter = Control.MOUSE_FILTER_STOP
		mouse_default_cursor_shape = Control.CURSOR_POINTING_HAND
		custom_minimum_size = Vector2(NewsLetters.LETTER_WIDTH, 56)
		var box := StyleBoxFlat.new()
		box.bg_color = HudStyle.PARCHMENT
		box.border_color = HudStyle.INK_SOFT
		box.set_border_width_all(1)
		box.content_margin_left = NewsLetters.SEAL_RADIUS * 2.0 + 16.0
		box.content_margin_right = 22.0
		box.content_margin_top = 6.0
		box.content_margin_bottom = 7.0
		box.shadow_color = HudStyle.SHADOW
		box.shadow_size = 4
		box.shadow_offset = Vector2(0, 2)
		add_theme_stylebox_override("panel", box)
		_heraldry = PortraitLoader.heraldry_texture(str(item.get("faction_id", "")))
		var column := VBoxContainer.new()
		column.add_theme_constant_override("separation", 0)
		column.mouse_filter = Control.MOUSE_FILTER_IGNORE
		add_child(column)
		var rubric := HudStyle.label(NewsLetters.kind_label(str(item.get("kind", ""))).to_upper(), 11, HudStyle.RUBRIC)
		column.add_child(rubric)
		var title := HudStyle.label(str(item.get("title", "")), HudStyle.FONT_BODY, HudStyle.INK)
		title.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		title.custom_minimum_size = Vector2(NewsLetters.LETTER_WIDTH - NewsLetters.SEAL_RADIUS * 2.0 - 40.0, 0)
		if not expanded:
			title.max_lines_visible = 2
			title.text_overrun_behavior = TextServer.OVERRUN_TRIM_WORD_ELLIPSIS
		column.add_child(title)
		var text := str(item.get("text", ""))
		if expanded and text != "" and text != title.text:
			var rule := ColorRect.new()
			rule.color = HudStyle.GOLD
			rule.custom_minimum_size = Vector2(0, 1)
			rule.mouse_filter = Control.MOUSE_FILTER_IGNORE
			column.add_child(rule)
			var body := HudStyle.label(text, HudStyle.FONT_SMALL + 1, HudStyle.INK_SOFT)
			body.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
			body.custom_minimum_size = title.custom_minimum_size
			column.add_child(body)
		var actions_hint := "Clic : lire · clic droit : écarter" if not expanded else "Clic : replier · clic droit : écarter"
		var interest := str(item.get("interest", ""))  # U5 : pourquoi cette nouvelle est retenue
		RichTooltip.attach_plain(self, "news_card_actions", {"title": interest if interest != "" else "Actions", "body": actions_hint})
		mouse_entered.connect(func() -> void:
			_hover = true
			queue_redraw())
		mouse_exited.connect(func() -> void:
			_hover = false
			queue_redraw())

	func _gui_input(event: InputEvent) -> void:
		var click := event as InputEventMouseButton
		if click == null or not click.pressed:
			return
		if click.button_index == MOUSE_BUTTON_LEFT:
			owner_list.activate.call_deferred(index)
			accept_event()
		elif click.button_index == MOUSE_BUTTON_RIGHT:
			owner_list.dismiss.call_deferred(index)
			accept_event()

	func _draw() -> void:
		var rect := Rect2(Vector2.ZERO, size)
		if _hover:
			draw_rect(rect.grow(-1), HudStyle.PARCHMENT_LIGHT)
		# Pli : coin supérieur droit rabattu.
		var fold := 16.0
		draw_colored_polygon(PackedVector2Array([
			Vector2(size.x - fold, 0), Vector2(size.x, 0), Vector2(size.x, fold)]), HudStyle.PARCHMENT_DARK)
		draw_line(Vector2(size.x - fold, 0), Vector2(size.x, fold), HudStyle.INK_SOFT, 1.0, true)
		# Liseré rubrique à gauche (manuscrit : initiale rubriquée).
		draw_rect(Rect2(0, 0, 3, size.y), HudStyle.RUBRIC)
		# Sceau de cire au milieu du bord gauche, avec l'écu de la faction.
		var center := Vector2(NewsLetters.SEAL_RADIUS + 8.0, minf(size.y * 0.5, 30.0))
		var kind := str(item.get("kind", ""))
		var inner := HudStyle.draw_wax_seal(self, center, NewsLetters.SEAL_RADIUS, NewsLetters.wax_color(kind), hash(item.get("title", "")) % 31)
		if _heraldry != null:
			draw_circle(center, inner - 1.0, HudStyle.PARCHMENT_LIGHT)
			HudStyle.draw_texture_fit(self, _heraldry, center + Vector2(0, 0.5), inner * 1.45)


	## Infobulle en sections (`attach_plain` ne pose pas `plain_tooltip_host.gd` sur une classe
	## scriptée ; Q8 : sans elle, clé et BBCode bruts).
	func _make_custom_tooltip(for_text: String) -> Object:
		return RichTooltip.panel_for(for_text, self)
