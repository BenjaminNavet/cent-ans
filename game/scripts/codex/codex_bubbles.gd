extends CanvasLayer

## Autoload `CodexBubbles` (H2) : bulles imbriquées au-dessus de toute
## l'interface. Une UI branche ses textes par `attach(label)` (un `RichTextLabel` dont le BBCode
## vient de `CodexText.format`) :
##  - survol d'un mot-lien ≥ 0,35 s → bulle fille près de la souris (titre, catégorie, résumé) ;
##  - la souris peut entrer dans la bulle ; ses liens ouvrent la bulle suivante (pile de 6 au
##    plus, la plus ancienne se ferme au-delà) ;
##  - quitter toutes les bulles → fermeture après 0,4 s de grâce ; Échap ferme tout ;
##  - clic gauche sur un lien ou sur une bulle → fiche complète dans la fenêtre Codex ;
##  - clic droit sur un lien → bulle épinglée ; clic droit sur une bulle → épingle / détache ;
##  - touche T (`codex_pin_tooltip`, B1) : verrouille, par priorité, la bulle non épinglée la plus
##    récente (ou celle en attente de survol), l'infobulle riche visible (F2), ou l'infobulle simple
##    du contrôle survolé, convertie en bulle épinglée avec auto-liens. L'événement n'est consommé
##    que si quelque chose a été verrouillé (T ouvre aussi l'arbre des techniques).
## Chaque bulle retient sa bulle parente (méta `parent`) : épingler une bulle épingle ses
## ancêtres, détacher une bulle détache ses descendantes, de sorte que la chaîne reste entière.
## Chaque bulle ouverte marque sa fiche découverte. La fenêtre Codex (touche K, `codex_open`)
## vit sur un calque juste en dessous. Présentation pure : aucune règle de jeu.
##
## IB3 (ADR 0109, spec IB § 3.2) : chaîne à **Alt maintenu** (`tooltip_explore`, Alt seul — Alt+Maj
## des formations est ignoré) :
##  - Alt enfoncé au-dessus d'une infobulle (native visible ou contrôle survolé qui en a une), d'un
##    mot-lien en attente ou d'une bulle non épinglée → bulle verrouillée « par la chaîne », sur place ;
##  - Alt maintenu : survol d'un mot-lien d'une bulle → fille après `chain.hover_delay_s`, déjà
##    verrouillée ; un autre mot de la même bulle remplace la branche issue du précédent ; le mot
##    source reste surligné dans la parente tant que sa fille est ouverte ;
##  - Alt relâché : hors de toute bulle, les bulles verrouillées par la chaîne se ferment après
##    `chain.close_grace_s` (celles épinglées au clic droit ou par T restent).
## Délais, grâce et taille de la pile : bloc `chain` de `data/ui/tooltip_style.json` (constantes
## ci-dessous en repli). Bulle verrouillée : version détaillée `TooltipView.build(spec, true)` quand
## `RichTooltip.spec_for` existe (IB1), BBCode sinon.

signal bubble_opened(id: String)
signal entry_requested(id: String)

const LAYER := 110
const WIDTH := 340.0
const HOVER_DELAY := 0.35
const CLOSE_GRACE := 0.4
const MAX_BUBBLES := 6
const MOUSE_OFFSET := Vector2(14, 18)
const MARGIN := 8.0
const INK := Color(0.22, 0.14, 0.07)
const MUTED := "#6b5a40"
## IB3 : réglages de la chaîne (`chain` de `data/ui/tooltip_style.json`), lus via `MapPaths`.
const STYLE_FILE := "ui/tooltip_style.json"
const MAP_PATHS_SCRIPT := preload("res://scripts/map/map_paths.gd")
const CHAIN_FALLBACK := {
	"hover_delay_s": 0.12, "idle_hover_delay_s": HOVER_DELAY, "close_grace_s": CLOSE_GRACE,
	"max_bubbles": MAX_BUBBLES,
}
## Fond du mot-lien source d'une fille ouverte par la chaîne.
const SOURCE_HIGHLIGHT := "#e8cf8a"

static var _chain_cache: Dictionary = {}

## Pile des bulles ouvertes (de la plus ancienne à la plus récente).
var bubbles: Array[PanelContainer] = []
var _pending_id := ""
var _pending_source: Control = null
var _pending_time := 0.0
var _hover_id := ""
var _hover_source: Control = null
var _outside_time := 0.0
var _window: Control
## Contrôle survolé et durée du survol (T n'épingle une infobulle simple qu'une fois affichée).
var _hovered_control: Control = null
var _hovered_time := 0.0
## IB3 : Alt (`tooltip_explore`) maintenu seul.
var _explore_held := false
## Étiquettes de bulles dont le surlignage de mot source est à recalculer (hors survol d'un lien).
var _highlight_dirty: Array[RichTextLabel] = []


