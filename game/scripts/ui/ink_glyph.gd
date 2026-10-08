class_name InkGlyph
extends RefCounted

## DN ui-prod : icônes à l'encre (groupe `glyph` de `data/ui/icons_ink.json`) à la place des
## glyphes Unicode de police (✠ ✔ ⚔ ⚜ ⚑ ⚒ ✉ ★) dans les textes de l'interface. Chaque appel prend un
## glyphe de repli : tant que l'icône n'existe pas (non générée, bibliothèque absente), le texte
## garde son glyphe d'origine.


static func _icons() -> Node:
	var tree := Engine.get_main_loop() as SceneTree
	return tree.root.get_node_or_null("IconLibrary") if tree != null else null


## Vrai si l'icône `id` existe dans la famille à l'encre.
static func has(id: String) -> bool:
	var library := _icons()
	return library != null and bool(library.call("is_ink", id))


## BBCode `[img]` de l'icône `id` (pour un RichTextLabel), sinon `fallback` tel quel.
static func bbcode(id: String, fallback: String, size: int = 16) -> String:
	if not has(id):
		return fallback
	var library := _icons()
	var code := str(library.call("bbcode", id, size))
	return code if code != "" else fallback


## Texture de l'icône `id`, null si absente (l'appelant garde son glyphe).
static func texture(id: String) -> Texture2D:
	if not has(id):
		return null
	return _icons().call("get_icon", id) as Texture2D


## Pose l'icône `id` sur un bouton et retire son texte `fallback` ; sans icône, ne change rien.
static func apply_button(button: Button, id: String, fallback: String, size: int = 16) -> void:
	var tex := texture(id)
	if tex == null:
		button.text = fallback
		return
	button.text = ""
	button.icon = tex
	button.expand_icon = false
	button.add_theme_constant_override("icon_max_width", size)
	_icons().call("apply_state_tints", button)


## Étiquette d'en-tête `glyph + titre` : rangée (icône, titre) si l'icône existe, sinon un Label
## « fallback  titre ». `label` reçoit le style par l'appelant (renvoyé avec la rangée).
static func heading_row(id: String, fallback: String, title: String, size: int = 20) -> Dictionary:
	var label := UiBuild.label(title)
	var tex := texture(id)
	if tex == null:
		label.text = "%s  %s" % [fallback, title]
		return {"node": label, "label": label}
	var row := HBoxContainer.new()
	row.add_theme_constant_override("separation", 6)
	var rect := TextureRect.new()
	rect.texture = tex
	rect.custom_minimum_size = Vector2(size, size)
	rect.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	rect.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
	rect.modulate = _icons().call("tint", HudStyle.RUBRIC)
	row.add_child(rect)
	row.add_child(label)
	return {"node": row, "label": label}
