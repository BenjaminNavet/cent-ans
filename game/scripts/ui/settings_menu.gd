class_name SettingsMenu
extends PanelContainer

## F3 — fenêtre des réglages (menu de départ, menu pause, menu de la carte) : quatre onglets
## (Affichage, Carte, Partie, Son) liés à l'autoload `Settings`, qui persiste chaque
## changement dans `user://settings.cfg` et l'applique aussitôt. Échap ou « Fermer » :
## signal `closed` (la fenêtre se libère).
## Lot P2e (ADR 0097, bible DA § 12.1) : rejoint la zone `MODAL` de `UiLayout` dans `_ready`
## (plus de veil ni de `CenterContainer` propres) — vaut pour ses deux points d'ouverture
## (`pause_menu.gd`, `flow_controller.gd`, ce dernier hors lot) puisque le rattachement se fait
## ici, une seule fois. Tailles par `UiType` ; ouverture et fermeture par `UiMotion`.

signal closed

var settings: Node = null
## Onglet ouvert d'emblée (nom d'onglet, ex. « Son » pour Menu → Son…) ; "" = le premier.
var initial_tab: String = ""
var _controls: Dictionary = {}  # clé → contrôle
## NT6d : réaffectation des touches (action en attente de sa touche, boutons, avertissement).
var _capturing: String = ""
var _key_buttons: Dictionary = {}
var _key_notice: Label = null


func _ready() -> void:
	theme = load("res://scenes/ui/parchment_theme.tres")
	mouse_filter = Control.MOUSE_FILTER_STOP
	process_mode = Node.PROCESS_MODE_ALWAYS
	custom_minimum_size = Vector2(700, 500)
	settings = get_node_or_null("/root/Settings")
	_build()
	UiZones.put(UiZones.Zone.MODAL, self)
	UiMotion.fade_in(self)


## Contenu de la fenêtre (onglets, boutons) ; reconstruit sur place par `_on_reset`.
func _build() -> void:
	var box := UiBuild.vbox(10)
	add_child(box)
	var title := UiBuild.label("Réglages")
	UiType.apply(title, UiType.TITLE)
	box.add_child(title)
	if settings == null:
		var missing := UiBuild.label("Réglages indisponibles (autoload Settings absent).", 0, null, false, 0.0, box)
	else:
		var tabs := TabContainer.new()
		tabs.name = "Tabs"
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
	var buttons := UiBuild.hbox(10)
	buttons.alignment = BoxContainer.ALIGNMENT_END
	box.add_child(buttons)
	var reset := UiBuild.button("Valeurs par défaut", _on_reset, buttons)
	var close_button := UiBuild.button("Fermer", close)
	close_button.name = "CloseButton"
	buttons.add_child(close_button)


func _unhandled_input(event: InputEvent) -> void:
	if visible and event.is_action_pressed("ui_cancel"):
		close()
		get_viewport().set_input_as_handled()