func _ready() -> void:
	layer = LAYER
	process_mode = Node.PROCESS_MODE_ALWAYS


## Branche un `RichTextLabel` (BBCode issu de `CodexText.format`) sur les bulles.
func attach(label: RichTextLabel) -> void:
	if label == null or label.has_meta("codex_attached"):
		return
	label.set_meta("codex_attached", true)
	label.meta_underlined = false  # le soulignement marque seulement les fiches non lues
	label.meta_hover_started.connect(_on_meta_hover_started.bind(label))
	label.meta_hover_ended.connect(_on_meta_hover_ended.bind(label))
	label.meta_clicked.connect(_on_meta_clicked)


## Ouvre la bulle de la fiche `id` à `at` (position de la souris si négative), au-dessus de la
## bulle d'index `parent_index` (-1 : nouvelle pile ; les bulles non épinglées au-dessus se
## ferment). `chain` (IB3) : bulle verrouillée par la chaîne à Alt ; les bulles verrouillées par
## la chaîne au-dessus de la parente sont aussi remplacées. Renvoie la bulle, ou null si la fiche
## n'existe pas.
func open(id: String, at: Vector2 = Vector2(-1, -1), parent_index: int = -1, pinned: bool = false, chain: bool = false) -> PanelContainer:
	var codex := CodexText.store()
	if codex == null or not bool(codex.call("has_entry", id)):
		return null
	var parent: PanelContainer = bubbles[parent_index] if parent_index >= 0 and parent_index < bubbles.size() else null
	var existing := _child_of(parent, id)
	if existing == null or not chain:
		_trim_above(parent_index, chain)
	if existing != null and is_instance_valid(existing) and bubbles.has(existing):
		if pinned:
			set_pinned(existing, true)
		elif chain:
			chain_lock(existing)
		return existing
	var bubble := _make_bubble(id, _entry_bbcode(id), false, parent)
	_push(bubble, at)
	if pinned:
		set_pinned(bubble, true)
	elif chain:
		chain_lock(bubble)
	codex.call("discover", id)
	bubble_opened.emit(id)
	return bubble


## Bulle au texte libre (infobulle épinglée), toujours épinglée ; `entry_id` : fiche ouverte par
## un clic sur la bulle (titre lié d'une infobulle riche), vide sinon.
func open_text(bbcode: String, at: Vector2 = Vector2(-1, -1), entry_id: String = "") -> PanelContainer:
	var bubble := _make_bubble(entry_id, bbcode, true)
	bubble.set_meta("free_text", true)
	_push(bubble, at)
	return bubble


## IB3 : bulle épinglée au contenu déjà construit (version détaillée `TooltipView.build(spec,
## true)`) ; ses `RichTextLabel` sont branchés sur les bulles.
func open_view(view: Control, at: Vector2 = Vector2(-1, -1), entry_id: String = "") -> PanelContainer:
	var bubble := _make_bubble(entry_id, "", true, null, view)
	bubble.set_meta("free_text", true)
	_push(bubble, at)
	return bubble


## IB3 : Alt (`tooltip_explore`) enfoncé seul. Verrouille par la chaîne, par priorité : la fille
## en attente de survol (ouverte aussitôt), la bulle non épinglée sous la souris, l'infobulle
## native visible, l'infobulle du contrôle survolé. True si quelque chose a été verrouillé.
func explore_lock() -> bool:
	_drop_freed_sources()
	if _pending_id != "":
		var id := _pending_id
		var source := _pending_source
		_pending_id = ""
		_pending_source = null
		var child := open(id, -Vector2.ONE, _index_of_source(source), false, true)
		if child != null:
			_mark_source(child, source, id)
			return true
	var under := _bubble_under_mouse()
	if under != null:
		if not under.get_meta("pinned", false):
			chain_lock(under)
			return true
		return false
	var count := bubbles.size()
	if pin_native_tooltip() or pin_hovered_tooltip():
		if bubbles.size() > count:
			_apply_pinned(bubbles[-1], true, true)
		return true
	return false


