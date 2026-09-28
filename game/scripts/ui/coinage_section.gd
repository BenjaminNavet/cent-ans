class_name CoinageSection
extends VBoxContainer

## H11 — section « Monnaie » du panneau de faction, construite en code : monnaie actuelle,
## niveau des prix (jauge 100-400), seigneuriage et refonte (prévus et saison passée), sélecteur
## des quatre niveaux (infobulle riche `RichTooltip.coinage`), ordre `set_coinage` et refus en
## rouge, texte court aux liens du Codex. Aucune règle ici : montants, effets et refus viennent de
## `CampaignSim.get_coinage` / `submit_order` (`docs/design/h5-h6-api.md` § 2 et 5).

signal coinage_changed(level: String)

const ERROR_COLOR := Color(0.55, 0.20, 0.15)
const MUTED_COLOR := Color(0.42, 0.33, 0.20)
const PRICE_MIN := 100.0
const PRICE_MAX := 400.0
const EXPLANATION := "Les rois « muent » la monnaie : moins d'argent fin dans chaque pièce rapporte un [[cdx_mutations_monetaires|seigneuriage]] immédiat, mais les prix montent et les rentes fixes des bourgeois et du clergé fondent. [[cdx_nicole_oresme|Nicole Oresme]] plaide pour une monnaie forte, stable, comptée en [[cdx_livre_tournois|livres tournois]]."

var coinage: Dictionary = {}
## Boutons du sélecteur, par niveau (`strong`, `sound`, `debased`, `heavily_debased`).
var level_buttons: Dictionary = {}
var last_result: Dictionary = {}
var read_only := false

var header_label: Label
var current_label: Label
var price_label: Label
var price_bar: ProgressBar
var seigniorage_label: Label
var recoinage_label: Label
var changed_label: Label
var selector: GridContainer
var error_label: Label
var explanation_label: RichTextLabel
var _sim: Object = null
var _faction: String = ""


func _init() -> void:
	name = "CoinageSection"
	add_theme_constant_override("separation", 3)
	size_flags_horizontal = Control.SIZE_EXPAND_FILL
	var head := HBoxContainer.new()
	add_child(head)
	header_label = Label.new()
	header_label.text = "Monnaie"
	header_label.add_theme_font_size_override("font_size", UiType.size(UiType.BODY))
	header_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	head.add_child(header_label)
	current_label = _small_label(14, false)
	head.add_child(current_label)

	var price_row := HBoxContainer.new()
	price_row.add_theme_constant_override("separation", 8)
	add_child(price_row)
	price_label = _small_label(13, false)
	price_label.mouse_filter = Control.MOUSE_FILTER_PASS
	price_row.add_child(price_label)
	price_bar = ProgressBar.new()
	price_bar.min_value = PRICE_MIN
	price_bar.max_value = PRICE_MAX
	price_bar.show_percentage = false
	price_bar.custom_minimum_size = Vector2(120, 10)
	price_bar.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	price_bar.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	price_bar.mouse_filter = Control.MOUSE_FILTER_PASS
	price_row.add_child(price_bar)

	seigniorage_label = _small_label(13)
	add_child(seigniorage_label)
	recoinage_label = _small_label(13)
	add_child(recoinage_label)
	changed_label = _small_label(12)
	changed_label.text = "Monnaie déjà changée cette année : prochain changement l'an prochain."
	changed_label.add_theme_color_override("font_color", MUTED_COLOR)
	changed_label.hide()
	add_child(changed_label)

	selector = GridContainer.new()
	selector.columns = 2
	selector.add_theme_constant_override("h_separation", 4)
	selector.add_theme_constant_override("v_separation", 2)
	add_child(selector)

	error_label = _small_label(12)
	error_label.add_theme_color_override("font_color", ERROR_COLOR)
	error_label.hide()
	add_child(error_label)

	explanation_label = RichTextLabel.new()
	explanation_label.bbcode_enabled = true
	explanation_label.fit_content = true
	explanation_label.scroll_active = false
	explanation_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	explanation_label.custom_minimum_size = Vector2(300, 0)
	explanation_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	explanation_label.add_theme_color_override("default_color", RichTooltip.INK)
	for key in ["normal_font_size", "bold_font_size", "italics_font_size"]:
		explanation_label.add_theme_font_size_override(key, UiType.size(UiType.CAPTION))
	add_child(explanation_label)


