extends Control

## Aperçu des composants du HUD de campagne (lot F10a) sur un fond de carte, avec des données
## de démonstration de 1337 (ost de Philippe VI, confiscation de la Guyenne…).
##
## Usage : godot --path game res://tests/hud_preview.tscn [--resolution 1920x1080] -- [options]
##   --screenshot=<chemin.png>  capture après quelques images puis quitte
##   --units=<n>                armée de n régiments (test des deux rangs, max 20)
##   --expand                   déplie la première lettre
##   --no-block                 aucune décision bloquante (cloche normale)
##   --select=<i,j>             sélection multiple dans le bandeau
##   --no-general               armée sans chef

const MAP_BACKGROUND := "res://../docs/img/audit-2026-09-23/map-army.jpg"
## Zone de la capture d'audit sans interface (barre haute et panneaux exclus).
const MAP_REGION := Rect2(0, 40, 840, 500)
const MARGIN := 16.0
const MINIMAP_SIZE := Vector2(240, 168)
const TOP_BAR_HEIGHT := 36.0
const SCREENSHOT_DELAY_FRAMES := 12

var seal: GeneralSeal
var strip: ArmyStrip
var cluster: EndTurnCluster
var letters: NewsLetters
var _minimap: Panel
var _screenshot_path := ""
var _countdown := -1


func _ready() -> void:
	set_anchors_preset(Control.PRESET_FULL_RECT)
	theme = load("res://scenes/ui/parchment_theme.tres")
	_add_background()
	_add_placeholders()
	seal = (load("res://scenes/ui/general_seal.tscn") as PackedScene).instantiate()
	strip = (load("res://scenes/ui/army_strip.tscn") as PackedScene).instantiate()
	cluster = (load("res://scenes/ui/end_turn_cluster.tscn") as PackedScene).instantiate()
	letters = (load("res://scenes/ui/news_letters.tscn") as PackedScene).instantiate()
	for node in [seal, strip, cluster, letters]:
		add_child(node)
	_fill_demo(OS.get_cmdline_user_args())
	resized.connect(layout_hud)
	# Le bandeau se redimensionne après `set_army` / `max_width` : replacer au prochain cycle.
	strip.minimum_size_changed.connect(func() -> void: layout_hud.call_deferred())
	letters.resized.connect(layout_hud)
	layout_hud.call_deferred()
	for signal_name in ["unit_selected", "selection_changed", "split_requested"]:
		strip.connect(signal_name, func(value: Variant) -> void: print("ArmyStrip.%s %s" % [signal_name, value]))
	seal.general_requested.connect(func(id: String) -> void: print("GeneralSeal.general_requested ", id))
	cluster.end_turn_requested.connect(func() -> void: print("EndTurnCluster.end_turn_requested"))
	cluster.alert_activated.connect(func(alert: Dictionary) -> void: print("EndTurnCluster.alert_activated ", alert))
	letters.news_activated.connect(func(item: Dictionary) -> void: print("NewsLetters.news_activated ", item.get("title")))
	letters.news_dismissed.connect(func(item: Dictionary) -> void: print("NewsLetters.news_dismissed ", item.get("title")))


## Placement de référence (à reproduire dans `map_ui`) : sceau en bas à gauche, cloche en bas
## à droite, bandeau centré dans l'espace libre entre les deux, lettres sous la minicarte.
func layout_hud() -> void:
	var view := size
	seal.position = Vector2(MARGIN, view.y - seal.size.y - MARGIN)
	cluster.size = cluster.get_combined_minimum_size()
	cluster.position = view - cluster.size - Vector2(MARGIN * 0.5, MARGIN * 0.5)
	var left := seal.position.x + seal.get_combined_minimum_size().x + 16.0
	var right := cluster.position.x + cluster.fan_left_edge() - 10.0
	var gap := right - left
	strip.max_width = gap
	strip.size = Vector2.ZERO  # rétrécit à la taille minimale
	strip.position = Vector2(left + (gap - strip.size.x) * 0.5, view.y - strip.size.y - MARGIN * 0.75)
	_minimap.position = Vector2(view.x - MINIMAP_SIZE.x - MARGIN, TOP_BAR_HEIGHT + 8.0)
	letters.position = Vector2(view.x - NewsLetters.LETTER_WIDTH - MARGIN, _minimap.position.y + MINIMAP_SIZE.y + 10.0)


func _process(_delta: float) -> void:
	if _countdown > 0:
		_countdown -= 1
		if _countdown == 0:
			_take_screenshot()