## IB3 : verrouille `bubble` (et ses ancêtres non épinglées) « par la chaîne » : elles se ferment
## après la grâce une fois Alt relâché et la souris hors des bulles.
func chain_lock(bubble: PanelContainer) -> void:
	var ancestor := parent_of(bubble)
	if ancestor != null and not ancestor.get_meta("pinned", false):
		chain_lock(ancestor)
	if not bubble.get_meta("pinned", false) or bubble.get_meta("chain_locked", false):
		_apply_pinned(bubble, true, true)


## IB3 : true tant qu'Alt (`tooltip_explore`) est maintenu seul.
func explore_held() -> bool:
	return _explore_held


## IB3 : réglage `key` du bloc `chain` de `data/ui/tooltip_style.json` (repli : constantes).
static func chain_setting(key: String) -> float:
	if _chain_cache.is_empty():
		_chain_cache = CHAIN_FALLBACK.duplicate()
		var path := _data_dir().path_join(STYLE_FILE)
		if not FileAccess.file_exists(path):
			path = MAP_PATHS_SCRIPT.project_root().path_join("data").path_join(STYLE_FILE)
		var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path)) if FileAccess.file_exists(path) else null
		if parsed is Dictionary and (parsed as Dictionary).get("chain") is Dictionary:
			_chain_cache.merge(parsed["chain"], true)
		else:
			push_warning("CodexBubbles : %s illisible, réglages de chaîne de repli." % STYLE_FILE)
	return float(_chain_cache.get(key, CHAIN_FALLBACK.get(key, 0.0)))


## Relit `tooltip_style.json` au prochain accès (tests).
static func reload_chain_settings() -> void:
	_chain_cache = {}


static func _data_dir() -> String:
	var tree := Engine.get_main_loop() as SceneTree
	if tree != null and tree.root != null:
		var map_paths := tree.root.get_node_or_null("MapPaths")
		if map_paths != null:
			return str(map_paths.get("data_dir"))
	return MAP_PATHS_SCRIPT.project_root().path_join("data")


## Touche T : verrouille la bulle ou l'infobulle visible la plus récente. Renvoie true si
## quelque chose a été verrouillé (l'appelant ne consomme l'événement que dans ce cas).
func pin_current() -> bool:
	if pin_top_bubble() or pin_native_tooltip():
		return true
	if _hovered_control == null or _hovered_time < _tooltip_delay():
		return false
	return pin_hovered_tooltip()


## Épingle la bulle en attente de survol (ouverte aussitôt) ou la bulle non épinglée la plus récente.
func pin_top_bubble() -> bool:
	_drop_freed_sources()
	if _pending_id != "":
		var id := _pending_id
		var source := _pending_source
		_pending_id = ""
		_pending_source = null
		if open(id, -Vector2.ONE, _index_of_source(source), true) != null:
			return true
	for index in range(bubbles.size() - 1, -1, -1):
		if not bubbles[index].get_meta("pinned", false) or bubbles[index].get_meta("chain_locked", false):
			set_pinned(bubbles[index], true)
			return true
	return false


## Épingle l'infobulle riche affichée (F2) : bulle interactive au même endroit.
func pin_native_tooltip() -> bool:
	var panel := RichTooltip.visible_panel()
	if panel == null:
		return false
	var window := panel.get_window()
	var at := Vector2(window.position)
	if not window.is_embedded():
		at -= Vector2(get_tree().root.position)
	window.hide()
	var entry := RichTooltip.title_entry(RichTooltip.last_bbcode)
	var hovered := get_viewport().gui_get_hovered_control()
	var view := _detailed_view(_tooltip_at(hovered, hovered.get_local_mouse_position()) if hovered != null else "")
	if view != null:
		open_view(view, at, entry)
	else:
		open_text(RichTooltip.last_bbcode, at, entry)
	return true


## Épingle l'infobulle simple du contrôle survolé (texte converti avec auto-liens).
func pin_hovered_tooltip() -> bool:
	var control := get_viewport().gui_get_hovered_control()
	if control == null:
		return false
	return pin_control_tooltip(control, control.get_local_mouse_position())


## Bulle épinglée tirée de l'infobulle de `control` au point `local_pos` (remonte aux parents
## comme Godot tant que le contrôle laisse passer la souris). False si aucune infobulle.
func pin_control_tooltip(control: Control, local_pos: Vector2 = Vector2.ZERO) -> bool:
	if control == null or _index_of_source(control) >= 0 or control is PanelContainer and bubbles.has(control as PanelContainer):
		return false
	var text := _tooltip_at(control, local_pos)
	if text.strip_edges() == "":
		return false
	var at := _hide_native_tooltips()
	var formatted := CodexText.format(text, true)
	var view := _detailed_view(text)
	if view != null:
		open_view(view, at, RichTooltip.title_entry(formatted))
	else:
		open_text(formatted, at, RichTooltip.title_entry(formatted))
	return true