func close() -> void:
	closed.emit()
	UiMotion.fade_out(self, UiMotion.DURATION, true)


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
	var label := UiBuild.label(text)
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
	# PB3b (ADR 0080) : mise à l'échelle 3D MetalFX (FSR hors Metal).
	var upscale_labels: Array = Array(RenderQuality.UPSCALE_LABELS).duplicate()
	upscale_labels[0] = "Automatique (%s)" % RenderQuality.upscale_label(RenderQuality.preset_upscale(RenderQuality.preset()))
	_options(grid, "video/upscale", "Mise à l'échelle", Array(RenderQuality.UPSCALE_CHOICES), upscale_labels,
		"Calcule l'image 3D en plus petit puis l'agrandit avec MetalFX (puces Apple ; FSR ailleurs) : plus d'images par seconde, image un peu plus douce. Qualité : trois quarts de la définition. Performance : moitié de la définition, pour les machines modestes ou les très grands écrans. Automatique : selon la qualité graphique. L'interface reste nette dans tous les cas.")
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
	_check(grid, "map/stance_fill", "Lavis des positions diplomatiques", "En vue 3D, chaque province est teintée selon son détenteur : or vos terres, vert vos alliés et vassaux, rouge vos ennemis en guerre, gris léger les autres. Une province occupée garde les hachures de l'occupant au bord.")
	_options(grid, "map/ai_moves", "Mouvements de l'IA", Array(AiTurnReplay.MODES), Array(AiTurnReplay.MODE_LABELS),
		"En fin de tour, les armées des autres factions que vous voyez marchent sur la carte. Suivre : la caméra se porte sur celles qui vous concernent puis revient. Montrer : sans bouger la caméra. Masquer : fin de tour immédiate. Espace passe l'animation.")
	var replay_speeds: Array = AiTurnReplay.speeds()
	_options(grid, "map/ai_moves_speed", "Vitesse des mouvements de l'IA", replay_speeds,
		replay_speeds.map(func(value: float) -> String: return "×%s" % String.num(value, 1).trim_suffix(".0")))
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
	_options(grid, "battle/max_figures", "Figurines maximum", budgets, budgets.map(func(count: int) -> String: return Money.digits(count)),
		"Nombre maximal de figurines dessinées sur tout le champ de bataille. Si les armées sont plus nombreuses, la taille des unités est réduite pour tenir dans ce plafond. Baissez-le si les grandes batailles ralentissent.")
	_check(grid, "battle/cinematic", "Plan cinématique au premier choc", "Quelques secondes de caméra rapprochée sur le premier choc entre deux lignes, puis retour à votre vue. Espace ou Échap pour passer.")
	_check(grid, "battle/cinematic_slowmo", "Ralenti du plan cinématique", "Le premier choc est montré au ralenti ; la bataille reprend son allure ensuite.")


## AU1 : un curseur par bus (Général, Musique, Ambiance, Bataille, Interface, Voix).
func _build_sound(grid: GridContainer) -> void:
	for spec in AudioBuses.PLAYER_BUSES:
		var bus_name: String = spec[0]
		_slider(grid, str(spec[1]), float(settings.call("bus_volume", bus_name)), 0.0, 1.0, 0.05, func(value: float) -> void: settings.call("set_bus_volume", bus_name, value))
	# VO1 : voix.
	_check(grid, "voice/advisor", "Conseiller", "Le chroniqueur Jean le Bel commente le premier tour, la première bataille, le premier siège et les alertes importantes (voix et sous-titre).")
	_check(grid, "voice/barks", "Répliques des unités", "Les régiments répondent à la sélection et aux ordres, crient à la charge et en déroute.")


## Lot U7 + NT6d : disposition du clavier (préréglage), puis liste des actions de l'`InputMap`
## avec leur touche, un bouton « Changer » qui capture la prochaine touche (conflit : les deux
## actions échangent leur touche, avec avertissement) et « Rétablir par défaut ».
func _build_controls(grid: GridContainer) -> void:
	_options(grid, "input/layout", "Disposition du clavier", Array(ShortcutSheet.LAYOUTS), Array(ShortcutSheet.LAYOUT_LABELS),
		"Change les lettres affichées sur les boutons et dans l'aide. Les touches de déplacement suivent leur place sur le clavier (Z Q S D en AZERTY, W A S D en QWERTY). Les touches réaffectées ci-dessous s'ajoutent à ce préréglage.")
	var sheet := GridContainer.new()
	sheet.name = "ShortcutGrid"
	sheet.columns = 3
	sheet.add_theme_constant_override("h_separation", 12)
	sheet.add_theme_constant_override("v_separation", 2)
	var scroll := ScrollContainer.new()
	scroll.custom_minimum_size = Vector2(0, 230)
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	scroll.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	scroll.add_child(sheet)
	_label(grid, "Touches de la carte")
	var column := VBoxContainer.new()
	column.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_key_notice = Label.new()
	_key_notice.name = "KeyNotice"
	_key_notice.add_theme_color_override("font_color", HudStyle.RUBRIC)
	_key_notice.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_key_notice.visible = false
	column.add_child(_key_notice)
	column.add_child(scroll)
	var restore := UiBuild.button("Rétablir par défaut", _on_restore_keys)
	restore.name = "RestoreKeys"
	restore.tooltip_text = "Remet toutes les touches de la carte à leur valeur d'origine."
	column.add_child(restore)
	grid.add_child(column)
	_fill_shortcuts(sheet)
	settings.changed.connect(func(key: String) -> void:
		if (key == "input/layout" or key == KeyBindings.SETTING_KEY) and is_instance_valid(sheet):
			_fill_shortcuts(sheet))