func _add_background() -> void:
	var texture: Texture2D = null
	var absolute := ProjectSettings.globalize_path(MAP_BACKGROUND)
	if FileAccess.file_exists(absolute):
		var image := Image.load_from_file(absolute)
		if image != null and not image.is_empty():
			var atlas := AtlasTexture.new()
			atlas.atlas = ImageTexture.create_from_image(image)
			atlas.region = MAP_REGION
			texture = atlas
	if texture == null:
		var fill := ColorRect.new()
		fill.color = Color(0.55, 0.60, 0.45)
		fill.set_anchors_preset(Control.PRESET_FULL_RECT)
		add_child(fill)
		return
	var rect := TextureRect.new()
	rect.texture = texture
	rect.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	rect.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_COVERED
	rect.set_anchors_preset(Control.PRESET_FULL_RECT)
	rect.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(rect)


## Barre haute et minicarte factices (hors lot) pour juger les proportions.
func _add_placeholders() -> void:
	var bar := Panel.new()
	var bar_box := HudStyle.panel_box(4, 0)
	bar_box.shadow_size = 3
	bar.add_theme_stylebox_override("panel", bar_box)
	bar.set_anchors_preset(Control.PRESET_TOP_WIDE)
	bar.custom_minimum_size = Vector2(0, TOP_BAR_HEIGHT)
	add_child(bar)
	var bar_label := HudStyle.label("France    Trésor 60 000 ₶ (+4 218)    Printemps 1337", HudStyle.FONT_BODY)
	bar_label.position = Vector2(16, 8)
	bar.add_child(bar_label)
	_minimap = Panel.new()
	_minimap.add_theme_stylebox_override("panel", HudStyle.panel_box(4))
	_minimap.size = MINIMAP_SIZE
	add_child(_minimap)
	var mini_label := HudStyle.label("Minicarte-portulan (F6)", HudStyle.FONT_SMALL, HudStyle.INK_FADED)
	mini_label.position = Vector2(46, 74)
	_minimap.add_child(mini_label)


func _fill_demo(args: PackedStringArray) -> void:
	var unit_count := 8
	var expand := false
	var blocking := true
	var selection: Array = []
	var with_general := true
	for arg in args:
		if arg.begins_with("--screenshot="):
			_screenshot_path = arg.trim_prefix("--screenshot=")
			_countdown = SCREENSHOT_DELAY_FRAMES
		elif arg.begins_with("--units="):
			unit_count = clampi(int(arg.trim_prefix("--units=")), 0, 20)
		elif arg == "--expand":
			expand = true
		elif arg == "--no-block":
			blocking = false
		elif arg == "--no-general":
			with_general = false
		elif arg.begins_with("--select="):
			for part in arg.trim_prefix("--select=").split(",", false):
				selection.append(int(part))

	var catalog := ArmyStrip.load_unit_catalog(ProjectSettings.globalize_path("res://../data"))
	var army := demo_army(unit_count)
	var character := demo_character() if with_general else {}
	if not with_general:
		army["general"] = ""
		army["general_name"] = ""
	seal.set_general(character, army, "fac_france")
	strip.set_army(army, 20, catalog, "Ost de Philippe VI" if with_general else "Ost de France")
	if not selection.is_empty():
		strip.select(selection)
	cluster.set_date("Printemps 1337", 0)
	var alerts := demo_alerts()
	if not blocking:
		alerts = alerts.filter(func(a: Dictionary) -> bool: return not EndTurnCluster.is_blocking(a))
	cluster.set_alerts(alerts)
	for item in demo_news():
		letters.push_news(item)
	if expand:
		letters.activate(0)


## Armée de Philippe VI telle que la renvoie `CampaignSim.get_army` (8 régiments, ou `count`).
static func demo_army(count: int = 8) -> Dictionary:
	var base := [
		{"unit_type": "unit_knights", "name": "Chevaliers", "strength": 60, "max_strength": 60, "morale": 80},
		{"unit_type": "unit_knights", "name": "Chevaliers", "strength": 52, "max_strength": 60, "morale": 74},
		{"unit_type": "unit_knights", "name": "Chevaliers", "strength": 60, "max_strength": 60, "morale": 80},
		{"unit_type": "unit_men_at_arms_foot", "name": "Hommes d'armes à pied", "strength": 80, "max_strength": 80, "morale": 75},
		{"unit_type": "unit_men_at_arms_foot", "name": "Hommes d'armes à pied", "strength": 61, "max_strength": 80, "morale": 58},
		{"unit_type": "unit_crossbowmen", "name": "Arbalétriers", "strength": 100, "max_strength": 100, "morale": 45},
		{"unit_type": "unit_genoese_crossbowmen", "name": "Arbalétriers génois", "strength": 34, "max_strength": 100, "morale": 18},
		{"unit_type": "unit_mounted_sergeants", "name": "Sergents montés", "strength": 80, "max_strength": 80, "morale": 55},
	]
	var extra := [
		{"unit_type": "unit_urban_militia", "name": "Milice urbaine", "strength": 120, "max_strength": 120, "morale": 35},
		{"unit_type": "unit_flemish_pikemen", "name": "Piquiers flamands", "strength": 110, "max_strength": 120, "morale": 50},
		{"unit_type": "unit_trebuchet", "name": "Trébuchet", "strength": 20, "max_strength": 20, "morale": 40},
	]
	var units: Array = []
	for i in count:
		units.append((base[i] if i < base.size() else extra[(i - base.size()) % extra.size()]).duplicate())
	return {
		"faction": "fac_france", "general": "chr_philippe_vi", "general_name": "Philippe VI de Valois",
		"location": "prov_ile_de_france", "units": units, "movement_points": 3, "supply": 92,
		"stance": "normal", "path": [], "embarked": false,
	}