## Texte d'infobulle de `control` au point `local_pos`, en remontant aux parents comme Godot tant
## que le contrôle laisse passer la souris.
func _tooltip_at(control: Control, local_pos: Vector2) -> String:
	var text := ""
	var current := control
	var pos := local_pos
	while current != null:
		text = current.get_tooltip(pos)
		if text != "" or current.mouse_filter == Control.MOUSE_FILTER_STOP or current.top_level:
			break
		pos = current.get_transform() * pos
		current = current.get_parent() as Control
	return text


## IB3 : version détaillée d'une infobulle (`TooltipView.build(RichTooltip.spec_for(text),
## true)`) quand IB1 fournit `RichTooltip.spec_for` ; null sinon (repli BBCode).
func _detailed_view(text: String) -> Control:
	if text.strip_edges() == "":
		return null
	var rich: Script = RichTooltip
	var view_script: Script = TooltipView
	if not _script_has(rich, "spec_for") or not _script_has(view_script, "build"):
		return null
	var spec: Variant = rich.call("spec_for", text)
	if not spec is Dictionary or (spec as Dictionary).is_empty():
		return null
	var view: Variant = view_script.call("build", spec, true)
	return view as Control if view is Control else null


static func _script_has(script: Script, method: String) -> bool:
	if script == null:
		return false
	for info: Dictionary in script.get_script_method_list():
		if str(info.get("name", "")) == method:
			return true
	return false


## Bulle parente de `bubble` (null pour une racine).
func parent_of(bubble: PanelContainer) -> PanelContainer:
	var parent: Variant = bubble.get_meta("parent") if bubble.has_meta("parent") else null
	if parent is PanelContainer and is_instance_valid(parent) and bubbles.has(parent):
		return parent
	return null


func close_all() -> void:
	for bubble in bubbles:
		bubble.queue_free()
	bubbles.clear()
	_pending_id = ""
	_outside_time = 0.0


func close_unpinned() -> void:
	for bubble in bubbles.duplicate():
		if not bubble.get_meta("pinned", false):
			_remove(bubble)


func bubble_count() -> int:
	return bubbles.size()


## Id de la fiche de la bulle au sommet de la pile (vide si aucune ou texte libre).
func top_id() -> String:
	return str(bubbles[-1].get_meta("codex_id", "")) if not bubbles.is_empty() else ""


## Épingle (et ses ancêtres, pour garder la chaîne) ou détache (et ses descendantes) `bubble`.
func set_pinned(bubble: PanelContainer, pinned: bool) -> void:
	if pinned:
		var ancestor := parent_of(bubble)
		if ancestor != null and (not ancestor.get_meta("pinned", false) or ancestor.get_meta("chain_locked", false)):
			set_pinned(ancestor, true)
	else:
		for child in bubbles.duplicate():
			if parent_of(child) == bubble and child.get_meta("pinned", false):
				set_pinned(child, false)
	_apply_pinned(bubble, pinned)


## `chain` (IB3) : verrouillée par la chaîne à Alt (fermée à la grâce, Alt relâché), et non
## épinglée durablement (clic droit, T).
func _apply_pinned(bubble: PanelContainer, pinned: bool, chain: bool = false) -> void:
	bubble.set_meta("pinned", pinned)
	bubble.set_meta("chain_locked", pinned and chain)
	# Épinglée : page à bande d'or (lot UI1) plutôt que simple note marginale.
	var style: StyleBox = HudStyle.panel_box(10) if pinned else RichTooltip.panel_style()
	bubble.add_theme_stylebox_override("panel", style)
	var footer := bubble.find_child("Footer", true, false) as Label
	if footer != null:
		footer.text = _footer_text(str(bubble.get_meta("codex_id", "")), pinned, pinned and chain)


# --- Fenêtre Codex ---------------------------------------------------------------------------


## Fenêtre Codex (créée à la demande).
func window() -> Control:
	# U11 : sur la carte, la fenêtre commune « Codex » (onglet Histoire) remplace la fenêtre seule.
	var hub := get_tree().get_first_node_in_group("codex_hub") if is_inside_tree() else null
	if hub != null and hub.get("codex_window") != null:
		return hub.get("codex_window")
	if _window == null:
		_window = CodexWindow.new()
		_window.name = "CodexWindow"
		_window.hide()
		# P2c : fenêtre seule (hors `CodexHub`, ex. en bataille) dans `UiZones.Zone.MODAL` — fond
		# assombri, centrée sur sa taille propre, sous les bulles (étage `UiZones.DEFAULT_LAYER`
		# < `LAYER`).
		UiZones.put(UiZones.Zone.MODAL, _window)
	return _window


