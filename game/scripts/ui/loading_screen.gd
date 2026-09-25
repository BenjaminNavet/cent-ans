class_name LoadingScreen
extends CanvasLayer

## F3, refait par MM1 — écran de chargement entre le menu et la carte : miniature du Codex ou
## des événements (de préférence propre à la faction) en grand dans un cadre d'or, la même en
## fond assombri, écu et nom de la faction, citation historique datée, conseil de jeu, étape en
## cours et barre de progression. Textes et illustrations : `data/ui/front_end.json` (`loading`).
##
## Étapes (celles de `campaign_map.tscn`) :
##  1. ressources de la scène (chargement en tâche de fond, progression réelle du
##     `ResourceLoader`) ;
##  2. carte et simulation : `_ready` de la carte (lecture de `data/map/`, relief, tuiles de
##     terrain, rivières, côtes, villes, puis campagne neuve ou sauvegarde) — bloc synchrone,
##     l'écran est dessiné juste avant ;
##  3. premières images (compilation des shaders), puis fondu vers la carte.
## Usage : `LoadingScreen.start(get_tree())` après avoir renseigné `SimFacade.pending_*`.
## Capture : `-- --loading-shot=<chemin.png>` enregistre l'écran à l'étape 2 puis quitte.

signal finished(scene: Node)

const SCENE_PATH := "res://scenes/ui/loading_screen.tscn"
const CAMPAIGN_SCENE := "res://scenes/campaign_map.tscn"
const SETTLE_FRAMES := 12
const FADE_SECONDS := 0.45
const ART_SIZE := Vector2(800, 450)
const STEPS := [
	"Ressources de la carte",
	"Relief, provinces et terrain ; mise en place de la campagne",
	"Premières images",
]

var scene_path: String = CAMPAIGN_SCENE
var progress: float = 0.0
var step_index: int = -1
var result_scene: Node = null
## Illustration, citation et conseil tirés pour cet écran (captures, tests).
var illustration_path: String = ""
var quote: Dictionary = {}
var tip: String = ""

var _root_control: Control
var _bar: ProgressBar
var _step_label: Label
var _percent_label: Label
var _fade: ColorRect


## Crée l'écran à la racine et lance le chargement de la carte ; renvoie l'écran.
static func start(tree: SceneTree, path: String = CAMPAIGN_SCENE) -> LoadingScreen:
	var screen: LoadingScreen = (load(SCENE_PATH) as PackedScene).instantiate()
	screen.scene_path = path
	tree.root.add_child(screen)
	screen.run.call_deferred()
	return screen


func _ready() -> void:
	layer = 100
	process_mode = Node.PROCESS_MODE_ALWAYS
	_build()