## Fiche telle que la renvoie `CampaignSim.get_character`.
static func demo_character() -> Dictionary:
	return {
		"id": "chr_philippe_vi", "name": "Philippe VI de Valois", "epithet": "", "faction": "fac_france",
		"title": "Roi de France", "skills": {"command": 5, "governance": 4, "court": 6},
		"skill_points": 2, "skills_learned": ["skl_command_1", "skl_court_1"], "experience": 140,
	}


static func demo_alerts() -> Array:
	return [
		{"kind": "chronicle_decision", "text": "Confisquer la Guyenne d'Édouard III, vassal félon ?", "blocking": true},
		{"kind": "enemy_army", "text": "Ost anglais signalé en Flandre wallonne", "province_id": "prov_flandre_wallonne", "army_id": "army_12"},
		{"kind": "enemy_army", "text": "Troupes du comte de Hainaut aux marches du Cambrésis", "province_id": "prov_hainaut", "army_id": "army_17"},
		{"kind": "siege", "text": "Le connétable assiège Saint-Macaire (Guyenne)", "province_id": "prov_guyenne"},
		{"kind": "debt", "text": "Trésor négatif dans 3 saisons au train actuel"},
		{"kind": "idle_character", "text": "Jean, duc de Normandie, est sans affectation", "character_id": "chr_jean_de_normandie"},
		{"kind": "construction_done", "text": "Paris : halle aux draps achevée", "province_id": "prov_ile_de_france"},
		{"kind": "research_done", "text": "Recherche achevée : arbalète à tour"},
	]


## Nouvelles de 1337, de la plus ancienne à la plus récente.
static func demo_news() -> Array:
	return [
		{"kind": "birth", "title": "Naissance de Venceslas, fils du roi Jean de Bohême",
			"text": "À Prague, la reine Béatrice donne un fils au roi aveugle.", "faction_id": "fac_bohemia"},
		{"kind": "death", "title": "Mort de Guillaume Iᵉʳ, comte de Hainaut",
			"text": "Le beau-père d'Édouard III s'éteint à Valenciennes.", "faction_id": "fac_hainaut"},
		{"kind": "succession", "title": "Guillaume II succède en Hainaut",
			"text": "Le nouveau comte penche pour le parti anglais.", "faction_id": "fac_hainaut"},
		{"kind": "faction_met", "title": "Le comte Louis de Nevers se range au parti du roi",
			"text": "La Flandre reste fidèle à son suzerain, malgré les villes drapières.", "faction_id": "fac_flanders"},
		{"kind": "war_declared", "title": "Philippe VI confisque la Guyenne",
			"text": "Le 24 mai 1337, le roi déclare le duché confisqué pour félonie : Édouard III a donné asile à Robert d'Artois.",
			"faction_id": "fac_france", "province_id": "prov_guyenne"},
		{"kind": "war_declared", "title": "Édouard III revendique la couronne de France",
			"text": "Petit-fils de Philippe le Bel par sa mère Isabelle, le roi d'Angleterre défie « Philippe de Valois, qui se dit roi de France ».",
			"faction_id": "fac_england"},
	]


func _take_screenshot() -> void:
	await RenderingServer.frame_post_draw
	var image := get_viewport().get_texture().get_image()
	var path := _screenshot_path
	if path.is_relative_path() and not path.begins_with("res://") and not path.begins_with("user://"):
		path = ProjectSettings.globalize_path("res://").path_join("..").path_join(path).simplify_path()
	DirAccess.make_dir_recursive_absolute(path.get_base_dir())
	var err := image.save_png(path)
	print("HudPreview: screenshot %s (%s, %dx%d)" % [path, error_string(err), image.get_width(), image.get_height()])
	get_tree().quit(0 if err == OK else 1)