func is_window_open() -> bool:
	var current := window()
	return current != null and current.visible


## Ouvre la fenêtre Codex sur `id` (liste seule si vide) ; les bulles non épinglées se ferment.
func open_entry(id: String = "") -> void:
	close_unpinned()
	window().call("open", id)
	if id != "":
		entry_requested.emit(id)


func toggle_window() -> void:
	if is_window_open():
		window().hide()
	else:
		open_entry()


# --- Entrées ---------------------------------------------------------------------------------


func _input(event: InputEvent) -> void:
	if event is InputEventKey and event.is_action("tooltip_explore"):
		_on_explore_key(event as InputEventKey)
		return
	if event is InputEventKey and event.pressed and _explore_held and (event as InputEventKey).shift_pressed:
		_explore_held = false  # Alt+Maj+1…6 : formations (`battle_formation_picker.gd`), pas la chaîne
	if event.is_action_pressed("codex_pin_tooltip") and pin_current():
		get_viewport().set_input_as_handled()
	elif event.is_action_pressed("ui_cancel"):
		if not bubbles.is_empty():
			close_all()
			get_viewport().set_input_as_handled()
		elif is_window_open():
			window().hide()
			get_viewport().set_input_as_handled()
	elif event is InputEventMouseButton and event.pressed:
		_on_mouse_pressed(event as InputEventMouseButton)


## IB3 : Alt seul enfoncé → chaîne active et verrouillage de l'infobulle survolée ; relâché →
## la grâce des bulles verrouillées par la chaîne commence hors des bulles. Alt avec Maj, Ctrl ou
## Cmd (formations Alt+Maj+1…6) : ignoré.
func _on_explore_key(event: InputEventKey) -> void:
	if not event.pressed:
		_explore_held = false
		_outside_time = 0.0
		return
	if event.echo:
		return
	if event.shift_pressed or event.ctrl_pressed or event.meta_pressed:
		_explore_held = false
		return
	_explore_held = true
	if explore_lock():
		get_viewport().set_input_as_handled()


func _notification(what: int) -> void:
	if what == NOTIFICATION_APPLICATION_FOCUS_OUT or what == NOTIFICATION_WM_WINDOW_FOCUS_OUT:
		_explore_held = false  # Alt relâché hors de la fenêtre : pas d'événement de relâche


func _unhandled_input(event: InputEvent) -> void:
	if event.is_action_pressed("codex_open"):
		toggle_window()
		get_viewport().set_input_as_handled()


func _on_mouse_pressed(event: InputEventMouseButton) -> void:
	_drop_freed_sources()
	var under := _bubble_under_mouse()
	if event.button_index == MOUSE_BUTTON_RIGHT:
		if _hover_id != "":
			open(_hover_id, -Vector2.ONE, _index_of_source(_hover_source), true)
			get_viewport().set_input_as_handled()
		elif under != null:
			# IB3 : une bulle verrouillée par la chaîne devient épinglée durablement.
			set_pinned(under, not under.get_meta("pinned", false) or under.get_meta("chain_locked", false))
			get_viewport().set_input_as_handled()
	elif event.button_index == MOUSE_BUTTON_LEFT and under != null and _hover_id == "":
		var id := str(under.get_meta("codex_id", ""))
		if id != "":
			open_entry(id)
			get_viewport().set_input_as_handled()


func _on_meta_hover_started(meta: Variant, label: RichTextLabel) -> void:
	var id := CodexText.meta_id(meta)
	if id == "":
		return
	_hover_id = id
	_hover_source = label
	var parent := _index_of_source(label)
	if _child_of(bubbles[parent] if parent >= 0 else null, id) != null:
		_pending_id = ""
		return
	_pending_id = id
	_pending_source = label
	_pending_time = 0.0


func _on_meta_hover_ended(_meta: Variant, label: RichTextLabel) -> void:
	if label == _hover_source:
		_hover_id = ""
		_hover_source = null
	if label == _pending_source:
		_pending_id = ""
		_pending_source = null


func _on_meta_clicked(meta: Variant) -> void:
	var id := CodexText.meta_id(meta)
	if id != "":
		open_entry(id)


