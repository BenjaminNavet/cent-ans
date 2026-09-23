class_name LoadingScreen
extends CanvasLayer

## F3 — écran de chargement entre le menu et la carte : fond illustré voilé, écu et nom de
## la faction, étapes cochées au fil du chargement, barre de progression, conseil.
##
## Étapes (celles de `campaign_map.tscn`) :
##  1. ressources de la scène (chargement en tâche de fond, progression réelle du
##     `ResourceLoader`) ;
##  2. carte et simulation : `_ready` de la carte (lecture de `data/map/`, relief, tuiles de
##     terrain, rivières, côtes, villes, puis campagne neuve ou sauvegarde) — bloc synchrone,
##     l'écran est dessiné juste avant ;
##  3. premières images (compilation des shaders), puis fondu vers la carte.
## Usage : `LoadingScreen.start(get_tree())` après avoir renseigné `SimFacade.pending_*`.

signal finished(scene: Node)

const SCENE_PATH := "res://scenes/ui/loading_screen.tscn"
const CAMPAIGN_SCENE := "res://scenes/campaign_map.tscn"
const SETTLE_FRAMES := 12
const FADE_SECONDS := 0.45
const STEPS := [
	"Ressources de la carte",
	"Relief, provinces et terrain ; mise en place de la campagne",
	"Premières images",
]
## Conseils affichés pendant le chargement (texte d'interface).
const TIPS := [
	"Un tour est une saison : l'hiver ralentit les armées et affame celles qui sont en terre ennemie.",
	"Échap ouvre le menu pause : sauvegarde, réglages, aide et retour au menu.",
	"Le rapport de saison résume chaque fin de tour ; cliquez une ligne pour y porter la caméra.",
	"Les alertes à droite signalent sièges, armées ennemies aux frontières et dettes.",
	"Un général expérimenté vaut plusieurs compagnies : confiez vos armées à vos meilleurs vassaux.",
	"La chronique propose des décisions historiques : vous avez deux tours pour choisir.",
	"Les archers anglais redoutent la pluie ; les chevaliers, la boue et les pieux.",
]

var scene_path: String = CAMPAIGN_SCENE
var progress: float = 0.0
var step_index: int = -1
var result_scene: Node = null

var _root_control: Control
var _background: MenuBackground
var _bar: ProgressBar
var _step_labels: Array[Label] = []
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
	_root_control.set_anchors_preset(Control.PRESET_FULL_RECT)
	_root_control.theme = load("res://scenes/ui/parchment_theme.tres")
	add_child(_root_control)
	_background = MenuBackground.new()
	_background.dim = 0.35
	_root_control.add_child(_background)

	var center := CenterContainer.new()
	center.set_anchors_preset(Control.PRESET_FULL_RECT)
	_root_control.add_child(center)
	var panel := PanelContainer.new()
	panel.custom_minimum_size = Vector2(620, 0)
	center.add_child(panel)
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 10)
	panel.add_child(box)

	var facade := get_node_or_null("/root/SimFacade")
	var header := HBoxContainer.new()
	header.add_theme_constant_override("separation", 14)
	box.add_child(header)
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
	var shield_texture := PortraitLoader.heraldry_texture(faction_id)
	if shield_texture != null:
		var shield := TextureRect.new()
		shield.texture = shield_texture
		shield.custom_minimum_size = Vector2(64, 72)
		shield.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
		shield.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
		header.add_child(shield)
	var titles := VBoxContainer.new()
	header.add_child(titles)
	var title := Label.new()
	title.text = str(facade.call("faction_info", faction_id).get("name", "Cent Ans")) if facade != null and faction_id != "" else "Cent Ans"
	title.add_theme_font_size_override("font_size", 28)
	titles.add_child(title)
	var sub := Label.new()
	sub.text = subtitle
	sub.add_theme_color_override("font_color", Color(0.40, 0.28, 0.14))
	titles.add_child(sub)

	box.add_child(HSeparator.new())
	for text in STEPS:
		var label := Label.new()
		label.text = "○  %s" % text
		label.add_theme_color_override("font_color", Color(0.45, 0.38, 0.30))
		box.add_child(label)
		_step_labels.append(label)
	_bar = ProgressBar.new()
	_bar.min_value = 0.0
	_bar.max_value = 1.0
	_bar.step = 0.001
	_bar.custom_minimum_size = Vector2(0, 18)
	box.add_child(_bar)
	var tip := Label.new()
	tip.text = "Conseil : %s" % TIPS[randi() % TIPS.size()]
	tip.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	tip.add_theme_font_size_override("font_size", 14)
	tip.add_theme_color_override("font_color", Color(0.35, 0.24, 0.12))
	box.add_child(tip)

	_fade = ColorRect.new()
	_fade.color = Color(0, 0, 0, 1)
	_fade.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_fade.set_anchors_preset(Control.PRESET_FULL_RECT)
	_root_control.add_child(_fade)


func _set_step(index: int, value: float) -> void:
	for i in _step_labels.size():
		var label := _step_labels[i]
		if i < index:
			label.text = "✓  %s" % STEPS[i]
			label.add_theme_color_override("font_color", Color(0.22, 0.36, 0.16))
		elif i == index:
			label.text = "▸  %s…" % STEPS[i]
			label.add_theme_color_override("font_color", Color(0.22, 0.14, 0.07))
	step_index = index
	_set_progress(value)


func _set_progress(value: float) -> void:
	progress = clampf(value, 0.0, 1.0)
	_bar.value = progress


func _frames(count: int) -> void:
	for _i in count:
		await get_tree().process_frame


func run() -> void:
	var tree := get_tree()
	var tween := create_tween()
	tween.tween_property(_fade, "color:a", 0.0, FADE_SECONDS * 0.6)
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
	fade_out.tween_property(_root_control, "modulate:a", 0.0, FADE_SECONDS)
	await fade_out.finished
	finished.emit(scene)
	queue_free()