func _fill_shortcuts(sheet: GridContainer) -> void:
	_key_buttons.clear()
	for child in sheet.get_children():
		sheet.remove_child(child)
		child.queue_free()
	for section in KeyBindings.sections():
		var title := UiBuild.label(str(section["title"]), 0, HudStyle.RUBRIC, false, 0.0, sheet)
		sheet.add_child(Control.new())
		sheet.add_child(Control.new())
		for row in section["actions"]:
			var action: String = row[0]
			var what := UiBuild.label(str(row[1]))
			UiType.apply(what, UiType.CAPTION)
			what.size_flags_horizontal = Control.SIZE_EXPAND_FILL
			sheet.add_child(what)
			var keys := UiBuild.label(ShortcutSheet.action_keys(action))
			UiType.apply(keys, UiType.CAPTION)
			keys.custom_minimum_size = Vector2(90, 0)
			sheet.add_child(keys)
			var change := UiBuild.button("Changer", _begin_capture.bind(action))
			change.name = "Change_" + action
			sheet.add_child(change)
			_key_buttons[action] = change


## Attend la prochaine touche pour `action` (touche principale).
func _begin_capture(action: String) -> void:
	_end_capture()
	_capturing = action
	var button := _key_buttons.get(action) as Button
	if button != null:
		button.text = "Appuyez sur une touche…"
	_show_notice("« %s » : appuyez sur la nouvelle touche (Échap pour annuler)." % KeyBindings.label_of(action))


func _end_capture() -> void:
	if _capturing != "":
		var button := _key_buttons.get(_capturing) as Button
		if button != null and is_instance_valid(button):
			button.text = "Changer"
	_capturing = ""


func _input(event: InputEvent) -> void:
	if _capturing == "":
		return
	var key := event as InputEventKey
	if key == null or not key.pressed or key.echo:
		return
	get_viewport().set_input_as_handled()
	if key.keycode == KEY_ESCAPE:
		_end_capture()
		_show_notice("")
		return
	if key.keycode in [KEY_SHIFT, KEY_CTRL, KEY_ALT, KEY_META]:
		return  # un modificateur seul n'est pas une touche
	capture_key(key)


## Applique `event` comme nouvelle touche de l'action en cours de capture (appelée par `_input`).
func capture_key(event: InputEventKey) -> void:
	var action := _capturing
	if action == "":
		return
	_end_capture()
	var code := KeyBindings.encode(event)
	var other := KeyBindings.rebind(action, 0, code, settings)
	if other != "":
		_show_notice("Touche déjà utilisée par « %s » : les deux actions ont échangé leur touche." % KeyBindings.label_of(other))
	else:
		_show_notice("")


func _on_restore_keys() -> void:
	_end_capture()
	KeyBindings.reset("", settings)
	_show_notice("Touches rétablies par défaut.")


func _show_notice(text: String) -> void:
	if _key_notice != null and is_instance_valid(_key_notice):
		_key_notice.text = text
		_key_notice.visible = text != ""


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
	# Reconstruit le contenu sur place (même nœud : les ouvreurs gardent une référence valide),
	# après le signal du bouton qui va être libéré.
	_rebuild.call_deferred()


func _rebuild() -> void:
	var tabs := find_child("Tabs", true, false) as TabContainer
	if tabs != null and tabs.get_current_tab_control() != null:
		initial_tab = str(tabs.get_current_tab_control().name)  # rester sur l'onglet ouvert
	for child in get_children():
		remove_child(child)
		child.queue_free()
	_controls.clear()
	_build()
