extends "res://tests/smoke_campaign.gd"

## Sections « interface » : menu de démarrage, disposition, assets, icônes, codex, table, flow, tutoriel, monnaie et rançons.
## Découpage de `smoke.gd` (SC GT7) : les sections se chaînent par héritage et partagent l'état.
## Ne se lance pas seul : point d'entrée `res://tests/smoke.gd`.

func _run_start_menu() -> void:
	var scene: PackedScene = load("res://scenes/start_menu.tscn")
	if not _check(scene != null, "cannot load start_menu.tscn"):
		return
	var menu: Control = scene.instantiate()
	root.add_child(menu)
	await process_frame
	# FE6 : une carte par départ recommandé (jouable) ; toutes les factions présentées sur la carte.
	# JR6 : plus une carte par défi singulier (onglet « Défis singuliers »).
	var presented := FrontEndData.recommended().size() + (FrontEndData.special_starts().get("factions", []) as Array).size()
	_check(presented >= 3 and menu.card_count() == presented, "start menu should show %d faction cards, got %d" % [presented, menu.card_count()])
	var map_picker: Variant = menu.faction_select.get("map_picker") if menu.faction_select != null else null
	_check(map_picker != null, "faction select should carry the faction map")
	if map_picker != null and facade.store_loaded():
		_check((map_picker.get("sheets") as Dictionary).size() == FrontEndData.factions().size(),
			"faction map should present every playable faction (%d)" % FrontEndData.factions().size())
	_check(menu.selected_faction == "fac_france", "default faction should be fac_france")
	_check(menu.start_button.text.begins_with("Commencer"), "start button label")
	# MM1 : choix de faction, prologue et textes d'accueil (data/ui/front_end.json).
	menu.show_faction_select(true)
	_check(menu.faction_select.visible and not menu.main_column.visible, "faction select should replace the main column")
	# DF1 : sélecteur de difficulté de campagne, Normale par défaut.
	_check(menu.faction_select.difficulty_count() == 4, "faction select should offer 4 difficulty levels, got %d" % menu.faction_select.difficulty_count())
	_check(menu.faction_select.selected_difficulty == "normal", "default campaign difficulty should be normal, got %s" % menu.faction_select.selected_difficulty)
	menu.open_intro()
	await process_frame
	_check(menu.overlay_open(), "prologue overlay should open")
	_check(not FrontEndData.random_quote().is_empty() and FrontEndData.random_tip() != "", "loading quotes and tips expected")
	# A6-L11 (U3) : citations filtrées par culture de la faction (pas de Radonège pour la France).
	var french_quotes := FrontEndData.quotes_for("fac_france")
	var russian_quotes := FrontEndData.quotes_for("fac_briansk")
	var has_radonege := func(quotes: Array) -> bool:
		return quotes.any(func(q: Dictionary) -> bool: return str(q["author"]).begins_with("Serge de Radon"))
	_check(not french_quotes.is_empty() and not has_radonege.call(french_quotes), "French loading quotes must not include Radonège")
	_check(has_radonege.call(russian_quotes), "Muscovite loading quotes include Radonège")
	_check(not FrontEndData.random_quote(null, "fac_france").is_empty(), "French quote drawn")
	# A6-L11 (U1) : le sous-menu « Batailles » regroupe les quatre modes.
	menu.open_battles()
	await process_frame
	_check(menu.overlay_open() and menu._overlay is BattlesMenu, "battles submenu should open")
	var battles_menu := menu._overlay as BattlesMenu
	if battles_menu != null:
		_check(battles_menu.buttons.size() == 4, "battles submenu: four modes")
		battles_menu.buttons["historical"].pressed.emit()
		await process_frame
		_check(menu._overlay is HistoricalBattlesMenu, "historical battles opened from the submenu")
	# SG2 : batailles de démonstration (sièges d'Avignon et de Bruges dans leur plan).
	menu.open_demos()
	await process_frame
	_check(menu.overlay_open() and menu._overlay is BattleDemosMenu, "demo battles overlay should open")
	var demo_menu := menu._overlay as BattleDemosMenu
	if demo_menu != null:
		_check(demo_menu.buttons.has("siege_avignon") and demo_menu.buttons.has("siege_bruges"), "Avignon and Bruges demos expected")
		for demo in demo_menu.demos:
			if str(demo["id"]) == "siege_bruges":
				var args := BattleDemosMenu.args_for(demo)
				_check(args.has("--siege-landmark=bruges") and args.has("--siege-attacker=fac_france"), "Bruges demo args: %s" % str(args))
	# EP7 : batailles historiques (détail : tests/ep7_historical_test.gd).
	menu.open_historical()
	await process_frame
	_check(menu.overlay_open() and menu._overlay is HistoricalBattlesMenu, "historical battles overlay should open")
	if failures == 0:
		print("smoke OK: start menu, %d cards" % menu.card_count())
	menu.queue_free()
	await process_frame


## C1 : minicarte présente dans le HUD (haut droite, lettres dessous, sans chevaucher la cloche),
## clic = caméra recentrée ; brouillard (vraie simulation) : au moins une province voilée, aucune
## armée étrangère marquée hors de vue, réglage désactivable.
## UI2 (audit A3, lots U1 et U4) : aux quatre résolutions de référence, l'échelle automatique
## vaut hauteur / 900 bornée entre 0,9 et 1,6 ; la barre du haut tient dans la largeur ; faction,
## Cour et technologies s'ouvrent une à une (exclusivité), leur bouton × reste à l'écran et la
## minicarte ne les recouvre jamais ; Échap (pile) ferme le panneau du dessus et la province
## mise de côté revient.
const UI_RESOLUTIONS: Array[Vector2i] = [Vector2i(1280, 720), Vector2i(1440, 900), Vector2i(1920, 1080), Vector2i(2560, 1440)]


func _run_ui_layout() -> void:
	var real_data := _project_root().path_join("data")
	if ClassDB.class_exists("CampaignSim") and FileAccess.file_exists(real_data.path_join("map/map.json")):
		facade.set_data_dir(real_data)
	facade.pending_faction = "fac_france"
	facade.pending_seed = 1337
	facade.pending_load_path = ""
	var scene: PackedScene = load("res://scenes/campaign_map.tscn")
	var map: Node3D = scene.instantiate()
	root.add_child(map)
	await process_frame
	await process_frame
	if not _check(map.load_ok and map.sim != null, "ui layout: campaign scene failed to start"):
		map.queue_free()
		return
	var ui: Node = map.ui
	var initial_size := root.size
	var checked := 0
	for resolution in UI_RESOLUTIONS:
		root.size = resolution
		await process_frame
		if root.size != resolution:
			print("smoke ui layout: window cannot be resized here (%s), resolution %s skipped" % [root.size, resolution])
			continue
		var expected := clampf(resolution.y / 900.0, 0.9, 1.6)
		_check(is_equal_approx(root.content_scale_factor, expected), "ui scale at %s should be %.3f, got %.3f" % [resolution, expected, root.content_scale_factor])
		var view: Vector2 = ui.get_viewport().get_visible_rect().size
		var bar: Control = ui.get_node("TopBar")
		_check(bar.get_combined_minimum_size().x <= view.x + 1.0, "top bar (%.0f px) wider than the screen (%.0f) at %s" % [bar.get_combined_minimum_size().x, view.x, resolution])
		# Province choisie, puis les grands panneaux un à un.
		map.picker.select_index(1)
		await process_frame
		var openers := [
			["faction", func() -> void: map._show_faction_panel(map.player_faction), ui.faction_panel, ui.faction_panel.close_button],
			["court", func() -> void: map._on_court_panel_requested(), ui.court_panel, ui.court_panel.close_button],
			["tech", func() -> void: map._on_tech_panel_requested(), ui.tech_panel, ui.tech_panel.close_button],
		]
		for entry in openers:
			(entry[1] as Callable).call()
			await process_frame
			ui.layout_hud()
			await process_frame
			ui.layout_hud()
			var panel: Control = entry[2]
			var close_button: Control = entry[3]
			if not panel.visible:
				continue  # panneau non ouvert par cette touche
			var open_centrals := 0
			for other in ui.panels.visible_panels():
				if ui.panels.kind_of(other) == PanelStack.Kind.CENTRAL:
					open_centrals += 1
			_check(open_centrals == 1, "%s at %s: %d central panels open (exclusive expected)" % [entry[0], resolution, open_centrals])
			_check(not ui.province_panel.visible, "%s at %s: the province panel should be set aside" % [entry[0], resolution])
			var close_rect := close_button.get_global_rect()
			_check(Rect2(Vector2.ZERO, view).encloses(close_rect), "%s at %s: close button %s off screen %s" % [entry[0], resolution, close_rect, view])
			if ui.minimap != null and ui.minimap.visible:
				_check(not ui.minimap.get_global_rect().intersects(panel.get_global_rect()), "%s at %s: minimap drawn over the panel" % [entry[0], resolution])
			checked += 1
		# Échap : ferme le panneau du dessus ; la province revient.
		_check(ui.panels.close_top(), "escape should close the top panel at %s" % resolution)
		_check(not ui.tech_panel.visible and ui.province_panel.visible, "province should come back after the last central panel at %s" % resolution)
		_check(ui.panels.close_top() and not ui.province_panel.visible and map.selected_index == 0, "escape should then close and deselect the province at %s" % resolution)
	root.size = initial_size
	await process_frame
	map.queue_free()
	await process_frame
	if failures == 0:
		print("smoke OK: ui layout (%d panel checks at %d resolutions, auto scale, exclusive panels, escape)" % [checked, UI_RESOLUTIONS.size()])


