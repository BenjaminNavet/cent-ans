class_name SettingsMenu
extends Control

## F3 — fenêtre des réglages (menu de départ, menu pause, menu de la carte) : quatre onglets
## (Affichage, Carte, Partie, Son) liés à l'autoload `Settings`, qui persiste chaque
## changement dans `user://settings.cfg` et l'applique aussitôt. Échap ou « Fermer » :
## signal `closed` (la fenêtre se libère).

signal closed

var settings: Node = null
## Onglet ouvert d'emblée (nom d'onglet, ex. « Son » pour Menu → Son…) ; "" = le premier.
var initial_tab: String = ""
var _controls: Dictionary = {}  # clé → contrôle


func _ready() -> void:
	set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	theme = load("res://scenes/ui/parchment_theme.tres")
	mouse_filter = Control.MOUSE_FILTER_STOP
	process_mode = Node.PROCESS_MODE_ALWAYS
	settings = get_node_or_null("/root/Settings")
	var veil := ColorRect.new()
	veil.color = Color(0.05, 0.03, 0.01, 0.45)
	veil.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	add_child(veil)
	var center := CenterContainer.new()
	center.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	add_child(center)
	var panel := PanelContainer.new()
	panel.custom_minimum_size = Vector2(700, 500)
	center.add_child(panel)
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 10)
	panel.add_child(box)
	var title := Label.new()
	title.text = "Réglages"
	title.add_theme_font_size_override("font_size", 26)
	box.add_child(title)
	if settings == null:
		var missing := Label.new()
		missing.text = "Réglages indisponibles (autoload Settings absent)."
		box.add_child(missing)
	else:
		var tabs := TabContainer.new()
		tabs.size_flags_vertical = Control.SIZE_EXPAND_FILL
		box.add_child(tabs)
		_build_display(_tab(tabs, "Affichage"))
		_build_map(_tab(tabs, "Carte"))
		_build_game(_tab(tabs, "Partie"))
		_build_battle(_tab(tabs, "Bataille"))
		_build_sound(_tab(tabs, "Son"))
		_build_controls(_tab(tabs, "Commandes"))  # U7
		_build_accessibility(_tab(tabs, "Accessibilité"))  # U12
		var start_tab := tabs.get_node_or_null(initial_tab) if initial_tab != "" else null
		if start_tab != null:
			tabs.current_tab = start_tab.get_index()
	var buttons := HBoxContainer.new()
	buttons.alignment = BoxContainer.ALIGNMENT_END
	buttons.add_theme_constant_override("separation", 10)
	box.add_child(buttons)
	var reset := Button.new()
	reset.text = "Valeurs par défaut"
	reset.pressed.connect(_on_reset)
	buttons.add_child(reset)
	var close_button := Button.new()
	close_button.name = "CloseButton"
	close_button.text = "Fermer"
	close_button.pressed.connect(close)
	buttons.add_child(close_button)


func _unhandled_input(event: InputEvent) -> void:
	if visible and event.is_action_pressed("ui_cancel"):
		close()
		get_viewport().set_input_as_handled()


func close() -> void:
	closed.emit()
	queue_free()


func _tab(tabs: TabContainer, label: String) -> GridContainer:
	var margin := MarginContainer.new()
	margin.name = label
	for side in ["left", "right", "top", "bottom"]:
		margin.add_theme_constant_override("margin_" + side, 12)
	tabs.add_child(margin)
	var grid := GridContainer.new()
	grid.columns = 2
	grid.add_theme_constant_override("h_separation", 18)
	grid.add_theme_constant_override("v_separation", 10)
	margin.add_child(grid)
	return grid


func _label(grid: GridContainer, text: String, tooltip: String = "") -> void:
	var label := Label.new()
	label.text = text
	label.tooltip_text = tooltip
	label.mouse_filter = Control.MOUSE_FILTER_PASS
	label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	grid.add_child(label)