func _build() -> void:
	_root_control = Control.new()
	_root_control.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	_root_control.theme = load("res://scenes/ui/parchment_theme.tres")
	add_child(_root_control)

	var facade := get_node_or_null("/root/SimFacade")
	var faction_id := ""
	var subtitle := "Nouvelle campagne — printemps 1337"
	if facade != null:
		faction_id = str(facade.get("pending_faction"))
		var load_path := str(facade.get("pending_load_path"))
		if load_path != "":
			var wrapper_name := load_path.get_file().get_basename()
			subtitle = "Chargement : %s" % SaveSlots.display_name(wrapper_name)
			var meta_file := SaveSlots.meta_path(wrapper_name)
			if FileAccess.file_exists(meta_file):
				var meta: Variant = JSON.parse_string(FileAccess.get_file_as_string(meta_file))
				if meta is Dictionary:
					faction_id = str(meta.get("faction", faction_id))
					subtitle += " — %s" % meta.get("date", "")
	illustration_path = FrontEndData.random_illustration(faction_id)
	quote = FrontEndData.random_quote()
	tip = FrontEndData.random_tip()
	var art_texture := PortraitLoader.load_texture(illustration_path) if illustration_path != "" else null

	# Fond : la miniature couvrant l'écran, très assombrie ; à défaut, la carte ancienne.
	var base := ColorRect.new()
	base.color = FrontEndStyle.NIGHT
	base.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	_root_control.add_child(base)
	if art_texture != null:
		var backdrop := TextureRect.new()
		backdrop.texture = art_texture
		backdrop.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
		backdrop.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_COVERED
		backdrop.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
		backdrop.modulate = Color(0.24, 0.2, 0.17)
		_root_control.add_child(backdrop)
	else:
		var map := MenuBackground.new()
		map.dim = 0.6
		_root_control.add_child(map)

	var margin := MarginContainer.new()
	margin.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	margin.add_theme_constant_override("margin_left", 64)
	margin.add_theme_constant_override("margin_right", 64)
	margin.add_theme_constant_override("margin_top", 48)
	margin.add_theme_constant_override("margin_bottom", 40)
	_root_control.add_child(margin)
	var column := VBoxContainer.new()
	column.add_theme_constant_override("separation", 18)
	margin.add_child(column)

	# En-tête : écu, nom de la faction, sous-titre.
	var header := HBoxContainer.new()
	header.add_theme_constant_override("separation", 16)
	column.add_child(header)
	var shield_texture := PortraitLoader.heraldry_texture(faction_id)
	if shield_texture != null:
		var shield := TextureRect.new()
		shield.texture = shield_texture
		shield.custom_minimum_size = Vector2(56, 64)
		shield.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
		shield.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
		header.add_child(shield)
	var titles := VBoxContainer.new()
	titles.add_theme_constant_override("separation", -4)
	header.add_child(titles)
	var name := str(facade.call("faction_info", faction_id).get("name", "Cent Ans")) if facade != null and faction_id != "" else "Cent Ans"
	titles.add_child(FrontEndStyle.label(name, 38, Color(0.97, 0.92, 0.80), FrontEndStyle.title_font(), 6))
	titles.add_child(FrontEndStyle.label(subtitle, 19, FrontEndStyle.GOLD, FrontEndStyle.title_italic(), 4))

	# Corps : miniature encadrée à gauche, citation et conseil à droite.
	var body := HBoxContainer.new()
	body.add_theme_constant_override("separation", 44)
	body.size_flags_vertical = Control.SIZE_EXPAND_FILL
	column.add_child(body)
	var frame := PanelContainer.new()
	var frame_style := StyleBoxFlat.new()
	frame_style.bg_color = FrontEndStyle.GOLD_DARK
	frame_style.border_color = FrontEndStyle.GOLD
	frame_style.set_border_width_all(3)
	frame_style.set_content_margin_all(6)
	frame_style.shadow_color = Color(0, 0, 0, 0.6)
	frame_style.shadow_size = 20
	frame.add_theme_stylebox_override("panel", frame_style)
	frame.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	body.add_child(frame)
	var art := TextureRect.new()
	art.texture = art_texture
	art.custom_minimum_size = ART_SIZE
	art.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	art.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_COVERED
	frame.add_child(art)

	var side := VBoxContainer.new()
	side.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	side.alignment = BoxContainer.ALIGNMENT_CENTER
	side.add_theme_constant_override("separation", 14)
	body.add_child(side)
	if not quote.is_empty():
		var mark := FrontEndStyle.label("«", 64, Color(FrontEndStyle.GOLD, 0.8), FrontEndStyle.title_font())
		mark.custom_minimum_size = Vector2(0, 40)
		side.add_child(mark)
		var quote_label := FrontEndStyle.label(str(quote.get("text", "")), 26, Color(0.95, 0.90, 0.78), FrontEndStyle.title_italic(), 4)
		quote_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		side.add_child(quote_label)
		var source := str(quote.get("author", ""))
		if str(quote.get("source", "")) != "":
			source += ", %s" % quote["source"]
		var attribution := FrontEndStyle.label("— %s (%s)" % [source, quote.get("date", "")], 18, FrontEndStyle.GOLD, FrontEndStyle.body_font(), 3)
		attribution.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		side.add_child(attribution)
	var rule := ColorRect.new()
	rule.color = Color(FrontEndStyle.GOLD, 0.5)
	rule.custom_minimum_size = Vector2(0, 1)
	side.add_child(rule)
	if tip != "":
		side.add_child(FrontEndStyle.label("Conseil", 20, FrontEndStyle.GOLD, FrontEndStyle.title_font(), 3))
		var tip_label := FrontEndStyle.label(tip, 19, Color(0.88, 0.83, 0.72), FrontEndStyle.body_font(), 3)
		tip_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		side.add_child(tip_label)

	# Pied : étape en cours, pourcentage, barre dorée.
	var footer := HBoxContainer.new()
	column.add_child(footer)
	_step_label = FrontEndStyle.label("", 18, Color(0.88, 0.83, 0.72), FrontEndStyle.body_italic(), 3)
	_step_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	footer.add_child(_step_label)
	_percent_label = FrontEndStyle.label("", 18, FrontEndStyle.GOLD, FrontEndStyle.title_font(), 3)
	footer.add_child(_percent_label)
	_bar = ProgressBar.new()
	_bar.min_value = 0.0
	_bar.max_value = 1.0
	_bar.step = 0.001
	_bar.show_percentage = false
	_bar.custom_minimum_size = Vector2(0, 12)
	var bar_bg := StyleBoxFlat.new()
	bar_bg.bg_color = Color(0, 0, 0, 0.6)
	bar_bg.border_color = Color(FrontEndStyle.GOLD_DARK, 0.9)
	bar_bg.set_border_width_all(1)
	var bar_fill := StyleBoxFlat.new()
	bar_fill.bg_color = FrontEndStyle.GOLD
	_bar.add_theme_stylebox_override("background", bar_bg)
	_bar.add_theme_stylebox_override("fill", bar_fill)
	column.add_child(_bar)

	_fade = ColorRect.new()
	_fade.color = Color(0, 0, 0, 1)
	_fade.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_fade.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	_root_control.add_child(_fade)