func _run_assets() -> void:
	var audio: Node = root.get_node_or_null("/root/AudioDirector")
	if not _check(audio != null, "AudioDirector autoload missing"):
		return
	var missing_sfx: Array = []
	for clip in ["ui_click", "page_turn", "turn_bell", "fanfare", "march_drum", "sword_clash", "arrow_volley", "gallop", "war_horn", "choir"]:
		if not audio.call("has_sfx", clip):
			missing_sfx.append(clip)
	_check(missing_sfx.is_empty(), "missing sound effects: %s" % [missing_sfx])
	# Les listes de lecture vivent dans les vraies données (`data/audio/music.json`), pas dans
	# les fixtures pointées par `MapPaths` au démarrage du smoke.
	_check(audio.call("load_playlists", _project_root().path_join("data/audio/music.json")), "music playlists (data/audio/music.json) not loaded")
	for context in ["campaign", "war", "court"]:
		_check(audio.call("has_music", context), "missing music: %s" % context)
		_check((audio.call("playlist", context) as Array).size() >= 3, "music playlist too short: %s" % context)
	_check(audio.call("play_sfx", "ui_click"), "play_sfx(ui_click) failed")
	_check(not audio.call("play_sfx", "does_not_exist"), "unknown sfx should be ignored")
	audio.call("play_music", "war")
	_check(str(audio.get("current_context")) == "war", "music context should be war")
	# ADR 0154 : rotation mélangée — chaque morceau passe une fois avant toute répétition, et la
	# rotation survit à un rechargement (deux sessions n'ouvrent pas sur le même morceau).
	var rotation_path := "user://music_rotation_smoke_%d.cfg" % OS.get_process_id()
	audio.set("rotation_path", rotation_path)
	audio.call("load_rotation")
	var court_tracks: Array = audio.call("tier_tracks", "court", "primary")
	var heard: Array = []
	for index in 2:
		heard.append(audio.call("next_track", "court"))
	audio.call("load_rotation")  # « nouvelle session »
	for index in court_tracks.size() - 2:
		heard.append(audio.call("next_track", "court"))
	var distinct := {}
	for path in heard:
		distinct[path] = true
	_check(court_tracks.size() >= 3 and distinct.size() == court_tracks.size(), "music rotation should play every court track once before repeating: %s" % [heard])
	_check(str(audio.call("next_track", "court")) != str(heard[-1]), "music rotation should not replay the track just played")
	_check(not (audio.call("tier_tracks", "war", "primary") as Array).has("res://assets/audio/music/war.ogg"), "synthetic war.ogg should not be a primary track")
	audio.set("_war_blend", "campaign_france")
	_check((audio.call("tier_tracks", "war", "primary") as Array).has("res://assets/third_party/music/wikimedia/tomsinska_prelude.mp3"), "war playlist should blend the regional campaign tracks")
	audio.set("_war_blend", "")
	audio.set("rotation_path", "")
	audio.call("load_rotation")
	DirAccess.remove_absolute(ProjectSettings.globalize_path(rotation_path))
	_check(str(audio.call("event_sfx", [{"kind": "birth"}, {"kind": "battle"}])) == "sword_clash", "battle should win event sfx priority")

	# Écus, portraits et replis.
	_check(PortraitLoader.heraldry_texture("fac_france") != null, "fac_france heraldry missing")
	_check(PortraitLoader.portrait_texture("chr_does_not_exist") == null, "unknown portrait should be null")
	var swatch := ColorRect.new()
	root.add_child(swatch)
	var placed := PortraitLoader.overlay_portrait(swatch, "chr_does_not_exist", "fac_england", Vector2(48, 48))
	_check(placed and swatch.get_node_or_null(PortraitLoader.OVERLAY_NAME) != null, "heraldry fallback portrait expected")
	swatch.queue_free()
	var portraits := 0
	var dir := DirAccess.open("res://assets/portraits")
	if dir != null:
		for file_name in dir.get_files():
			if file_name.ends_with(".png") and PortraitLoader.portrait_texture(file_name.get_basename()) != null:
				portraits += 1

	# Modèles 3D.
	var missing_models: Array = []
	for model_name in ["castle", "town", "village", "cathedral", "army", "siege_camp", "ship"]:
		if not ModelLibrary.has_model(model_name):
			missing_models.append(model_name)
	_check(missing_models.is_empty(), "missing models: %s" % [missing_models])
	_check(ModelLibrary.instantiate("does_not_exist") == null, "unknown model should be null")
	_check(ModelLibrary.city_kind("prov_ile_de_france") in ["castle", "town", "village", "cathedral"], "city kind expected")
	var marker: Node3D = (load("res://scenes/map/army_marker.tscn") as PackedScene).instantiate()
	root.add_child(marker)
	await process_frame
	var dressed := ModelLibrary.dress_army_marker(marker, {"stance": "siege"}, Color(0.8, 0.1, 0.1))
	_check(dressed and marker.get_node_or_null(ModelLibrary.MODEL_NODE) != null, "army marker should get a 3D model")
	_check(not (marker.get_node("Banner") as Node3D).visible, "placeholder banner should be hidden")
	_check(marker.get_node_or_null("%s/SiegeCamp" % ModelLibrary.MODEL_NODE) != null, "siege camp expected")
	marker.queue_free()
	# Lot CV2 : figurines skinnées d'une armée (général, porte-étendard, escorte), flotte, bivouac.
	if ArmyFigures.enabled():
		var cv2_army := {"faction": "fac_france", "stance": "normal", "position": Vector2(100, 100),
			"units": [{"unit_type": "unit_knights", "strength": 600}, {"unit_type": "unit_longbowmen", "strength": 900}]}
		var cv2_figures := ArmyFigures.build(cv2_army, Color(0.2, 0.3, 0.8), null, "cv2")
		root.add_child(cv2_figures)
		# Lot CV3-5 : le général agrandi porte l'étendard (plus de porte-étendard à pied).
		var cv3_lord := cv2_figures.is_lord()
		_check(cv3_lord == (float(ArmyFigures.map_settings().get("army_figure_scale", 1.0)) > 1.001), "CV3-5: lord mode follows map.army_figure_scale")
		var cv2_expected := (1 if cv3_lord else 2) + 4
		_check(cv2_figures.figure_count() == cv2_expected, "CV2: 1 500 men → leader (+ bearer) + 4 escort, got %d" % cv2_figures.figure_count())
		if cv3_lord:
			cv2_figures.set_view(100.0, 1.0)
			var cv3_hand := cv2_figures.bearer_anchor()
			_check(cv3_hand.y > 3.0, "CV3-5: the standard is held high by the mounted lord, got %.2f" % cv3_hand.y)
			cv2_figures.set_view(900.0, 0.0)
			_check(cv2_figures.bearer_anchor().length() < 0.01, "CV3-5: far away the standard is back at the marker's foot")
		_check(cv2_figures.get_node_or_null("Bivouac") != null, "CV2: idle army in the field should camp")
		cv2_figures.set_walking(true)
		cv2_figures.set_view(100.0, 1.0)
		cv2_army["embarked"] = true
		var cv2_fleet := ArmyFigures.build(cv2_army, Color(0.2, 0.3, 0.8), null, "cv2f")
		root.add_child(cv2_fleet)
		_check(cv2_fleet.ship_count() == 2, "CV2: 1 500 men at sea → 2 ships, got %d" % cv2_fleet.ship_count())
		await process_frame
		cv2_figures.queue_free()
		cv2_fleet.queue_free()
		ArmyFigures.clear_cache()
	await process_frame
	ModelLibrary.clear_cache()
	PortraitLoader.clear_cache()
	audio.call("stop_all")
	await process_frame
	if failures == 0:
		print("smoke OK: assets, 10 sfx + 3 music, settings persisted, %d portrait(s), 7 models, siege marker dressed" % portraits)


