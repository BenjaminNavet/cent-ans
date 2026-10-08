class_name TooltipHost
extends RefCounted

## Hôte unique des infobulles : toute bulle native (`_make_custom_tooltip`) passe par
## `TooltipHost.bubble` ; il construit le contrôle (sections `TooltipView` ou BBCode), tient la
## dernière bulle (épinglage T de `CodexBubbles`), pose les clés `ib:` sur `tooltip_text` et
## attache le script générique aux contrôles natifs. Le contenu (specs, textes) vit dans
## `RichTooltip`, le rendu en sections dans `TooltipView`. Aucune règle de jeu ici.

## Dernière infobulle construite (épinglage par `CodexBubbles`, touche T) et son BBCode.
static var last_panel: WeakRef = null
static var last_bbcode: String = ""
## IB1 : spec de la dernière infobulle en sections (`TooltipView.build`), {} après `from_bbcode`.
static var last_spec: Dictionary = {}

## IB1 (ADR 0109) : préfixe des clés d'infobulle en sections portées par `tooltip_text`
## (« ib:<kind>:<id> », puis le BBCode de repli sur les lignes suivantes) ; le `live` de la clé
## est rangé en métadonnée `LIVE_META` du contrôle.
const KEY_PREFIX := "ib:"
const LIVE_META := &"ib_live"
## IB2 : script générique attaché par `attach_plain` aux contrôles natifs sans classe dédiée.
const PLAIN_HOST_SCRIPT := preload("res://scripts/ui/plain_tooltip_host.gd")


## Retient la dernière bulle construite (`panel`, son BBCode complet, sa spec ou {}).
static func record(panel: Control, bbcode: String, spec: Dictionary) -> void:
	last_panel = weakref(panel)
	last_bbcode = bbcode
	last_spec = spec


## Style parchemin commun aux infobulles et aux bulles du Codex.
static func panel_style() -> StyleBox:
	return HudStyle.note_box(6)


## Contrôle d'infobulle : panneau parchemin + texte BBCode (largeur fixe, hauteur ajustée).
## Le texte passe par `CodexText.format` (liens `[[…]]` et alias du Codex rubriqués). B1 : pied
## « T : maintenir ouverte » (la touche T verrouille l'infobulle en bulle du Codex).
static func from_bbcode(bbcode: String) -> Control:
	bbcode = fallback_of(bbcode)
	var panel := PanelContainer.new()
	if ResourceLoader.exists(RichTooltip.THEME_PATH):
		panel.theme = load(RichTooltip.THEME_PATH)
	panel.add_theme_stylebox_override("panel", panel_style())
	var box := UiBuild.vbox(4, panel)
	var label := RichTextLabel.new()
	label.bbcode_enabled = true
	label.fit_content = true
	label.scroll_active = false
	label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	label.custom_minimum_size = Vector2(RichTooltip.WIDTH, 0)
	label.add_theme_color_override("default_color", RichTooltip.INK)
	# P2c : infobulle compacte — variation `Caption` (14 px, plancher de la bible § 12.2).
	UiType.apply(label, UiType.CAPTION)
	label.add_theme_font_size_override("bold_font_size", UiType.size(UiType.CAPTION))
	label.text = CodexText.format(bbcode, true)
	label.name = "Text"
	box.add_child(label)
	var footer := UiBuild.label(footer_text(RichTooltip.title_entry(label.text) != ""))
	footer.name = "Footer"
	UiType.apply(footer, UiType.CAPTION)
	footer.add_theme_color_override("font_color", Color(RichTooltip.MUTED))
	footer.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
	box.add_child(footer)
	record(panel, label.text, {})
	return panel


## Pied des infobulles riches : la fiche liée au titre se lit une fois l'infobulle verrouillée.
static func footer_text(has_entry: bool) -> String:
	return "T : maintenir ouverte" + (" · puis clic : lire la fiche" if has_entry else "")


## IB1 : texte de `tooltip_text` d'une infobulle en sections — clé « ib:<kind>:<id> » sur la
## première ligne, BBCode de repli ensuite (lu par `from_bbcode` et par les tests de contenu).
static func tooltip_key(kind: String, id: String, fallback_bbcode: String) -> String:
	return "%s%s:%s\n%s" % [KEY_PREFIX, kind, id, fallback_bbcode]