func _set_step(index: int, value: float) -> void:
	if index < STEPS.size():
		_step_label.text = "%s…  (%d / %d)" % [STEPS[index], index + 1, STEPS.size()]
	else:
		_step_label.text = "Prêt."
	step_index = index
	_set_progress(value)


func _set_progress(value: float) -> void:
	progress = clampf(value, 0.0, 1.0)
	_bar.value = progress
	_percent_label.text = "%d %%" % int(round(progress * 100.0))


func _frames(count: int) -> void:
	for _i in count:
		await get_tree().process_frame


func run() -> void:
	var tree := get_tree()
	var tween := create_tween()
	var fade_time := 0.01 if Accessibility.reduce_motion() else FADE_SECONDS  # U12
	tween.tween_property(_fade, "color:a", 0.0, fade_time * 0.6)
	await tween.finished

	# 1. Ressources (chargement en tâche de fond).
	_set_step(0, 0.02)
	var packed: PackedScene = null
	if ResourceLoader.load_threaded_request(scene_path) == OK:
		var status_progress: Array = []
		while true:
			var status := ResourceLoader.load_threaded_get_status(scene_path, status_progress)
			if status == ResourceLoader.THREAD_LOAD_LOADED:
				packed = ResourceLoader.load_threaded_get(scene_path)
				break
			if status != ResourceLoader.THREAD_LOAD_IN_PROGRESS:
				break
			if not status_progress.is_empty():
				_set_progress(0.02 + 0.33 * float(status_progress[0]))
			await tree.process_frame
	if packed == null:
		packed = load(scene_path)

	# 2. Carte et simulation (synchrone : l'écran est dessiné avant).
	_set_step(1, 0.38)
	await _frames(2)
	await _maybe_screenshot()
	var scene := packed.instantiate() if packed != null else null
	var previous := tree.current_scene
	if scene != null:
		tree.root.add_child(scene)
		tree.current_scene = scene
	if previous != null and previous != scene:
		previous.queue_free()
	result_scene = scene

	# 3. Premières images puis fondu.
	_set_step(2, 0.85)
	for frame in SETTLE_FRAMES:
		await tree.process_frame
		_set_progress(0.85 + 0.15 * float(frame + 1) / SETTLE_FRAMES)
	_set_step(STEPS.size(), 1.0)
	var fade_out := create_tween()
	fade_out.tween_property(_root_control, "modulate:a", 0.0, fade_time)
	await fade_out.finished
	finished.emit(scene)
	queue_free()


func _maybe_screenshot() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--loading-shot="):
			var path := arg.trim_prefix("--loading-shot=")
			await RenderingServer.frame_post_draw
			var image := get_viewport().get_texture().get_image()
			DirAccess.make_dir_recursive_absolute(path.get_base_dir())
			var err := image.save_png(path)
			print("LoadingScreen: screenshot %s (%s)" % [path, error_string(err)])
			get_tree().quit(0 if err == OK else 1)
			await get_tree().process_frame