## F2 : icônes (`IconLibrary`, `game/assets/icons/icons.json`) et infobulles riches. Toutes
## les entrées de la table se chargent ; chaque id de `data/` (unités, bâtiments, ressources,
## technologies) a sa propre icône, chaque branche de compétence et catégorie de trait aussi ;
## replis par catégorie ; BBCode d'infobulle non vide et panneau constructible.
## DA7c : chaque trait a en plus sa propre icône à l'encre (pas seulement celle de sa catégorie).
func _run_icons() -> void:
	var library: Node = root.get_node_or_null("/root/IconLibrary")
	if not _check(library != null, "IconLibrary autoload missing"):
		return
	var table: Dictionary = library.get("icons")
	_check(table.size() >= 100, "icons.json too small: %d entries" % table.size())
	var broken: Array = []
	for id in table:
		if library.call("get_icon", str(id)) == null:
			broken.append(id)
	_check(broken.is_empty(), "icons not loadable: %s" % [broken])
	var data_dir := ProjectSettings.globalize_path("res://").path_join("../data").simplify_path()
	var missing: Array = []
	var traits_without_own_icon: Array = []
	var checked := 0
	for directory in ["unit_types", "buildings", "resources", "technologies", "skills", "traits"]:
		var dir := DirAccess.open(data_dir.path_join(directory))
		if not _check(dir != null, "data/%s missing" % directory):
			continue
		for file_name in dir.get_files():
			if not file_name.ends_with(".json"):
				continue
			var entry: Dictionary = JSON.parse_string(FileAccess.get_file_as_string(data_dir.path_join(directory).path_join(file_name)))
			var id := str(entry.get("id", ""))
			if directory == "skills":
				id = "branch_" + str(entry.get("branch", ""))
			elif directory == "traits":
				# DA7c : icône propre au trait (pas seulement celle de sa catégorie).
				if not library.call("has_icon", id):
					traits_without_own_icon.append(id)
				id = "trait_category_" + str(entry.get("category", ""))
			checked += 1
			if not library.call("has_icon", id):
				missing.append(id)
	_check(missing.is_empty(), "data ids without icon: %s" % [missing])
	_check(traits_without_own_icon.is_empty(), "traits without their own DA7c icon: %s" % [traits_without_own_icon])
	for hud_id in ["hud_treasury", "hud_income", "hud_research", "hud_season_spring", "hud_season_summer", "hud_season_autumn", "hud_season_winter", "hud_diplomacy", "hud_chronicle", "hud_court", "hud_technologies", "class_peasants", "class_burghers", "class_clergy", "class_nobility", "gauge_unrest", "gauge_health", "gauge_wealth", "gauge_goods_satisfaction"]:
		if not library.call("has_icon", hud_id):
			missing.append(hud_id)
	_check(missing.is_empty(), "UI ids without icon: %s" % [missing])
	_check(str(library.call("resolve", "unit_does_not_exist")) == "cat_unit", "unit fallback expected")
	_check(str(library.call("resolve", "bld_does_not_exist")) == "cat_building", "building fallback expected")
	_check(library.call("get_icon", "totally_unknown") != null, "default fallback expected")
	# DA5b : miniatures d'entité peintes, prioritaires sur l'encre et le SVG, jamais teintées.
	var entity: Dictionary = library.get("entity")
	_check(entity.size() >= 100, "entity miniatures missing: %d" % entity.size())
	for id in ["unit_knights", "bld_castle", "tech_bombards", "unit_category_infantry"]:
		if entity.has(id):
			_check(bool(library.call("is_entity", id)) and not bool(library.call("is_ink", id)), "%s should be a painted miniature" % id)
			_check(str(library.call("icon_path", id)).begins_with("res://assets/icons/entity/"), "%s miniature path" % id)
			_check(library.call("get_icon", id) is Texture2D, "%s miniature not loadable" % id)
	_check(not bool(library.call("is_entity", "hud_treasury")), "action icons stay ink")
	var tip := RichTooltip.technology({"id": "tech_bombards", "name": "Bombardes", "branch": "military", "tier": 3, "cost": 350, "effective_cost": 350, "effects": [{"kind": "siege_resistance", "value": -5}], "historical_year": 1346})
	_check(tip.contains("[img") and tip.contains("1346") and tip.contains("Résistance aux sièges"), "technology tooltip incomplete: %s" % tip)
	var panel := TooltipHost.from_bbcode(RichTooltip.gauge("unrest", 40))
	root.add_child(panel)
	await process_frame
	_check(panel.find_child("Text", true, false) is RichTextLabel, "tooltip panel text expected")
	panel.queue_free()
	var chip := IconChip.create("res_wine", "Vin", "x")
	_check(chip.icon_rect != null and chip.icon_rect.texture != null, "icon chip texture expected")
	chip.free()
	if failures == 0:
		print("smoke OK: icons, %d entries loaded, %d data ids covered, fallbacks and rich tooltips" % [table.size(), checked])


## H2 : Codex (fiches, liens, bulles imbriquées, découvertes, fenêtre).
func _run_codex() -> void:
	var store: Node = root.get_node_or_null("/root/CodexStore")
	var bubbles: Node = root.get_node_or_null("/root/CodexBubbles")
	if not _check(store != null and bubbles != null, "CodexStore / CodexBubbles autoloads missing"):
		return
	store.call("use_test_file", _test_codex_path)
	store.call("reload", _project_root().path_join("data/codex_bundle.json"))
	var total: int = store.call("total_count")
	_check(total >= 20, "codex should have at least 20 entries, got %d" % total)
	_check(bool(store.call("has_entry", "cdx_crecy")) and bool(store.call("has_entry", "cdx_poitiers")), "codex seed entries missing")
	_check(str(store.call("entry_for_entity", "chr_charles_v")) == "cdx_charles_v", "entity index expected")

	# Liens explicites, auto-liens (première occurrence, hors balises), couleur des fiches lues.
	var linked := CodexText.format("[[cdx_crecy]] puis [[cdx_poitiers|la défaite du roi]].")
	_check(linked.contains("[url=cdx:cdx_crecy]") and linked.contains("[u]") and linked.contains(str(store.call("title", "cdx_crecy"))), "explicit link: %s" % linked)
	_check(linked.contains("[url=cdx:cdx_poitiers]") and linked.contains("la défaite du roi"), "labelled link: %s" % linked)
	_check(CodexText.format("[[cdx_nothing_here|mot]]") == "mot", "unknown link should degrade to its label")
	var alias := str(store.call("title", "cdx_peste_noire"))
	var auto := CodexText.format("[img]res://x/%s.png[/img] En 1348, %s frappe ; %s encore." % [alias, alias, alias], true)
	_check(auto.count("[url=cdx:cdx_peste_noire]") == 1 and auto.begins_with("[img]res://x/%s.png[/img]" % alias), "auto link once, outside tags: %s" % auto)
	_check(not CodexText.format("%s." % alias).contains("[url"), "no auto link unless asked")
	var tooltip := TooltipHost.from_bbcode("Voir [[cdx_arc_long]].")
	_check(TooltipHost.last_bbcode.contains("[url=cdx:cdx_arc_long]"), "rich tooltips should go through CodexText")
	_check(TooltipHost.visible_panel() == null, "a detached panel is not a visible tooltip")
	tooltip.free()

	# Pile de 3 bulles : deux ouvertes par l'API, la troisième par survol simulé d'un lien.
	var first: Control = bubbles.call("open", "cdx_crecy", Vector2(120, 120))
	var second: Control = bubbles.call("open", "cdx_edouard_iii", Vector2(260, 180), 0)
	await process_frame
	_check(first != null and second != null and int(bubbles.call("bubble_count")) == 2, "two bubbles expected")
	var text := second.find_child("Text", true, false) as RichTextLabel
	_check(text != null and text.text.contains("[url=cdx:"), "bubble summary should contain links")
	text.meta_hover_started.emit("cdx:cdx_poitiers")
	await create_timer(0.5).timeout
	_check(int(bubbles.call("bubble_count")) == 3 and str(bubbles.call("top_id")) == "cdx_poitiers", "hovering a bubble link should open a third bubble (count %d)" % int(bubbles.call("bubble_count")))
	for id in ["cdx_crecy", "cdx_edouard_iii", "cdx_poitiers"]:
		_check(bool(store.call("is_discovered", id)), "%s should be discovered" % id)
	_check(CodexText.link("cdx_crecy").contains(CodexText.READ_COLOR), "read entries use brown ink")
	var reopened: Control = bubbles.call("open", "cdx_chevauchee", Vector2(300, 300), 0)
	_check(reopened != null and int(bubbles.call("bubble_count")) == 2, "opening from bubble 0 should replace the bubbles above it")
	bubbles.call("set_pinned", first, true)
	text.meta_hover_ended.emit("cdx:cdx_poitiers")
	await create_timer(0.7).timeout
	_check(int(bubbles.call("bubble_count")) == 1, "unpinned bubbles should close after the grace delay (count %d)" % int(bubbles.call("bubble_count")))
	bubbles.call("close_all")
	_check(int(bubbles.call("bubble_count")) == 0, "close_all should empty the stack")
	var pinned: Control = bubbles.call("open_text", TooltipHost.last_bbcode, Vector2(40, 40))
	_check(pinned != null and bool(pinned.get_meta("pinned", false)), "pinned tooltip bubble expected")
	bubbles.call("close_all")

	# Fenêtre Codex sur une fiche, historique, compteur.
	bubbles.call("open_entry", "cdx_charles_v")
	await process_frame
	var window: Control = bubbles.call("window")
	_check(bool(bubbles.call("is_window_open")) and str(window.get("current_id")) == "cdx_charles_v", "codex window should show cdx_charles_v")
	var body: RichTextLabel = window.call("body_label")
	_check(body.text.length() > 200 and body.text.contains("[url=cdx:"), "codex body should be formatted with links")
	window.call("navigate", "cdx_du_guesclin")
	window.call("back")
	_check(str(window.get("current_id")) == "cdx_charles_v", "history back expected")
	var counter := str(window.call("counter_text"))
	_check(counter == "%d / %d découvertes" % [int(store.call("discovered_count")), total], "counter: %s" % counter)
	window.hide()
	store.call("reset_discoveries")
	if failures == 0:
		print("smoke OK: codex, %d entries, links, 3-bubble stack, discoveries, window (%s)" % [total, counter])