func _check(grid: GridContainer, key: String, text: String, tooltip: String = "") -> void:
	_label(grid, text, tooltip)
	var check := CheckBox.new()
	check.button_pressed = bool(settings.call("get_value", key))
	check.toggled.connect(func(on: bool) -> void: settings.call("set_value", key, on))
	grid.add_child(check)
	_controls[key] = check


func _options(grid: GridContainer, key: String, text: String, choices: Array, labels: Array, tooltip: String = "") -> void:
	_label(grid, text, tooltip)
	var option := OptionButton.new()
	var current: Variant = settings.call("get_value", key)
	var selected := -1
	for index in choices.size():
		option.add_item(str(labels[index]), index)
		if typeof(choices[index]) != typeof(current):
			continue
		if (typeof(current) == TYPE_FLOAT and is_equal_approx(float(choices[index]), float(current))) or choices[index] == current:
			selected = index
	if selected == -1:
		option.add_item(str(current), choices.size())
		choices = choices + [current]
		selected = choices.size() - 1
	option.select(selected)
	option.item_selected.connect(func(index: int) -> void: settings.call("set_value", key, choices[index]))
	grid.add_child(option)
	_controls[key] = option


func _slider(grid: GridContainer, text: String, value: float, min_value: float, max_value: float, step: float, setter: Callable, key: String = "") -> HSlider:
	_label(grid, text)
	var slider := HSlider.new()
	slider.min_value = min_value
	slider.max_value = max_value
	slider.step = step
	slider.value = value
	slider.custom_minimum_size = Vector2(220, 0)
	slider.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	slider.value_changed.connect(func(new_value: float) -> void: setter.call(new_value))
	grid.add_child(slider)
	if key != "":
		_controls[key] = slider
	return slider


func _constant(constant_name: String) -> Array:
	return Array(settings.get_script().get_script_constant_map().get(constant_name, []))


func _build_display(grid: GridContainer) -> void:
	_check(grid, "video/fullscreen", "Plein écran")
	var resolutions: Array = _constant("RESOLUTIONS")
	var labels: Array = resolutions.map(func(size: Vector2i) -> String: return "%d × %d" % [size.x, size.y])
	_options(grid, "video/resolution", "Résolution (fenêtré)", resolutions, labels)
	_check(grid, "video/vsync", "Synchronisation verticale", "Limite l'affichage à la fréquence de l'écran.")
	var detected_label: String = RenderQuality.LABELS[RenderQuality.LEVELS.find(RenderQuality.detected_level())]
	_options(grid, "video/quality", "Qualité graphique", [RenderQuality.AUTO] + Array(RenderQuality.LEVELS),
		["Automatique (%s)" % detected_label] + Array(RenderQuality.LABELS),
		"Automatique : choisie selon la carte graphique détectée. Basse : sans anticrénelage, ombres simples et proches, sans occlusion ni halo, relief sans détail fin, moitié moins d'arbres et de particules, soldats simplifiés plus tôt. Moyenne : occlusion ambiante, trois quarts des arbres et de l'herbe. Haute : lumière rebondie (SSIL), brume volumétrique par mauvais temps, tout le détail. Ultra : illumination globale (SDFGI), brume volumétrique permanente, ombres et détails plus lointains.")
	# Lot U4 : l'échelle suit la hauteur de la fenêtre ; ces réglages l'ajustent.
	var sizes: Array = _constant("UI_SIZES")
	var size_labels := {0.8: "Très petite", 0.9: "Petite", 1.0: "Normale", 1.1: "Grande", 1.25: "Très grande"}
	_options(grid, "interface/ui_size", "Taille de l'interface", sizes, sizes.map(func(value: float) -> String: return str(size_labels.get(value, "%d %%" % roundi(value * 100.0)))),
		"Échelle automatique selon la hauteur de la fenêtre (actuellement %d %%), multipliée par ce réglage." % roundi(float(settings.call("effective_ui_scale")) * 100.0))
	var texts: Array = _constant("TEXT_SIZES")
	var text_labels := {0.9: "Petite", 1.0: "Normale", 1.15: "Grande", 1.3: "Très grande"}
	_options(grid, "interface/text_size", "Taille du texte", texts, texts.map(func(value: float) -> String: return str(text_labels.get(value, "%d %%" % roundi(value * 100.0)))),
		"Agrandit les textes seuls, sans changer la taille des panneaux et des icônes.")


