class_name ReliefCacheNotice
extends PanelContainer

## Avis non bloquant « relief rapproché limité » (lot ZG7b, ADR 0036) : montré une seule fois par
## session quand le cache du relief fin manque ou est incomplet (`ReliefCacheStatus`), en haut de
## l'écran de campagne, sous la barre supérieure. Il ne capte que les clics sur lui-même, donne la
## commande de régénération (sélectionnable, bouton « Copier ») et se ferme d'un clic.
## Rendu seulement.
## PO1 (bible DA § 12.5) : texte d'outil (commande `uv run`…) — l'avis ne s'affiche qu'en mode
## développeur (`Settings.is_dev()`, `-- --dev`) ; sinon le cache incomplet ne donne que les
## lignes `push_warning` du journal Godot.

const THEME_PATH := "res://scenes/ui/parchment_theme.tres"
const WIDTH := 560.0
const TOP := 58.0

## Une seule fois par session (toutes les cartes de campagne ouvertes depuis le lancement).
static var shown_this_session := false

var status: ReliefCacheStatus
var command_field: LineEdit
var close_button: Button


## Contrôle le cache de `map_dir`, écrit le résultat dans le journal et, s'il manque tout ou
## partie, ajoute l'avis à `parent` (couche UI de la carte). Renvoie l'état contrôlé (null si
## la pyramide est désactivée par `--no-pyramid` ou remplacée par `--pyramid-dir=`).
static func report(parent: Node, map_dir: String, relief_root: String) -> ReliefCacheStatus:
	for arg in OS.get_cmdline_user_args():
		if arg == "--no-pyramid" or arg.begins_with("--pyramid-dir="):
			return null
	var checked := ReliefCacheStatus.check(map_dir, relief_root)
	if checked.needs_notice():
		push_warning(checked.summary())
		push_warning("ReliefCache: close zoom limited; regenerate with `%s` (docs/geo.md)" % ReliefCacheStatus.REGEN_COMMAND)
	else:
		print(checked.summary())
	if checked.needs_notice() and not shown_this_session and parent != null and dev_mode():
		shown_this_session = true
		var notice := ReliefCacheNotice.new()
		notice.name = "ReliefCacheNotice"
		notice.setup(checked)
		parent.add_child(notice)
	return checked


## PO1 : vrai en mode développeur (autoload `Settings` absent : faux).
static func dev_mode() -> bool:
	var tree := Engine.get_main_loop() as SceneTree
	var settings: Node = tree.root.get_node_or_null("Settings") if tree != null else null
	return settings != null and bool(settings.call("is_dev"))


func setup(checked: ReliefCacheStatus) -> void:
	status = checked
	var exported := OS.has_feature("template")
	if ResourceLoader.exists(THEME_PATH):
		theme = load(THEME_PATH)
	mouse_filter = Control.MOUSE_FILTER_STOP
	anchor_left = 0.5
	anchor_right = 0.5
	offset_left = -WIDTH * 0.5
	offset_right = WIDTH * 0.5
	offset_top = TOP
	grow_horizontal = Control.GROW_DIRECTION_BOTH
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 6)
	add_child(box)
	var head := HBoxContainer.new()
	box.add_child(head)
	var title := Label.new()
	title.text = status.notice_title()
	title.add_theme_font_size_override("font_size", UiType.size(UiType.BODY))
	title.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	head.add_child(title)
	close_button = Button.new()
	close_button.text = "Fermer"
	close_button.tooltip_text = "Masquer cet avis jusqu'au prochain lancement du jeu"
	close_button.pressed.connect(dismiss)
	head.add_child(close_button)
	var body := Label.new()
	body.text = status.notice_text(exported)
	body.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	body.custom_minimum_size = Vector2(WIDTH - 32.0, 0.0)
	box.add_child(body)
	if exported:
		return
	# SZ7 (ADR 0077) : `relief-fetch` (paquet déjà cuit, plus rapide) proposé en premier ;
	# `relief-all` (recalcul, plusieurs heures) reste la solution de repli sans hébergement.
	var row := HBoxContainer.new()
	box.add_child(row)
	command_field = LineEdit.new()
	command_field.text = ReliefCacheStatus.FETCH_COMMAND
	command_field.editable = false
	command_field.selecting_enabled = true
	command_field.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	command_field.tooltip_text = "À lancer à la racine du dépôt. État détaillé : %s" % ReliefCacheStatus.CHECK_COMMAND
	# ZG7c : un champ non modifiable prend la couleur « désactivée » du thème (gris sur parchemin,
	# illisible) ; la commande est à lire et à copier : encre normale du champ.
	command_field.ready.connect(func() -> void:
		command_field.add_theme_color_override("font_uneditable_color", command_field.get_theme_color("font_color")))
	row.add_child(command_field)
	var copy := Button.new()
	copy.text = "Copier"
	copy.tooltip_text = "Copier la commande dans le presse-papiers"
	copy.pressed.connect(func() -> void: DisplayServer.clipboard_set(ReliefCacheStatus.FETCH_COMMAND))
	row.add_child(copy)
	var fallback := Label.new()
	fallback.text = "Sans hébergement disponible : %s" % ReliefCacheStatus.REGEN_COMMAND
	fallback.add_theme_font_size_override("font_size", UiType.size(UiType.CAPTION))
	fallback.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	fallback.custom_minimum_size = Vector2(WIDTH - 32.0, 0.0)
	box.add_child(fallback)


func dismiss() -> void:
	hide()
	queue_free()