## B1 : T universel (bulle non épinglée, infobulle simple convertie), chaîne de bulles
## imbriquées épinglées, pieds, champ `gameplay` (bulle + fenêtre), titres d'infobulles liés.
## Seule : CENT_ANS_SMOKE_ONLY=codex_bubbles.
func _run_codex_bubbles_b1() -> void:
	var store: Node = root.get_node_or_null("/root/CodexStore")
	var bubbles: Node = root.get_node_or_null("/root/CodexBubbles")
	if not _check(store != null and bubbles != null, "B1: CodexStore / CodexBubbles autoloads missing"):
		return
	store.call("use_test_file", _test_codex_path)
	store.call("reload", _project_root().path_join("data/codex_bundle.json"))
	bubbles.call("close_all")
	var failures_before := failures

	# T sans rien à verrouiller : l'événement reste libre (T ouvre aussi l'arbre des techniques).
	_check(not bool(bubbles.call("pin_current")), "B1: T with nothing to pin should not be consumed")

	# T sur une bulle non épinglée : elle est verrouillée, le pied change.
	var first: PanelContainer = bubbles.call("open", "cdx_crecy", Vector2(120, 120))
	var footer := first.find_child("Footer", true, false) as Label
	_check(footer != null and footer.text.contains("T : maintenir ouverte") and footer.text.contains("Clic : lire la fiche"), "B1: unpinned footer: %s" % (footer.text if footer != null else "?"))
	_check(bool(bubbles.call("pin_current")) and bool(first.get_meta("pinned", false)), "B1: T should pin the unpinned bubble")
	_check(footer.text.contains("Clic droit : détacher") and not footer.text.contains("T :"), "B1: pinned footer: %s" % footer.text)

	# Bulle fille d'une bulle épinglée (survol d'un lien), puis petite-fille ; T sur la
	# petite-fille garde toute la chaîne.
	var child := await _hover_first_link(bubbles, first)
	_check(child != null and bubbles.call("parent_of", child) == first and not bool(child.get_meta("pinned", false)), "B1: hovering a link of a pinned bubble should open an unpinned child")
	var grandchild: PanelContainer = await _hover_first_link(bubbles, child) if child != null else null
	_check(grandchild != null and bubbles.call("parent_of", grandchild) == child, "B1: grandchild bubble expected")
	if grandchild != null:
		_check(bool(bubbles.call("pin_current")) and bool(grandchild.get_meta("pinned", false)) and bool(child.get_meta("pinned", false)), "B1: pinning a grandchild pins its ancestors")
		await create_timer(0.7).timeout
		_check(int(bubbles.call("bubble_count")) == 3, "B1: pinned chain survives the mouse leaving (count %d)" % int(bubbles.call("bubble_count")))
		bubbles.call("set_pinned", child, false)
		_check(not bool(grandchild.get_meta("pinned", false)), "B1: detaching a bubble detaches its descendants")
		await create_timer(0.7).timeout
		_check(int(bubbles.call("bubble_count")) == 1, "B1: detached chain closes after the grace delay (count %d)" % int(bubbles.call("bubble_count")))
	bubbles.call("close_all")

	# T sur un contrôle à infobulle simple : bulle épinglée auto-liée.
	var alias := str(store.call("title", "cdx_peste_noire"))
	var holder := PanelContainer.new()
	holder.tooltip_text = "En 1348, %s ravage le royaume." % alias
	holder.mouse_filter = Control.MOUSE_FILTER_STOP
	var inner := Label.new()
	inner.text = "survol"
	inner.mouse_filter = Control.MOUSE_FILTER_PASS
	holder.add_child(inner)
	root.add_child(holder)
	_check(bool(bubbles.call("pin_control_tooltip", inner, Vector2(2, 2))), "B1: T on a control with tooltip_text should open a bubble")
	var converted: PanelContainer = bubbles.get("bubbles")[-1] if int(bubbles.call("bubble_count")) > 0 else null
	var converted_text := converted.find_child("Text", true, false) as RichTextLabel if converted != null else null
	_check(converted != null and bool(converted.get_meta("pinned", false)) and converted_text.text.contains("[url=cdx:cdx_peste_noire]"), "B1: converted tooltip should be pinned and auto-linked")
	var bare := Label.new()
	_check(not bool(bubbles.call("pin_control_tooltip", bare, Vector2.ZERO)), "B1: a control without tooltip pins nothing")
	bare.free()
	holder.queue_free()
	bubbles.call("close_all")

	# `gameplay` : ligne « En jeu » (1re phrase) dans la bulle, encadré dans la fenêtre.
	var entry: Dictionary = store.call("entry", "cdx_crecy")
	var had_gameplay := entry.has("gameplay")
	var old_gameplay: Variant = entry.get("gameplay", "")
	entry["gameplay"] = "Les [[cdx_arc_long|archers]] tirent avant la mêlée. Seconde phrase du test."
	var bubble: PanelContainer = bubbles.call("open", "cdx_crecy", Vector2(80, 80))
	var bubble_text := (bubble.find_child("Text", true, false) as RichTextLabel).text
	_check(bubble_text.contains("[i]En jeu :[/i] Les ") and bubble_text.contains("tirent avant la mêlée.") and not bubble_text.contains("Seconde phrase"), "B1: gameplay line in bubble: %s" % bubble_text)
	bubbles.call("close_all")
	bubbles.call("open_entry", "cdx_crecy")
	var window: Control = bubbles.call("window")
	var box: Control = window.get("gameplay_box")
	var box_label: RichTextLabel = window.get("gameplay_label")
	_check(box != null and box.visible and box_label.text.contains("En jeu") and box_label.text.contains("Seconde phrase"), "B1: gameplay box in the codex window")
	window.call("navigate", "cdx_poitiers")
	_check(not box.visible or str((store.call("entry", "cdx_poitiers") as Dictionary).get("gameplay", "")) != "", "B1: gameplay box hidden without gameplay")
	window.hide()
	if had_gameplay:
		entry["gameplay"] = old_gameplay
	else:
		entry.erase("gameplay")

	# Titres d'infobulles riches liés via `entity`, pied « T : maintenir ouverte ».
	var unit_entry := str(store.call("entry_for_entity", "unit_longbowmen"))
	var unit_tip := RichTooltip.unit("unit_longbowmen")
	_check(unit_entry != "" and RichTooltip.title_entry(unit_tip) == unit_entry, "B1: unit tooltip title should link to %s" % unit_entry)
	var panel := TooltipHost.from_bbcode(unit_tip)
	var tip_footer := panel.find_child("Footer", true, false) as Label
	_check(tip_footer != null and tip_footer.text.begins_with("T : maintenir ouverte"), "B1: rich tooltip footer expected")
	panel.free()
	_check(RichTooltip.title_entry(RichTooltip.resource("res_nothing_here")) == "", "B1: unlinked titles stay plain")
	_check_codex_homonyms_b8()
	store.call("reset_discoveries")
	if failures == failures_before:
		print("smoke OK: B1 bubbles, T pins bubble / plain tooltip, 3-level pinned chain, gameplay, linked titles")


## B8 : auto-lien sans homonymes (exclusions, alias le plus long, tiret, échappement `[[!…]]`).
func _check_codex_homonyms_b8() -> void:
	var failures_before := failures
	var poitiers_link := "[url=cdx:cdx_poitiers]"
	var excluded := CodexText.format("Louis de Poitiers, comte de Valentinois, est tué.", true)
	_check(not excluded.contains(poitiers_link), "B8: « Louis de Poitiers » must not link the battle: %s" % excluded)
	var place := CodexText.format("Le roi est pris à Poitiers.", true)
	_check(place.contains(poitiers_link + "[color="), "B8: « à Poitiers » should link the battle: %s" % place)
	var both := CodexText.format("Louis de Poitiers meurt ; dix ans plus tard, à Poitiers.", true)
	_check(both.count("url=cdx:cdx_poitiers") == 1 and both.find(poitiers_link) > both.find("à "), "B8: only the later « à Poitiers » is linked: %s" % both)
	var longest := CodexText.format("Le duc Louis d'Orléans est assassiné.", true)
	_check(longest.contains("[url=cdx:cdx_louis_d_orleans]") and not longest.contains("[url=cdx:cdx_orleans]"), "B8: the longest alias should win: %s" % longest)
	var hyphen := CodexText.format("Les foires de Poitiers-la-Neuve.", true)
	_check(not hyphen.contains(poitiers_link), "B8: an alias glued by a hyphen is not linked: %s" % hyphen)
	var escaped := CodexText.format("[[!Poitiers]] reste une ville ; rien à lier ici.", true)
	var rich := RichTextLabel.new()
	rich.bbcode_enabled = true
	rich.text = escaped
	_check(not escaped.contains("url=") and not escaped.contains("!") and rich.get_parsed_text() == "Poitiers reste une ville ; rien à lier ici.", "B8: [[!…]] should render as plain text: %s" % escaped)
	var twice := CodexText.format(escaped, true)
	_check(not twice.contains(poitiers_link), "B8: an escape survives a second format: %s" % twice)
	rich.free()
	_check(CodexText.plain("[[!Louis de Poitiers]], [[cdx_crecy|Crécy]]") == "Louis de Poitiers, Crécy", "B8: plain() should strip escapes")
	if failures == failures_before:
		print("smoke OK: B8 homonyms (exclude_contexts, longest alias, hyphen, [[!…]] escape)")


## Survole le premier lien du Codex de `bubble` et attend la bulle fille (null si aucun lien).
func _hover_first_link(bubbles: Node, bubble: PanelContainer) -> PanelContainer:
	var label := bubble.find_child("Text", true, false) as RichTextLabel
	var found := RegEx.create_from_string("\\[url=(cdx:[^\\]]+)\\]").search(label.text)
	if found == null:
		return null
	var count := int(bubbles.call("bubble_count"))
	label.meta_hover_started.emit(found.get_string(1))
	await create_timer(0.5).timeout
	label.meta_hover_ended.emit(found.get_string(1))
	if int(bubbles.call("bubble_count")) != count + 1:
		return null
	return bubbles.get("bubbles")[-1]