## Clé « ib:<kind>:<id> » en tête de `text`, "" s'il n'en porte pas.
static func key_of(text: String) -> String:
	return text.get_slice("\n", 0) if text.begins_with(KEY_PREFIX) else ""


## `text` sans sa clé « ib: » éventuelle (le BBCode de repli).
static func fallback_of(text: String) -> String:
	if not text.begins_with(KEY_PREFIX):
		return text
	var cut := text.find("\n")
	return text.substr(cut + 1) if cut >= 0 else ""


## IB1 : pose l'infobulle en sections de `kind`/`id` sur `control` (clé + repli dans
## `tooltip_text`, `live` en métadonnée) ; `_make_custom_tooltip` la reconstruit par `bubble`.
static func set_tooltip(control: Control, kind: String, id: String, live: Dictionary = {}) -> void:
	_ensure_host(control)
	control.set_meta(LIVE_META, live)
	control.tooltip_text = tooltip_key(kind, id, RichTooltip.to_bbcode(RichTooltip.spec_for(KEY_PREFIX + kind + ":" + id, live)))


## IB1 : contrôle d'infobulle pour `_make_custom_tooltip(for_text)` de `owner` : rendu en
## sections (`TooltipView`, version courte) si le texte porte une clé « ib: », sinon `from_bbcode`.
static func bubble(for_text: String, owner: Object = null) -> Control:
	var key := key_of(for_text)
	if key == "":
		return from_bbcode(for_text)
	var live: Dictionary = {}
	if owner != null and owner.has_meta(LIVE_META):
		live = owner.get_meta(LIVE_META)
	var spec := RichTooltip.spec_for(key, live)
	if spec.is_empty():
		return from_bbcode(for_text)
	return TooltipView.build(spec, false)


## Infobulle native actuellement affichée (dans sa fenêtre surgissante), sinon null.
static func visible_panel() -> Control:
	var panel: Control = last_panel.get_ref() if last_panel != null else null
	if panel == null or not panel.is_inside_tree() or not panel.is_visible_in_tree():
		return null
	var window := panel.get_window()
	var loop := Engine.get_main_loop() as SceneTree
	if window == null or (loop != null and window == loop.root) or not window.visible:
		return null
	return panel


## Attache une infobulle brute `ib:plain:<key>` à `control` : pose `tooltip_text` (clé + repli
## BBCode) et, si `control` n'est pas déjà une classe à infobulle riche (`RichButton`, `IconChip`,
## `RichPanel`…, reconnue à son script), lui attache le script générique `plain_tooltip_host.gd`
## qui route `_make_custom_tooltip` vers `bubble`. `live` : `title`/`body`/`hint` dynamiques
## (ex. « Vitesse ×%d » selon la donnée du moment), sinon ceux de `tooltips.json`.
static func attach_plain(control: Control, key: String, live: Dictionary = {}) -> void:
	set_tooltip(control, "plain", key, live)


## Q8 : un contrôle sans `_make_custom_tooltip` montrerait la clé « ib: » et le BBCode bruts :
## contrôle natif → script générique `plain_tooltip_host.gd` ; classe scriptée → erreur (la
## méthode manque à la classe).
static func _ensure_host(control: Control) -> void:
	if control.has_method("_make_custom_tooltip"):
		return
	if control.get_script() == null:
		control.set_script(PLAIN_HOST_SCRIPT)
	else:
		push_error("TooltipHost: %s (%s) lacks _make_custom_tooltip (raw tooltip)" % [control.name, control.get_script().resource_path])




## Bulle d'ardoise du HUD (grappe de fin de tour) : première ligne en rubrique, dernière estompée.
static func hud_lines(for_text: String) -> Control:
	var panel := PanelContainer.new()
	panel.add_theme_stylebox_override("panel", HudStyle.panel_box(8))
	var box := VBoxContainer.new()
	panel.add_child(box)
	var lines := for_text.split("\n")
	for i in lines.size():
		var color := HudStyle.RUBRIC if i == 0 else (HudStyle.INK_FADED if i == lines.size() - 1 else HudStyle.INK)
		box.add_child(HudStyle.label(lines[i], UiType.size(UiType.CAPTION), color))
	return panel