## Oublie les sources de survol libérées (panneau fermé sans `meta_hover_ended`) : sinon
## `_hover_id` resterait posé (bulles jamais refermées) et le clic droit lirait un objet libéré.
func _drop_freed_sources() -> void:
	if _hover_source != null and not is_instance_valid(_hover_source):
		_hover_id = ""
		_hover_source = null
	if _pending_source != null and not is_instance_valid(_pending_source):
		_pending_id = ""
		_pending_source = null


func _process(delta: float) -> void:
	_drop_freed_sources()
	var hovered := get_viewport().gui_get_hovered_control()
	if hovered != _hovered_control:
		_hovered_control = hovered
		_hovered_time = 0.0
	else:
		_hovered_time += delta
	if _pending_id != "":
		_pending_time += delta
		var chain := _explore_held and _index_of_source(_pending_source) >= 0
		if _pending_time >= chain_setting("hover_delay_s" if chain else "idle_hover_delay_s"):
			var id := _pending_id
			var source := _pending_source
			_pending_id = ""
			var child := open(id, -Vector2.ONE, _index_of_source(source), false, chain)
			if chain and child != null:
				_mark_source(child, source, id)
	_refresh_highlights()
	var has_closable := false
	for bubble in bubbles:
		if _closable(bubble):
			has_closable = true
			break
	if not has_closable:
		_outside_time = 0.0
		return
	if _hover_id != "" or _bubble_under_mouse() != null:
		_outside_time = 0.0
	else:
		_outside_time += delta
		if _outside_time >= chain_setting("close_grace_s"):
			_outside_time = 0.0
			for bubble in bubbles.duplicate():
				if is_instance_valid(bubble) and bubbles.has(bubble) and _closable(bubble):
					_remove(bubble)


## Bulle refermée à la grâce : non épinglée, ou verrouillée par la chaîne une fois Alt relâché.
func _closable(bubble: PanelContainer) -> bool:
	if not bubble.get_meta("pinned", false):
		return true
	return bubble.get_meta("chain_locked", false) and not _explore_held


# --- Surlignage du mot source (IB3) ----------------------------------------------------------


## Retient sur `child` le mot-lien source (`id` dans l'étiquette `source` d'une bulle parente) ;
## il reste surligné tant que `child` est ouverte.
func _mark_source(child: PanelContainer, source: Control, id: String) -> void:
	var label := source as RichTextLabel
	if label == null or not is_instance_valid(label) or _index_of_source(label) < 0:
		return
	child.set_meta("source_label", label)
	child.set_meta("source_meta", CodexText.META_PREFIX + id)
	_queue_highlight(label)


func _queue_highlight(label: RichTextLabel) -> void:
	if label != null and is_instance_valid(label) and not _highlight_dirty.has(label):
		_highlight_dirty.append(label)


## Réécrit le BBCode des étiquettes à jour de surlignage. Attendue tant qu'un lien de l'étiquette
## est survolé (réécrire le texte sous un lien survolé fausserait `meta_hover_*`).
func _refresh_highlights() -> void:
	for label in _highlight_dirty.duplicate():
		if not is_instance_valid(label):
			_highlight_dirty.erase(label)
			continue
		if label == _hover_source:
			continue
		_highlight_dirty.erase(label)
		apply_source_highlight(label)


## Surligne dans `label` les mots-liens dont une bulle fille est ouverte (IB3).
func apply_source_highlight(label: RichTextLabel) -> void:
	var base := str(label.get_meta("base_text", label.text))
	var metas := PackedStringArray()
	for bubble in bubbles:
		if is_instance_valid(bubble) and not bubble.is_queued_for_deletion() and bubble.has_meta("source_label") and bubble.get_meta("source_label") == label:
			metas.append(str(bubble.get_meta("source_meta", "")))
	var text := base
	for meta in metas:
		text = highlight_links(text, meta, SOURCE_HIGHLIGHT)
	if metas.is_empty():
		label.remove_meta("base_text")
	else:
		label.set_meta("base_text", base)
	if label.text != text:
		label.text = text


## `bbcode` avec un fond `color` sous chaque lien `[url=meta]…[/url]`.
static func highlight_links(bbcode: String, meta: String, color: String) -> String:
	var open_tag := "[url=%s]" % meta
	var result := ""
	var from := 0
	while true:
		var start := bbcode.find(open_tag, from)
		if start < 0:
			break
		var inner := start + open_tag.length()
		var end := bbcode.find("[/url]", inner)
		if end < 0:
			break
		result += bbcode.substr(from, inner - from) + "[bgcolor=%s]" % color + bbcode.substr(inner, end - inner) + "[/bgcolor]"
		from = end
	return result + bbcode.substr(from)


