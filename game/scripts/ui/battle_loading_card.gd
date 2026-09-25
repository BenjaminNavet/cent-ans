class_name BattleLoadingCard
extends CanvasLayer

## Lot AR1 — écran de chargement illustré des batailles, sièges et batailles navales : une
## enluminure (Froissart, Vigiles de Charles VII…) en grand dans un cadre d'or, la même en fond
## assombri, le titre de l'épisode et une citation de chroniqueur (`data/ui/illustrations.json`,
## `loading`). Affiché au moins `min_seconds` (3 s) même si la scène se construit plus vite ;
## la construction de la scène (bloc synchrone) se fait pendant l'affichage.
## Usage :
##   var card := BattleLoadingCard.open(get_tree(), "siege")
##   await card.drawn           # l'écran est à l'image
##   ... instancier la bataille ...
##   card.close()               # attend le reste du temps minimal, fondu, puis libère
## Capture : `-- --battle-loading-shot=<chemin.png>` enregistre l'écran puis quitte.

signal drawn
signal closed

const FADE_SECONDS := 0.35
const ART_MAX := Vector2(1100, 619)

var context: String = "battle"
## Écran tiré (captures, tests).
var screen: Dictionary = {}
var _root_control: Control
var _opened_msec: int = 0
var _closing: bool = false


## Crée la carte à la racine ; `drawn` est émis une fois l'écran rendu.
static func open(tree: SceneTree, p_context: String, forced_id: String = "") -> BattleLoadingCard:
	var card := BattleLoadingCard.new()
	card.context = p_context
	if forced_id != "":
		for entry in ArtPlates.loading_screens(p_context):
			if str(entry.get("id", "")) == forced_id:
				card.screen = entry
	tree.root.add_child.call_deferred(card)  # sûr pendant `_ready` d'une scène
	return card


func _ready() -> void:
	layer = 110
	process_mode = Node.PROCESS_MODE_ALWAYS
	if screen.is_empty():
		screen = ArtPlates.random_loading_screen(context)
	_build()
	_opened_msec = Time.get_ticks_msec()
	_announce.call_deferred()


func _announce() -> void:
	# Deux images traitées : l'écran est dessiné (frame_post_draw n'arrive pas en --headless).
	await get_tree().process_frame
	await get_tree().process_frame
	await _maybe_screenshot()
	drawn.emit()


## Termine l'écran : attend la durée minimale restante, fondu, puis se libère.
func close() -> void:
	if _closing:
		return
	_closing = true
	var reduce := Accessibility.reduce_motion()
	var remaining := ArtPlates.min_seconds() - float(Time.get_ticks_msec() - _opened_msec) / 1000.0
	if remaining > 0.0:
		await get_tree().create_timer(remaining, true, false, true).timeout
	var tween := create_tween()
	tween.tween_property(_root_control, "modulate:a", 0.0, 0.01 if reduce else FADE_SECONDS)
	await tween.finished
	closed.emit()
	queue_free()


func _build() -> void:
	_root_control = Control.new()
	_root_control.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	_root_control.mouse_filter = Control.MOUSE_FILTER_STOP
	add_child(_root_control)
	var base := ColorRect.new()
	base.color = FrontEndStyle.NIGHT
	base.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	_root_control.add_child(base)
	var art_texture := ArtPlates.texture(screen)
	if art_texture != null:
		var backdrop := TextureRect.new()
		backdrop.texture = art_texture
		backdrop.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
		backdrop.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_COVERED
		backdrop.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
		backdrop.modulate = Color(0.22, 0.18, 0.15)
		_root_control.add_child(backdrop)

	var margin := MarginContainer.new()
	margin.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	for side in ["margin_left", "margin_right"]:
		margin.add_theme_constant_override(side, 56)
	margin.add_theme_constant_override("margin_top", 36)
	margin.add_theme_constant_override("margin_bottom", 32)
	_root_control.add_child(margin)
	var column := VBoxContainer.new()
	column.alignment = BoxContainer.ALIGNMENT_CENTER
	column.add_theme_constant_override("separation", 14)
	margin.add_child(column)

	var kicker := {"battle": "Bataille rangée", "siege": "Siège", "naval": "Bataille sur mer", "campaign": "Campagne"}
	var head := FrontEndStyle.label(str(kicker.get(context, "")).to_upper(), 16, FrontEndStyle.GOLD, FrontEndStyle.title_font(), 3)
	head.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	column.add_child(head)
	var title := FrontEndStyle.label(str(screen.get("title", "")), 40, Color(0.97, 0.92, 0.80), FrontEndStyle.title_font(), 6)
	title.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	column.add_child(title)

	var frame := PanelContainer.new()
	# Cadre du kit enluminé UI1 (vélin, filets d'or et vermillon, bossettes) : même style que
	# les fenêtres du jeu.
	frame.add_theme_stylebox_override("panel", HudStyle.panel_box(14))
	frame.size_flags_horizontal = Control.SIZE_SHRINK_CENTER
	frame.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	column.add_child(frame)
	var art := AspectRatioContainer.new()
	art.ratio = 16.0 / 9.0
	art.custom_minimum_size = _art_size()
	frame.add_child(art)
	var picture := TextureRect.new()
	picture.texture = art_texture
	picture.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	picture.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_COVERED
	art.add_child(picture)

	var quote: Dictionary = screen.get("quote", {})
	if not quote.is_empty():
		var quote_label := FrontEndStyle.label("« %s »" % quote.get("text", ""), 24, Color(0.95, 0.90, 0.78), FrontEndStyle.title_italic(), 4)
		quote_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		quote_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
		quote_label.custom_minimum_size = Vector2(minf(_art_size().x, 1000.0), 0)
		quote_label.size_flags_horizontal = Control.SIZE_SHRINK_CENTER
		column.add_child(quote_label)
		var attribution := FrontEndStyle.label("— %s, %s (%s)" % [quote.get("author", ""), quote.get("source", ""), quote.get("date", "")], 17, FrontEndStyle.GOLD, FrontEndStyle.body_font(), 3)
		attribution.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
		attribution.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		column.add_child(attribution)
	var hint := FrontEndStyle.label("Les armées prennent position…", 16, Color(0.80, 0.74, 0.62), FrontEndStyle.body_italic(), 3)
	hint.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	column.add_child(hint)


## Taille de l'enluminure : au plus ART_MAX, et au plus 58 % de la hauteur de la fenêtre.
func _art_size() -> Vector2:
	var viewport_size := Vector2(1600, 900)
	if is_inside_tree():
		viewport_size = get_viewport().get_visible_rect().size
	var height := minf(ART_MAX.y, viewport_size.y * 0.58)
	return Vector2(height * 16.0 / 9.0, height)


func _maybe_screenshot() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--battle-loading-shot="):
			var path := arg.trim_prefix("--battle-loading-shot=")
			await RenderingServer.frame_post_draw
			var image := get_viewport().get_texture().get_image()
			DirAccess.make_dir_recursive_absolute(path.get_base_dir())
			var err := image.save_png(path)
			print("BattleLoadingCard: screenshot %s (%s)" % [path, error_string(err)])
			get_tree().quit(0 if err == OK else 1)
			await get_tree().process_frame