func _build_map(grid: GridContainer) -> void:
	_check(grid, "camera/edge_pan", "Défilement par les bords de l'écran", "Aussi basculé par F2 sur la carte.")
	_slider(grid, "Vitesse de la caméra", float(settings.call("get_value", "camera/speed")), 0.4, 2.5, 0.1,
		func(value: float) -> void: settings.call("set_value", "camera/speed", value), "camera/speed")
	_check(grid, "map/fog_of_war", "Brouillard de guerre", "Provinces hors de vue voilées, armées étrangères masquées.")
	_check(grid, "interface/season_report", "Rapport de saison en fin de tour")
	_check(grid, "interface/confirm_end_turn", "Confirmer la fin du tour")
	_check(grid, "interface/next_hint", "Conseil : que faire maintenant", "Encart en haut à gauche de la carte qui propose l'action la plus utile du moment (clic : l'exécute). Masqué pendant le tutoriel.")
	_options(grid, "interface/news_filter", "Nouvelles reçues", Array(NewsInterest.MODES), Array(NewsInterest.MODE_LABELS),
		"Lettres scellées et bandeau du haut. Le journal garde toutes les nouvelles.")


func _build_game(grid: GridContainer) -> void:
	var choices: Array = _constant("AUTOSAVE_CHOICES")
	var labels: Array = choices.map(func(turns: int) -> String:
		if turns == 0:
			return "Désactivée"
		return "Chaque tour" if turns == 1 else "Tous les %d tours" % turns)
	_options(grid, "game/autosave_interval", "Sauvegarde automatique", choices, labels, "Trois emplacements tournants (auto_1 à auto_3).")
	_check(grid, "game/interactive_battles", "Livrer ses batailles en 3D", "Décoché : toutes les batailles du joueur sont résolues automatiquement.")
	_check(grid, "tutorial/enabled", "Tutoriel des premiers tours", "Guide pas à pas au début d'une nouvelle partie. Décoché : jamais affiché.")


## BV1 : sang et taille des unités (appliqués à la bataille suivante).
func _build_battle(grid: GridContainer) -> void:
	_options(grid, "battle/blood", "Sang", _constant("BLOOD_CHOICES"), ["Désactivé", "Modéré", "Complet"],
		"Gerbes, flaques au sol et cadavres ensanglantés. Modéré : plus discret, sans éclaboussures, traînées ni démembrements. Complet : démembrements sur les coups critiques.")
	_options(grid, "battle/unit_size", "Taille des unités", _constant("UNIT_SIZES"), ["Petite (× 0,5)", "Normale", "Grande (× 1,5)", "Ultra (× 2,5)", "Épique (× 4)"],
		"Figurines dessinées par soldat simulé : les effectifs et l'équilibre ne changent pas. Ultra et Épique sont exigeants pour la carte graphique (Épique : masses serrées, pour les grandes batailles).")
	var budgets: Array = _constant("MAX_FIGURES_CHOICES")
	_options(grid, "battle/max_figures", "Figurines maximum", budgets, budgets.map(func(count: int) -> String: return _thousands(count)),
		"Nombre maximal de figurines dessinées sur tout le champ de bataille. Si les armées sont plus nombreuses, la taille des unités est réduite pour tenir dans ce plafond. Baissez-le si les grandes batailles ralentissent.")
	_check(grid, "battle/cinematic", "Plan cinématique au premier choc", "Quelques secondes de caméra rapprochée sur le premier choc entre deux lignes, puis retour à votre vue. Espace ou Échap pour passer.")
	_check(grid, "battle/cinematic_slowmo", "Ralenti du plan cinématique", "Le premier choc est montré au ralenti ; la bataille reprend son allure ensuite.")


## 15000 -> « 15 000 » (espace insécable des milliers).
static func _thousands(count: int) -> String:
	var digits := str(count)
	var grouped := ""
	while digits.length() > 3:
		grouped = "\u00a0" + digits.right(3) + grouped
		digits = digits.left(digits.length() - 3)
	return digits + grouped