func _ready() -> void:
	explanation_label.text = CodexText.format("[i]%s[/i]" % EXPLANATION)
	var bubbles := get_node_or_null("/root/CodexBubbles")
	if bubbles != null:
		bubbles.call("attach", explanation_label)


func _small_label(font_size: int, wrap := true) -> Label:
	var label := Label.new()
	if wrap:
		label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		label.custom_minimum_size = Vector2(300, 0)
	label.add_theme_font_size_override("font_size", font_size)
	return label


## Remplit la section pour `faction` (`""` = joueur) ; `player_owned` faux → lecture seule.
## Section masquée si la simulation n'expose pas `get_coinage` (mock).
func show_for(faction: String, player_owned: bool = true, sim: Object = null) -> void:
	_faction = faction
	read_only = not player_owned
	_sim = sim if sim != null else _facade_sim()
	if _sim == null or not _sim.has_method("get_coinage"):
		hide()
		return
	coinage = _sim.call("get_coinage", faction)
	if coinage.is_empty():
		hide()
		return
	show()
	current_label.text = str(coinage.get("label", coinage.get("level", "")))
	var price := int(coinage.get("price_level", 100))
	price_label.text = "Niveau des prix : %d" % price
	price_bar.value = clampf(price, PRICE_MIN, PRICE_MAX)
	var tint := Color(0.25, 0.45, 0.20).lerp(Color(0.60, 0.15, 0.10), clampf((price - PRICE_MIN) / 150.0, 0.0, 1.0))
	var fill := StyleBoxFlat.new()
	fill.bg_color = tint
	price_bar.add_theme_stylebox_override("fill", fill)
	var price_tip := "Prix de 1337 = 100. Recrutement, entretien et constructions coûtent ×%s ; les rentes fixes des bourgeois et du clergé en souffrent." % str(snappedf(price / 100.0, 0.01)).replace(".", ",")
	price_label.tooltip_text = price_tip
	price_bar.tooltip_text = price_tip
	seigniorage_label.text = "Seigneuriage : %s / saison (saison passée : %s)" % [_pounds(int(coinage.get("seigniorage", 0)), true), _pounds(int(coinage.get("seigniorage_last_turn", 0)), true)]
	recoinage_label.text = "Refonte : %s / saison (saison passée : %s)" % [_pounds(int(coinage.get("recoinage", 0))), _pounds(int(coinage.get("recoinage_last_turn", 0)))]
	var changed := bool(coinage.get("changed_this_year", false))
	changed_label.visible = changed and not read_only
	_fill_selector(changed)


func _fill_selector(changed: bool) -> void:
	for child in selector.get_children():
		selector.remove_child(child)
		child.queue_free()
	level_buttons.clear()
	selector.visible = not read_only
	if read_only:
		return
	for option in coinage.get("options", []):
		var level := str(option.get("level", ""))
		var button := RichButton.new()
		var short := str(option.get("label", level)).trim_prefix("Monnaie ")
		button.text = short.left(1).to_upper() + short.substr(1)
		button.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		button.custom_minimum_size = Vector2(0, 26)
		button.toggle_mode = true
		button.button_pressed = bool(option.get("current", false))
		# Un niveau refusé (déjà changée cette année) reste cliquable : la simulation motive le refus.
		button.disabled = bool(option.get("current", false))
		button.tooltip_text = RichTooltip.coinage(option, changed)
		button.pressed.connect(func() -> void: request_level(level))
		selector.add_child(button)
		level_buttons[level] = button


## Ordre `set_coinage` ; en cas de refus, message de la simulation affiché en rouge.
func request_level(level: String) -> Dictionary:
	if _sim == null or read_only:
		return {}
	last_result = _sim.call("submit_order", {"type": "set_coinage", "level": level})
	var ok := bool(last_result.get("ok", false))
	show_for(_faction, not read_only, _sim)
	error_label.visible = not ok
	error_label.text = "Refusé : %s" % str(last_result.get("error", "?")) if not ok else ""
	if ok:
		coinage_changed.emit(level)
	return last_result


static func _pounds(value: int, signed := false) -> String:
	var sign := "+" if signed and value > 0 else ""
	return "%s%s %s" % [sign, RichTooltip.thousands(value), RichTooltip.POUND]


func _facade_sim() -> Object:
	var loop := Engine.get_main_loop() as SceneTree
	var facade: Node = loop.root.get_node_or_null("/root/SimFacade") if loop != null else null
	return facade.get("sim") if facade != null else null