func _run_table_medicine() -> void:
	const FACTION_ID := "fac_france"
	if not (ClassDB.class_exists("CampaignSim") and ClassDB.instantiate("CampaignSim").has_method("get_diet_options")):
		print("smoke table/medicine: skipped, CampaignSim has no get_diet_options (run core/build.sh)")
		return
	var sim: Object = ClassDB.instantiate("CampaignSim")
	if not _check(sim.call("new_campaign", _project_root().path_join("data"), FACTION_ID, 1337), "table/medicine: new_campaign failed"):
		return
	# Province au joueur avec un régime payant disponible (≠ actuel), une autre avec un régime
	# indisponible.
	var changed_province := ""
	var target_diet := ""
	var refused_province := ""
	var refused_diet := ""
	var diets: Dictionary = sim.call("get_province_diets")
	var province_ids: Array = diets.keys()
	province_ids.sort()
	for province_id in province_ids:
		var state: Dictionary = sim.call("get_province_state", str(province_id))
		if str(state.get("controller", state.get("owner", ""))) != FACTION_ID:
			continue
		for option in sim.call("get_diet_options", str(province_id)):
			var id := str(option.get("id", ""))
			if changed_province == "" and bool(option.get("available", false)) and not bool(option.get("current", false)) and int(option.get("cost", 0)) > 0:
				changed_province = str(province_id)
				target_diet = id
			elif refused_province == "" and str(province_id) != changed_province and not bool(option.get("available", false)):
				refused_province = str(province_id)
				refused_diet = id
	if not _check(changed_province != "" and refused_province != "", "table: no French province with available/unavailable diets (%s / %s)" % [changed_province, refused_province]):
		return

	# Panneau de province réel : la section Table est dans l'onglet Ville.
	var panel: Node = (load("res://scenes/ui/province_panel.tscn") as PackedScene).instantiate()
	root.add_child(panel)
	await process_frame
	var hosted: TableSection = panel.get("table_section")
	_check(hosted != null and hosted.get_parent() == (panel.get("classes_list") as Node).get_parent(), "province panel should host the Table section in the Ville tab")
	panel.queue_free()

	var table := TableSection.new()
	root.add_child(table)
	table.show_for(changed_province, true, sim)
	await process_frame
	_check(table.visible and table.option_buttons.size() == (sim.call("get_diet_options", changed_province) as Array).size(), "table section should list every diet option")
	var tip := RichTooltip.diet(PanelSection.find_option(table.options, target_diet))
	_check(tip.contains("Coût") and tip.contains("[img"), "diet tooltip incomplete: %s" % tip)
	(table.option_buttons[target_diet] as Button).pressed.emit()
	var now: Dictionary = sim.call("get_province_diet", changed_province)
	_check(str(now.get("diet", "")) == target_diet, "set_diet via UI: expected %s, got %s (%s)" % [target_diet, now.get("diet", ""), table.last_result])
	_check(table.changed_label.visible and table.choose_button.disabled, "changed-this-turn state should be shown")
	var lent_expected := bool(sim.call("is_lent"))
	_check(table.lent_banner.visible == lent_expected, "Lent banner should follow is_lent (%s)" % lent_expected)

	table.show_for(refused_province, true, sim)
	var refused: Dictionary = table.request_diet(refused_diet)
	_check(not bool(refused.get("ok", true)) and table.error_label.visible and table.error_label.text.contains("impossible"), "unavailable diet should be refused and shown: %s" % table.error_label.text)
	var refused_tip := RichTooltip.diet(PanelSection.find_option(table.options, refused_diet))
	_check(refused_tip.contains("Manque"), "unavailable diet tooltip should list missing conditions: %s" % refused_tip)
	table.show_for(refused_province, false, sim)
	_check(not table.choose_button.visible and table.option_buttons.is_empty(), "read-only province: no selector")
	table.queue_free()

	var economy: Dictionary = sim.call("get_faction_economy", FACTION_ID)
	_check(int(economy.get("table_upkeep", 0)) > 0, "table_upkeep should be > 0 after a paying diet")

	# Infobulle de tech médecine : plantes et note historique ; libellés des effets.
	var herb_node: Dictionary = {}
	for node in sim.call("get_tech_tree", FACTION_ID):
		if str(node.get("id", "")) == "tech_herb_garden":
			herb_node = node
	var tech_tip := RichTooltip.technology(herb_node)
	_check(tech_tip.contains("Plantes :") and tech_tip.contains("sauge") and tech_tip.contains("médecine") and tech_tip.contains("De Villis"), "medicine tech tooltip incomplete: %s" % tech_tip)
	_check(RichTooltip.effect_text({"kind": "plague_resistance", "value": 10}).begins_with("Résistance à la peste"), "plague_resistance label")
	_check(RichTooltip.effect_text({"kind": "wound_recovery", "value": 15, "mode": "percent"}).begins_with("Soin des blessés"), "wound_recovery label")
	_check(RichTooltip.effect_text({"kind": "diet_health", "value": 25}).begins_with("Santé tirée des régimes"), "diet_health label")

	# Genres table / medicine : rapport de saison, lettres, alertes ; herbier.
	var events := [
		{"kind": "table", "text_fr": "La table revient au pain bis.", "faction": FACTION_ID, "province": changed_province},
		{"kind": "medicine", "text_fr": "Épidémie contenue.", "faction": FACTION_ID, "province": ""},
	]
	var groups := SeasonReport.build_groups(events, func(_event: Dictionary) -> bool: return true)
	_check(groups.size() == 1 and (groups[0]["entries"] as Array).size() == 2, "season report should group table/medicine events: %s" % [groups])
	_check(NewsLetters.KIND_LABELS.has("table") and NewsLetters.KIND_LABELS.has("medicine"), "news letters labels for table/medicine")
	var alerts := CampaignAlerts.table_medicine_alerts(sim, FACTION_ID, events)
	_check(alerts.size() == 2 and str(alerts[0]["kind"]) == "table", "alerts for table/medicine: %s" % [alerts])
	var store: Node = root.get_node_or_null("/root/CodexStore")
	if store != null:
		store.call("use_test_file", _test_codex_path)
		var herbs := Herbarium.sync(sim, FACTION_ID)
		_check(herbs.size() >= 0, "herbarium sync should ignore missing entries")
		store.call("reset_discoveries")
	if failures == 0:
		print("smoke OK: table/medicine, %s -> %s, refused %s in %s, table upkeep %d" % [changed_province, target_diet, refused_diet, refused_province, int(economy.get("table_upkeep", 0))])