# --- Construction et placement ---------------------------------------------------------------


func _entry_bbcode(id: String) -> String:
	var codex := CodexText.store()
	var head := "[b]%s[/b]" % str(codex.call("title", id))
	var meta := PackedStringArray([str(codex.call("category_label", id))])
	var era := str(codex.call("era_label", id))
	if era != "":
		meta.append(era)
	head += "  [color=%s][i]%s[/i][/color]" % [MUTED, " · ".join(meta)]
	var entry: Dictionary = codex.call("entry", id)
	var text := "%s\n%s" % [head, CodexText.format(str(entry.get("summary", "")))]
	var gameplay := first_sentence(str(entry.get("gameplay", "")))
	if gameplay != "":
		text += "\n[i]En jeu :[/i] %s" % CodexText.format(gameplay)
	return text


## Première phrase d'un texte (jusqu'au premier « . », « ! », « ? » ou « … » suivi d'une espace
## ou en fin de texte), hors liens `[[…]]`.
static func first_sentence(text: String) -> String:
	text = text.strip_edges()
	var depth := 0
	for index in text.length():
		var character := text[index]
		if character == "[":
			depth += 1
		elif character == "]":
			depth = maxi(0, depth - 1)
		elif depth == 0 and ".!?…".contains(character) and (index + 1 == text.length() or text[index + 1] == " "):
			return text.substr(0, index + 1)
	return text


func _footer_text(id: String, pinned: bool, chain: bool = false) -> String:
	var parts := PackedStringArray()
	if id != "":
		parts.append("Clic : lire la fiche")
	if chain:
		parts.append("Clic droit : épingler")
	elif pinned:
		parts.append("Clic droit : détacher")
	else:
		parts.append("T : maintenir ouverte")
		parts.append("Alt : explorer")
	return " · ".join(parts)


## `view` (IB3) : contenu déjà construit (version détaillée) à la place du texte BBCode.
func _make_bubble(id: String, bbcode: String, pinned: bool, parent: PanelContainer = null, view: Control = null) -> PanelContainer:
	var bubble := PanelContainer.new()
	bubble.name = "Bubble"
	bubble.theme = load(RichTooltip.THEME_PATH)
	bubble.mouse_filter = Control.MOUSE_FILTER_STOP
	bubble.set_meta("codex_id", id)
	if parent != null:
		bubble.set_meta("parent", parent)
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 4)
	box.mouse_filter = Control.MOUSE_FILTER_PASS
	bubble.add_child(box)
	if view != null:
		_adopt_view(box, view)
	else:
		_add_text_label(box, bbcode)
	var footer := Label.new()
	footer.name = "Footer"
	UiType.apply(footer, UiType.CAPTION)
	footer.add_theme_color_override("font_color", Color(MUTED))
	footer.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
	box.add_child(footer)
	_apply_pinned(bubble, pinned)
	return bubble


## IB3 : place la vue détaillée dans la bulle, sans son cadre ni son pied (la bulle a les siens) ;
## ses textes sont branchés sur les bulles (chaîne depuis leurs mots-liens).
func _adopt_view(box: VBoxContainer, view: Control) -> void:
	if view is PanelContainer:
		view.add_theme_stylebox_override("panel", StyleBoxEmpty.new())
	for node in view.find_children("Footer", "", true, false):
		node.get_parent().remove_child(node)
		node.queue_free()
	view.mouse_filter = Control.MOUSE_FILTER_PASS
	box.add_child(view)
	for node in view.find_children("*", "RichTextLabel", true, false):
		(node as RichTextLabel).mouse_filter = Control.MOUSE_FILTER_PASS
		attach(node as RichTextLabel)


func _add_text_label(box: VBoxContainer, bbcode: String) -> void:
	var label := RichTextLabel.new()
	label.name = "Text"
	label.bbcode_enabled = true
	label.fit_content = true
	label.scroll_active = false
	label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	label.custom_minimum_size = Vector2(WIDTH, 0)
	label.mouse_filter = Control.MOUSE_FILTER_PASS
	label.add_theme_color_override("default_color", INK)
	# P2c : bulle compacte — variation `Caption` (14 px, plancher de la bible § 12.2), même taille
	# pour le gras (les bulles n'ont pas de variation « grasse » dédiée).
	UiType.apply(label, UiType.CAPTION)
	label.add_theme_font_size_override("bold_font_size", UiType.size(UiType.CAPTION))
	label.text = bbcode
	box.add_child(label)
	attach(label)