## AU1 : un curseur par bus (Général, Musique, Ambiance, Bataille, Interface, Voix).
func _build_sound(grid: GridContainer) -> void:
	for spec in AudioBuses.PLAYER_BUSES:
		var bus_name: String = spec[0]
		_slider(grid, str(spec[1]), float(settings.call("bus_volume", bus_name)), 0.0, 1.0, 0.05, func(value: float) -> void: settings.call("set_bus_volume", bus_name, value))
	# VO1 : voix.
	_check(grid, "voice/advisor", "Conseiller", "Le chroniqueur Jean le Bel commente le premier tour, la première bataille, le premier siège et les alertes importantes (voix et sous-titre).")
	_check(grid, "voice/barks", "Répliques des unités", "Les régiments répondent à la sélection et aux ordres, crient à la charge et en déroute.")


## Lot U7 : disposition du clavier et fiche des raccourcis, lue dans l'InputMap.
func _build_controls(grid: GridContainer) -> void:
	_options(grid, "input/layout", "Disposition du clavier", Array(ShortcutSheet.LAYOUTS), Array(ShortcutSheet.LAYOUT_LABELS),
		"Change les lettres affichées sur les boutons et dans l'aide. Les touches de déplacement suivent leur place sur le clavier (Z Q S D en AZERTY, W A S D en QWERTY).")
	var sheet := GridContainer.new()
	sheet.name = "ShortcutGrid"
	sheet.columns = 2
	sheet.add_theme_constant_override("h_separation", 18)
	sheet.add_theme_constant_override("v_separation", 2)
	var scroll := ScrollContainer.new()
	scroll.custom_minimum_size = Vector2(0, 260)
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	scroll.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	scroll.add_child(sheet)
	_label(grid, "Raccourcis de la carte")
	grid.add_child(scroll)
	_fill_shortcuts(sheet)
	settings.changed.connect(func(key: String) -> void:
		if key == "input/layout" and is_instance_valid(sheet):
			_fill_shortcuts(sheet))


func _fill_shortcuts(sheet: GridContainer) -> void:
	for child in sheet.get_children():
		sheet.remove_child(child)
		child.queue_free()
	for section in ShortcutSheet.sections():
		var title := Label.new()
		title.text = str(section["title"])
		title.add_theme_color_override("font_color", HudStyle.RUBRIC)
		sheet.add_child(title)
		sheet.add_child(Control.new())
		for line in section["lines"]:
			var keys := Label.new()
			keys.text = str(line[0])
			keys.add_theme_font_size_override("font_size", 15)
			keys.custom_minimum_size = Vector2(90, 0)
			sheet.add_child(keys)
			var what := Label.new()
			what.text = str(line[1])
			what.add_theme_font_size_override("font_size", 15)
			sheet.add_child(what)


## Lot U12 : mode daltonien, animations réduites, contraste renforcé.
func _build_accessibility(grid: GridContainer) -> void:
	_check(grid, Accessibility.KEY_COLORBLIND, "Mode daltonien",
		"Ajoute motifs et symboles aux couleurs : carte diplomatique hachurée, relations et moral marqués de symboles.")
	_check(grid, Accessibility.KEY_REDUCE_MOTION, "Réduire les animations",
		"Supprime les fondus et les travellings de caméra.")
	_check(grid, Accessibility.KEY_HIGH_CONTRAST, "Contraste renforcé",
		"Encre plus sombre, parchemin plus clair, bords plus épais.")


func _on_reset() -> void:
	if settings == null:
		return
	settings.call("reset_to_defaults")
	# Reconstruit la fenêtre sur les nouvelles valeurs.
	var parent := get_parent()
	var fresh := SettingsMenu.new()
	for connection in closed.get_connections():
		fresh.closed.connect(connection["callable"])
	for connection in closed.get_connections():
		closed.disconnect(connection["callable"])
	parent.add_child(fresh)
	queue_free()