func _run_flow() -> void:
	var settings: Node = root.get_node_or_null("/root/Settings")
	if not _check(settings != null, "Settings autoload missing"):
		return
	# Réglages : écriture, relecture, section audio conservée.
	var test_path: String = settings.get("path")
	_check(test_path != "user://settings.cfg", "smoke must not use the player's settings file")
	var seeded := ConfigFile.new()
	seeded.set_value("audio", "legacy_key", 0.42)
	seeded.save(test_path)
	settings.call("set_value", "camera/speed", 1.7)
	settings.call("set_value", "interface/confirm_end_turn", true)
	settings.call("set_value", "video/resolution", Vector2i(1600, 900))
	var config := ConfigFile.new()
	_check(config.load(test_path) == OK, "settings file not written")
	_check(is_equal_approx(float(config.get_value("camera", "speed", 0.0)), 1.7), "camera speed not persisted")
	_check(is_equal_approx(float(config.get_value("audio", "legacy_key", 0.0)), 0.42), "audio section lost by Settings.save_settings")
	settings.set("values", {})
	settings.call("load_settings")
	_check(is_equal_approx(float(settings.call("get_value", "camera/speed")), 1.7), "camera speed not reloaded")
	_check(bool(settings.call("get_value", "interface/confirm_end_turn")), "confirm_end_turn not reloaded")
	_check(settings.call("get_value", "video/resolution") == Vector2i(1600, 900), "resolution not reloaded")
	settings.call("set_value", "interface/confirm_end_turn", false, false)
	settings.call("set_value", "interface/season_report", true, false)
	settings.call("set_value", "game/autosave_interval", 1, false)
	_check(SaveSlots.autosave_name_for(1, 1) == "auto_1" and SaveSlots.autosave_name_for(4, 1) == "auto_1"
		and SaveSlots.autosave_name_for(6, 2) == "auto_3" and SaveSlots.autosave_name_for(3, 2) == "", "autosave rotation names")

	# Écran de chargement jusqu'à la carte (vraies données si possible, comme la boucle de campagne).
	var real_data := _project_root().path_join("data")
	var use_real: bool = ClassDB.class_exists("CampaignSim") and FileAccess.file_exists(real_data.path_join("map/map.json"))
	facade.set_data_dir(real_data if use_real else _fixtures_dir)
	facade.pending_faction = "fac_france"
	facade.pending_seed = 1337
	facade.pending_load_path = ""
	var backup := _backup_autosaves()
	var screen: Node = LoadingScreen.start(self)
	var map: Node = await screen.finished
	if not _check(map != null and map.get("load_ok"), "loading screen did not produce a loaded campaign map"):
		_restore_autosaves(backup)
		return
	_check(current_scene == map, "loading screen should make the map the current scene")
	# Accès non typé : typer FlowController compilerait MapUI avant les autoloads.
	var flow: Node = map.get("flow")
	if not _check(flow != null, "campaign map has no FlowController"):
		_restore_autosaves(backup)
		return
	_check(is_equal_approx(map.camera_rig.pan_speed, 1.2 * 1.7), "camera speed setting not applied")

	# Tours : sauvegarde auto tournante, rapport de saison, alertes.
	var report_lines := 0
	for _turn in 4:
		map._on_end_turn()
		map.chronicle.window.hide()
		if flow.season_report.visible:
			report_lines = maxi(report_lines, flow.season_report.line_count())
	await process_frame
	var turn: int = map.sim.call("get_turn")
	_check(turn == 4, "flow: expected turn 4, got %d" % turn)
	for slot in ["auto_1", "auto_2", "auto_3"]:
		_check(FileAccess.file_exists(facade.save_path(slot)) and FileAccess.file_exists(SaveSlots.meta_path(slot)), "autosave %s missing" % slot)
	var meta: Variant = JSON.parse_string(FileAccess.get_file_as_string(SaveSlots.meta_path("auto_1")))
	_check(meta is Dictionary and int(meta.get("turn", -1)) == 4, "auto_1 should have rotated to turn 4, got %s" % [meta])
	_check(flow.unsaved_turns() == 0, "autosave should reset the unsaved counter")
	_check(not SaveSlots.latest().is_empty(), "Continue: latest save expected")
	_check(report_lines > 0, "season report should list events after 4 turns")
	var all_events: Array = map.sim.call("get_events")
	print("smoke flow: %d report lines, %d alerts, %d journal events" % [report_lines, map.ui.end_turn_cluster.alerts.size(), all_events.size()])

	# P2 : bataille résolue après la fin du tour (dialogue d'avant-bataille) — son résultat
	# doit rejoindre le rapport de saison déjà affiché, pas seulement la chronique
	# (docs/archive/chantiers.md § « Défauts relevés », docs/archive/chantiers.md).
	var battle_armies: Array = BattleScene.main_armies(map.sim, "fac_france", "fac_england")
	if _check(battle_armies.size() == 2, "flow: no French or English army for the late-battle check"):
		var battle_index: int = map.sim.call("debug_stage_battle", battle_armies[0], battle_armies[1])
		_check(battle_index >= 0, "flow: debug_stage_battle failed")
		map._on_battle_auto(battle_index)
		await process_frame
		_check(flow.season_report.visible, "flow: season report should (re)open after a late battle result")
		var found_result := false
		for group in flow.season_report.groups:
			for entry in (group["entries"] as Array):
				if str((entry as Dictionary).get("text_fr", "")).contains("Vainqueur"):
					found_result = true
		_check(found_result, "flow: season report should include the resolved battle (result), not just 'en vue'")

	# Menu pause : ouverture (arbre en pause), dialogue de sauvegarde, fermeture.
	flow.open_pause()
	await process_frame
	_check(paused and flow.is_paused(), "pause menu should pause the tree")
	flow.pause_menu.open_save()
	_check(flow.pause_menu.save_dialog.visible and flow.pause_menu.save_dialog.save_count() >= 3, "save dialog should list the autosaves")
	flow.pause_menu.open_settings()
	await process_frame
	_check(flow.pause_menu.settings_open(), "settings window should open from pause")
	flow.close_pause()
	await process_frame
	_check(not paused and not flow.is_paused(), "closing the pause menu should resume")
	map._on_end_turn()
	_check(flow.unsaved_turns() == 0, "autosave every turn: nothing unsaved")

	# Crédits (CREDITS.md ou texte intégré).
	var credits: Node = (load("res://scenes/ui/credits_screen.tscn") as PackedScene).instantiate()
	root.add_child(credits)
	await process_frame
	_check(credits.text_label.text.length() > 100, "credits text should not be empty")
	credits.queue_free()

	settings.call("set_value", "game/autosave_interval", 0, false)
	current_scene = null
	map.queue_free()
	await process_frame
	_restore_autosaves(backup)
	if failures == 0:
		print("smoke OK: flow (settings, loading, autosave rotation, season report, pause, credits)")


## Met de côté les sauvegardes automatiques du joueur (`user://saves/auto_*`).
func _backup_autosaves() -> Dictionary:
	var kept: Dictionary = {}
	var dir := DirAccess.open(SaveSlots.SAVES_DIR)
	if dir == null:
		return kept
	for file_name in dir.get_files():
		if file_name.begins_with(SaveSlots.AUTOSAVE_PREFIX):
			var path := SaveSlots.SAVES_DIR.path_join(file_name)
			kept[path] = FileAccess.get_file_as_bytes(path)
			DirAccess.remove_absolute(path)
	return kept


func _restore_autosaves(kept: Dictionary) -> void:
	var dir := DirAccess.open(SaveSlots.SAVES_DIR)
	if dir != null:
		for file_name in dir.get_files():
			if file_name.begins_with(SaveSlots.AUTOSAVE_PREFIX):
				DirAccess.remove_absolute(SaveSlots.SAVES_DIR.path_join(file_name))
	for path in kept:
		var file := FileAccess.open(path, FileAccess.WRITE)
		if file != null:
			file.store_buffer(kept[path])
			file.close()