func _push(bubble: PanelContainer, at: Vector2) -> void:
	if at.x < 0 or at.y < 0:
		at = get_viewport().get_mouse_position() + MOUSE_OFFSET
	bubble.set_meta("anchor", at)
	bubble.position = at
	add_child(bubble)
	bubbles.append(bubble)
	while bubbles.size() > maxi(1, int(chain_setting("max_bubbles"))):
		_remove(bubbles[0])
	_outside_time = 0.0
	bubble.resized.connect(_clamp.bind(bubble))
	_clamp(bubble)


## Garde la bulle à l'écran : à gauche de l'ancre si elle déborde à droite, remontée sinon.
func _clamp(bubble: PanelContainer) -> void:
	if not is_instance_valid(bubble):
		return
	var area := get_viewport().get_visible_rect().size
	var at: Vector2 = bubble.get_meta("anchor", bubble.position)
	var pos := at
	if pos.x + bubble.size.x > area.x - MARGIN:
		pos.x = at.x - bubble.size.x - MOUSE_OFFSET.x * 2.0
	pos.x = clampf(pos.x, MARGIN, maxf(MARGIN, area.x - bubble.size.x - MARGIN))
	pos.y = clampf(pos.y, MARGIN, maxf(MARGIN, area.y - bubble.size.y - MARGIN))
	bubble.position = pos.floor()


## Ferme les bulles non épinglées au-dessus de `parent_index` ; `chain` (IB3) : aussi celles
## verrouillées par la chaîne (remplacement de branche).
func _trim_above(parent_index: int, chain: bool = false) -> void:
	for index in range(bubbles.size() - 1, parent_index, -1):
		if index >= bubbles.size():
			continue
		var bubble := bubbles[index]
		if not bubble.get_meta("pinned", false) or chain and bubble.get_meta("chain_locked", false):
			_remove(bubble)


## Retire `bubble` ; ses filles restantes (épinglées) sont rattachées à sa propre parente.
func _remove(bubble: PanelContainer) -> void:
	var grandparent := parent_of(bubble)
	bubbles.erase(bubble)
	var source: Variant = bubble.get_meta("source_label") if bubble.has_meta("source_label") else null
	if source is RichTextLabel and is_instance_valid(source):
		_queue_highlight(source)
	for child in bubbles:
		if child.has_meta("parent") and child.get_meta("parent") == bubble:
			if grandparent != null:
				child.set_meta("parent", grandparent)
			else:
				child.remove_meta("parent")
	_drop_freed_sources()
	if _hover_source != null and bubble.is_ancestor_of(_hover_source):
		_hover_id = ""
		_hover_source = null
	if _pending_source != null and bubble.is_ancestor_of(_pending_source):
		_pending_id = ""
		_pending_source = null
	bubble.queue_free()


## Bulle fille de `parent` (null : racine) sur la fiche `id`, null si aucune.
func _child_of(parent: PanelContainer, id: String) -> PanelContainer:
	for bubble in bubbles:
		if parent_of(bubble) == parent and not bubble.get_meta("free_text", false) and str(bubble.get_meta("codex_id", "")) == id:
			return bubble
	return null


## Position de l'infobulle native visible (masquée au passage), sinon (-1, -1) : la souris.
func _hide_native_tooltips() -> Vector2:
	var at := -Vector2.ONE
	for node in get_tree().root.find_children("*", "Window", true, false):
		var popup := node as Window
		if popup != null and popup.visible and popup.get_class() == "TooltipPanel":
			at = Vector2(popup.position) if popup.is_embedded() else Vector2(popup.position - get_tree().root.position)
			popup.hide()
	return at


func _tooltip_delay() -> float:
	return float(ProjectSettings.get_setting("gui/timers/tooltip_delay_sec", 0.5))


## Index de la bulle contenant `source`, -1 si `source` est hors des bulles.
func _index_of_source(source: Control) -> int:
	if source == null or not is_instance_valid(source):
		return -1
	for index in bubbles.size():
		if bubbles[index].is_ancestor_of(source):
			return index
	return -1


func _bubble_under_mouse() -> PanelContainer:
	var mouse := get_viewport().get_mouse_position()
	for index in range(bubbles.size() - 1, -1, -1):
		if bubbles[index].get_global_rect().has_point(mouse):
			return bubbles[index]
	return null