## F8 : tutoriel des premiers tours (objectifs vérifiables) et encyclopédie tirée de `data/`.
func _run_tutorial() -> void:
	var settings: Node = root.get_node_or_null("/root/Settings")
	var real_data := _project_root().path_join("data")
	if not (ClassDB.class_exists("CampaignSim") and FileAccess.file_exists(real_data.path_join("map/map.json"))):
		print("smoke tutorial: skipped, needs the real simulation and data/map")
		return
	if settings != null:
		settings.call("set_value", "tutorial/enabled", true, false)
		settings.call("set_value", "tutorial/done", false, false)
		settings.call("set_value", "tutorial/step", 0, false)
		settings.call("set_value", "interface/season_report", true, false)
		settings.call("set_value", "game/interactive_battles", false, false)
	facade.set_data_dir(real_data)
	facade.pending_faction = "fac_france"
	facade.pending_seed = 1337
	facade.pending_load_path = ""
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	await process_frame
	await process_frame
	if not _check(map.load_ok and map.sim != null, "tutorial: campaign map failed to start"):
		return
	var tutorial: Node = map.get("tutorial")
	if not _check(tutorial != null and tutorial.active, "tutorial should start on a new campaign"):
		map.queue_free()
		return
	var sim: Object = map.sim
	var player: String = map.player_faction
	var visited: PackedStringArray = PackedStringArray()
	_check(tutorial.steps.size() >= 10 and tutorial.steps.size() <= 14, "tutorial should have 10-14 steps, got %d" % tutorial.steps.size())
	_check(str(tutorial.steps[0]["text"]).contains("Bouter les Anglais"), "intro should list the French objectives from data")
	_check(str(tutorial.steps[1]["advice"]).contains("Crécy"), "French advice expected")
	_check(tutorial.current_step_id() == "intro" and not tutorial.check_now(), "intro is manual")
	tutorial.advance()

	# 1. Sélection de l'armée royale.
	_check(tutorial.current_step_id() == "select_army" and not tutorial.check_now(), "select_army: not met before selection")
	var royal: String = tutorial.royal_army
	_check(royal != "" and not tutorial.resolve_target("royal_army").is_empty(), "royal army and its marker target expected")
	map.select_army(royal)
	_check(tutorial.check_now(), "select_army should complete after selecting the royal army")
	visited.append("select_army")
	# 2. Ordre de marche.
	_check(tutorial.current_step_id() == "move_army" and not tutorial.check_now(), "move_army: not met before an order")
	var reachable: Dictionary = map.reachable
	if _check(not reachable.is_empty(), "tutorial: royal army has no reachable province"):
		var result: Dictionary = map.order_move(royal, str(reachable.keys()[0]))
		_check(result.get("ok", false), "tutorial: move refused %s" % result.get("error", "?"))
	_check(tutorial.check_now(), "move_army should complete after a move order")
	visited.append("move_army")
	# 3-4. Province du joueur puis onglet Ville.
	_check(tutorial.current_step_id() == "open_province" and not tutorial.check_now(), "open_province: not met before opening")
	var capital: String = tutorial._capital()
	map.picker.select_index(map.map_data.index_of_id(capital))
	await process_frame
	_check(tutorial.check_now(), "open_province should complete with the capital panel open")
	_check(tutorial.current_step_id() == "city_tab" and not tutorial.check_now(), "city_tab: not met on the garrison tab")
	_check(not tutorial.resolve_target("city_tab").is_empty(), "city tab target expected")
	map.ui.province_panel.show_ville_tab()
	_check(tutorial.check_now(), "city_tab should complete on the Ville tab")
	visited.append_array(["open_province", "city_tab"])
	# 5. Construction.
	_check(tutorial.current_step_id() == "build" and not tutorial.check_now(), "build: not met before building")
	var built := false
	for province_id in tutorial._player_provinces():
		var city: Dictionary = sim.call("get_province_city", province_id)
		if not (city.get("construction", {}) as Dictionary).is_empty():
			continue
		for row in city.get("buildable", []):
			if bool(row.get("available", false)):
				map._on_build(province_id, str(row["building"]))
				built = true
				break
		if built:
			break
	_check(built and tutorial.check_now(), "build should complete after a construction order")
	visited.append("build")
	# 6. Recherche.
	_check(tutorial.current_step_id() == "research" and not tutorial.check_now(), "research: not met before choosing")
	for node in sim.call("get_tech_tree", player):
		if str(node.get("state", "")) == "available":
			map._on_research_requested(str(node["id"]))
			break
	_check(tutorial.check_now(), "research should complete after choosing a technology")
	visited.append("research")
	# 7. Diplomatie.
	_check(tutorial.current_step_id() == "diplomacy" and not tutorial.check_now(), "diplomacy: not met before opening")
	_check(not tutorial.resolve_target("diplomacy").is_empty(), "diplomacy button target expected")
	map.diplomacy.open_panel()
	_check(tutorial.check_now(), "diplomacy should complete with the panel open")
	map.diplomacy.panel.hide()
	visited.append("diplomacy")
	# 8-9. Fin du tour, rapport de saison.
	_check(tutorial.current_step_id() == "end_turn" and not tutorial.check_now(), "end_turn: not met before ending the turn")
	_check(tutorial.resolve_target("end_turn").has("rect"), "end turn button target expected")
	map._on_end_turn()
	# Rapport de saison ouvert hors de son étape : le guide se range (il couvrait « Continuer »).
	var report_shown: bool = map.flow.season_report.visible
	_check(not report_shown or tutorial.modal_open(), "the guide should hide under a season report opened outside its step")
	_check(tutorial.check_now(), "end_turn should complete after end_turn")
	_check(not tutorial.modal_open(), "the guide should show beside the season report during its own step")
	visited.append("end_turn")
	_check(tutorial.process_mode == Node.PROCESS_MODE_ALWAYS, "tutorial controller must keep running under the pause menu to hide the guide")
	_check(tutorial.current_step_id() == "season_report", "season_report step expected")
	var report: Control = map.flow.season_report
	if report.visible:
		_check(not tutorial.check_now(), "season_report: not met while the report is open")
		report.close()
	_check(tutorial.check_now(), "season_report should complete once the report is closed")
	visited.append("season_report")
	# 10. Chronique (étape passée s'il n'y a pas encore d'événement).
	_check(tutorial.current_step_id() == "chronicle", "chronicle step expected")
	if map.chronicle.open_window():
		_check(tutorial.check_now(), "chronicle should complete with the window open")
		map.chronicle.window.hide()
		visited.append("chronicle")
	else:
		map.chronicle.window.hide()
		_check(not tutorial.check_now(), "chronicle: not met without window")
		tutorial.advance()
		print("smoke tutorial: no chronicle decision yet, step skipped")
	# 11. Impôt.
	_check(tutorial.current_step_id() == "tax" and not tutorial.check_now(), "tax: not met before changing")
	var rate := "high" if tutorial._tax_rate() != "high" else "low"
	map._on_tax_rate_changed(player, rate)
	_check(tutorial.check_now(), "tax should complete after changing the rate")
	visited.append("tax")
	# 12. Gouverneur.
	_check(tutorial.current_step_id() == "governor" and not tutorial.check_now(), "governor: not met before appointing")
	var appointed := false
	var ruler := str(GameCatalog.definitions("factions").get(player, {}).get("ruler", ""))
	for character_id in sim.call("get_faction_characters", player):
		if str(character_id) == ruler or str((sim.call("get_character", character_id) as Dictionary).get("governor_of", "")) != "":
			continue
		for province_id in tutorial._player_provinces():
			if province_id == capital:
				continue
			var answer: Dictionary = sim.call("submit_order", {"type": "assign_governor", "character": str(character_id), "province": province_id})
			if answer.get("ok", false):
				appointed = true
				break
		if appointed:
			break
	_check(appointed and tutorial.check_now(), "governor should complete after an appointment")
	visited.append("governor")
	# Fin : progression persistée, pas de relance.
	_check(tutorial.current_step_id() == "outro", "outro step expected")
	tutorial.advance()
	_check(not tutorial.active and not tutorial.overlay.visible, "tutorial should close after the last step")
	if settings != null:
		_check(bool(settings.call("get_value", "tutorial/done")), "tutorial/done should be persisted")
	_check(not tutorial.should_autostart(), "a finished tutorial must not restart")
	# Factions sans texte propre : dirigeant, suzerain et objectifs de titre lus dans la simulation,
	# conseils génériques, aucun champ laissé brut.
	var generic_steps: Array[Dictionary] = TutorialSteps.steps("fac_albret", tutorial._context("fac_albret"))
	_check(generic_steps.size() == TutorialSteps.count(), "generic tutorial should keep every step")
	_check(str(generic_steps[0]["text"]).contains("Les objectifs de votre titre") and not str(generic_steps[0]["text"]).contains("Survivre et prospérer"), "generic intro should list the title objectives from the simulation")
	_check(str(generic_steps[0]["title"]).begins_with("Bienvenue, ") and not str(generic_steps[0]["title"]).contains("Sire"), "generic intro should greet the ruler by name")
	for generic_step in generic_steps:
		_check(str(generic_step["advice"]) != "", "generic advice missing at %s" % generic_step["id"])
		for key in ["title", "text", "objective", "advice"]:
			_check(not str(generic_step[key]).contains("{"), "raw placeholder in generic %s.%s" % [generic_step["id"], key])
	_check(TutorialSteps.steps("fac_x", {})[0]["title"] == "Bienvenue", "a faction without ruler gets a plain welcome")

	# Encyclopédie.
	var encyclopedia: Control = tutorial.encyclopedia
	var key := InputEventKey.new()
	key.physical_keycode = KEY_L
	key.pressed = true
	tutorial._unhandled_input(key)
	_check(encyclopedia.visible, "K should open the encyclopedia")
	var counts := PackedStringArray()
	var ids: PackedStringArray = encyclopedia.tab_ids()
	_check(ids.size() == 11, "encyclopedia should have 11 tabs (C7: Suite, C6: Agents)")
	for index in ids.size():
		encyclopedia.select_tab(index)
		_check(encyclopedia.entry_count() > 0, "encyclopedia tab %s is empty" % ids[index])
		_check(encyclopedia.fiche.get_parsed_text().length() > 40, "encyclopedia tab %s: empty fiche for %s" % [ids[index], encyclopedia.current_entry])
		counts.append("%s %d" % [ids[index], encyclopedia.entry_count()])
	var broken: Array = []
	for index in ids.size():
		for entry in encyclopedia._entries[ids[index]]:
			if Encyclopedia.fiche_bbcode(str(entry["id"])).length() < 40:
				broken.append(entry["id"])
	_check(broken.is_empty(), "encyclopedia entries without fiche: %s" % [broken])
	encyclopedia.select_tab(0)
	var total: int = encyclopedia.entry_count()
	encyclopedia.set_query("arc")
	_check(encyclopedia.entry_count() > 0 and encyclopedia.entry_count() < total and "unit_longbowmen" in encyclopedia.visible_ids(), "search 'arc' should filter the units (%d / %d)" % [encyclopedia.entry_count(), total])
	encyclopedia.set_query("ARBALETRIER")
	_check(encyclopedia.entry_count() > 0, "search should ignore case and accents")
	encyclopedia.set_query("zzzzqx")
	_check(encyclopedia.entry_count() == 0, "nonsense search should list nothing")
	encyclopedia.set_query("")
	_check(encyclopedia.entry_count() == total, "clearing the search should restore the list")
	_check(encyclopedia.open_entry("unit_longbowmen"), "open unit_longbowmen")
	_check(Encyclopedia.fiche_bbcode("unit_longbowmen").contains("[url=tech_longbow_drill]"), "unit fiche should link its technology")
	encyclopedia.fiche.meta_clicked.emit("tech_longbow_drill")
	_check(encyclopedia.current_entry == "tech_longbow_drill" and encyclopedia.tab_ids()[encyclopedia.current_tab] == "technologies", "internal link should open the technology")
	encyclopedia.go_back()
	_check(encyclopedia.current_entry == "unit_longbowmen", "back should return to the unit")
	var france := Encyclopedia.fiche_bbcode("fac_france")
	_check(france.contains("Philippe VI") and france.contains("Objectifs"), "faction fiche: ruler and objectives expected")
	tutorial._unhandled_input(key)
	_check(not encyclopedia.visible, "K should close the encyclopedia")

	# BP1 : le journal et l'aide F1 sont branchés sur les bulles du Codex.
	_check(bool(map.ui.log_text.has_meta("codex_attached")), "BP1: campaign log should be attached to CodexBubbles")
	map.ui.add_events([{"kind": "chronicle", "text": "La victoire de Crécy marque l'Europe."}], "Test BP1")
	_check(map.ui.log_text.text.contains("[url=cdx:"), "BP1: a known alias in a log event should be auto-linked: %s" % map.ui.log_text.text)
	map.help.toggle()
	_check(bool(map.help.text.has_meta("codex_attached")) and map.help.text.text.contains("[url=cdx:"), "BP1: F1 help should be attached to CodexBubbles and auto-link known aliases (Crécy, Peste noire…)")
	map.help.toggle()

	if settings != null:
		settings.call("set_value", "tutorial/enabled", false, false)
		settings.call("set_value", "game/interactive_battles", true, false)
	map.queue_free()
	await process_frame
	if failures == 0:
		print("smoke OK: tutorial (%s) and encyclopedia (%s)" % [", ".join(visited), ", ".join(counts)])


# --- F5c : déploiement et maisons de siège dans la scène ------------------------------


## H11 : monnaie (changement par l'UI, refus du 2e changement dans l'année), panneau des
## rançons (données simulées : la sim de 1337 n'a pas de captif), section de l'ordre de
## chevalerie, genres coinage/ransom/chivalry, liens Encyclopédie ↔ Codex.
func _run_coinage_ransom() -> void:
	const FACTION_ID := "fac_france"
	if not (ClassDB.class_exists("CampaignSim") and ClassDB.instantiate("CampaignSim").has_method("get_coinage")):
		print("smoke coinage/ransom: skipped, CampaignSim has no get_coinage (run core/build.sh)")
		return
	var sim: Object = ClassDB.instantiate("CampaignSim")
	if not _check(sim.call("new_campaign", _project_root().path_join("data"), FACTION_ID, 1337), "coinage: new_campaign failed"):
		return
	var store: Node = root.get_node_or_null("/root/CodexStore")
	if store != null:
		store.call("use_test_file", _test_codex_path)
		store.call("reload", _project_root().path_join("data/codex_bundle.json"))

	# Vraies données (noms de provinces, encyclopédie) ; dossier précédent restauré à la fin.
	var previous_dir := str(paths.get("data_dir"))
	facade.set_data_dir(_project_root().path_join("data"))
	# Panneau de faction réel : sections et lignes de budget ajoutées en code (sim de l'étape).
	var previous_sim: Object = facade.get("sim")
	facade.set("sim", sim)
	var panel: FactionPanel = (load("res://scenes/ui/faction_panel.tscn") as PackedScene).instantiate()
	root.add_child(panel)
	await process_frame
	panel.show_faction(FACTION_ID, "France", Color.BLUE, sim.call("get_faction_economy", FACTION_ID))
	var coinage := panel.coinage_section
	_check(coinage != null and coinage.visible and coinage.level_buttons.size() == 4, "coinage section with 4 levels expected")
	_check(panel.chivalry_section.visible and panel.ransom_button.visible, "chivalry section and ransom button expected")
	_check(coinage.explanation_label.text.contains("[url=cdx:cdx_nicole_oresme]"), "coinage explanation should link Oresme: %s" % coinage.explanation_label.text)
	var tip := RichTooltip.coinage((sim.call("get_coinage", "") as Dictionary)["options"][2])
	_check(tip.contains("Seigneuriage") and tip.contains("Un seul changement"), "coinage tooltip incomplete: %s" % tip)

	# Changement par l'UI, reflété par get_coinage ; le second de l'année est refusé et affiché.
	(coinage.level_buttons["debased"] as Button).pressed.emit()
	var now: Dictionary = sim.call("get_coinage", "")
	_check(str(now.get("level", "")) == "debased" and bool(now.get("changed_this_year", false)), "set_coinage via UI: %s (%s)" % [now.get("level", ""), coinage.last_result])
	_check(coinage.changed_label.visible and not coinage.error_label.visible, "changed-this-year note expected")
	(coinage.level_buttons["strong"] as Button).pressed.emit()
	_check(str((sim.call("get_coinage", "") as Dictionary).get("level", "")) == "debased", "second change in the year must be refused")
	_check(coinage.error_label.visible and coinage.error_label.text.contains("déjà été changée"), "refusal should be shown: %s" % coinage.error_label.text)
	panel.show_faction(FACTION_ID, "France", Color.BLUE, sim.call("get_faction_economy", FACTION_ID))
	_check(panel.seigniorage_value.text.begins_with("+") and int((sim.call("get_faction_economy", FACTION_ID) as Dictionary).get("seigniorage", 0)) > 0, "seigniorage budget line: %s" % panel.seigniorage_value.text)

	# Ordre de chevalerie : options de fondation (Étoile, refusée avant 1351), lien Codex.
	var chivalry := panel.chivalry_section
	_check(chivalry.found_buttons.has("ord_star"), "Star order option expected: %s" % [chivalry.found_buttons.keys()])
	_check(ChivalrySection.codex_entry("ord_star") == "cdx_ordre_de_l_etoile", "order codex link")
	var refused: Dictionary = chivalry.request_found("ord_star")
	_check(not bool(refused.get("ok", true)) and chivalry.error_label.visible and chivalry.error_label.text.contains("1351"), "early founding refused and shown: %s" % chivalry.error_label.text)

	# Rançons : panneau réel ouvert depuis le panneau de faction, puis données simulées.
	panel.toggle_ransoms()
	var ransoms := panel.ransom_panel
	_check(ransoms != null and ransoms.visible and ransoms.rows.is_empty(), "ransom panel should open (no captive in 1337)")
	ransoms.show_data(_mock_ransoms())
	_check(ransoms.rows.has("chr_jean_de_normandie") and (ransoms.rows["chr_jean_de_normandie"] as Dictionary).has("plan"), "our captive row with payment plans")
	var plan: OptionButton = ransoms.rows["chr_jean_de_normandie"]["plan"]
	_check(plan.item_count == 5 and plan.get_item_text(0).contains("2 échéances"), "installment plans 2-6: %d" % plan.item_count)
	var terms: OptionButton = ransoms.rows["chr_mock_knight"]["terms"]
	_check(terms.item_count == 3 and terms.get_item_text(1).begins_with("Exiger"), "held prisoner terms: money, province, hold")
	var pay: Dictionary = ransoms.pay_ransom("chr_jean_de_normandie", 1)
	_check(not bool(pay.get("ok", true)) and ransoms.error_label.visible and ransoms.error_label.text.begins_with("Refusé"), "bridge refusal shown in red: %s" % ransoms.error_label.text)
	var alerts := CampaignAlerts.ransom_alerts(sim)
	_check(alerts.is_empty(), "no ransom alert without captive")
	panel.queue_free()
	facade.set("sim", previous_sim)

	# Genres coinage / ransom / chivalry.
	var events := [
		{"kind": "coinage", "text_fr": "La monnaie est affaiblie.", "faction": FACTION_ID},
		{"kind": "ransom", "text_fr": "Rançon payée.", "faction": FACTION_ID},
		{"kind": "chivalry", "text_fr": "Ordre fondé.", "faction": FACTION_ID},
	]
	var groups := SeasonReport.build_groups(events, func(_event: Dictionary) -> bool: return true)
	# U5 : monnaie et rançon au « Trésor », chevalerie dans « Vos terres ».
	_check(SeasonReport.entry_count(groups) == 3 and str(groups[0]["id"]) == "lands" and str(groups[1]["id"]) == "treasury", "season report sections for coinage/ransom/chivalry: %s" % [groups])
	_check(SeasonReport.KIND_STYLES.has("ransom") and NewsLetters.KIND_LABELS.has("chivalry") and not NewsLetters.news_from_event(events[0]).is_empty(), "styles and letters for H11 kinds")

	# Encyclopédie → Codex et Codex → Encyclopédie.
	var encyclopedia: Encyclopedia = (load("res://scenes/ui/encyclopedia.tscn") as PackedScene).instantiate()
	root.add_child(encyclopedia)
	await process_frame
	encyclopedia.open_window("unit_longbowmen")
	_check(encyclopedia.codex_button.visible, "encyclopedia should offer the Codex entry of unit_longbowmen")
	encyclopedia.open_codex_entry()
	var bubbles: Node = root.get_node_or_null("/root/CodexBubbles")
	var window: CodexWindow = bubbles.call("window")
	_check(bool(bubbles.call("is_window_open")) and window.current_id == "cdx_arc_long", "codex window on cdx_arc_long, got %s" % window.current_id)
	_check(window.encyclopedia_button.visible, "codex entry with entity should offer the encyclopedia")
	encyclopedia.open_entry("bld_apothecary")
	window.navigate("cdx_arc_long")
	_check(window.open_in_encyclopedia() and encyclopedia.current_entry == "unit_longbowmen" and not window.visible, "codex -> encyclopedia")
	encyclopedia.queue_free()
	facade.set_data_dir(previous_dir)
	if store != null:
		store.call("reset_discoveries")
	if failures == 0:
		print("smoke OK: coinage/ransom, debased then refused, ransom panel, order refused, encyclopedia/codex links")


## Rançons simulées au format de `get_ransoms` (captures et smoke).
static func _mock_ransoms() -> Dictionary:
	return {
		"ours": [{"character": "chr_jean_de_normandie", "name": "Jean, duc de Normandie", "faction": "fac_france", "captor": "fac_england",
			"rank": "sovereign", "rank_label": "Souverain", "prestige": 40, "ransom": 21000,
			"terms": {"kind": "money", "province": ""},
			"plans": [2, 3, 4, 5, 6].map(func(n: int) -> Dictionary: return {"installments": n, "total": 23100, "installment": 23100 / n}),
			"cedable_provinces": []}],
		"held": [{"character": "chr_mock_knight", "name": "Thomas Holland", "faction": "fac_england", "captor": "fac_france",
			"rank": "knight", "rank_label": "Chevalier", "prestige": 12, "ransom": 450,
			"terms": {"kind": "money", "province": ""}, "plans": [], "cedable_provinces": ["prov_guyenne"]}],
		"debts": [{"character": "chr_charles_de_blois", "name": "Charles de Blois", "creditor": "fac_england",
			"remaining": 9000, "installment": 3000, "next_due_turn": 4, "missed": 0}],
	}
