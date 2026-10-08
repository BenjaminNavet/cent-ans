class_name UiBuild
extends RefCounted

## Fabriques statiques pour construire une interface en code : une ligne remplace la création
## d'un conteneur ou d'un contrôle suivie de ses réglages usuels. Chaque fabrique ajoute le nœud
## à `parent` quand il est fourni ; les valeurs posées sont celles des overrides de thème
## habituels (`separation`, `margin_*`, `font_size`, `font_color`).


## `VBoxContainer` d'espacement `separation`, ajouté à `parent` s'il est fourni.
static func vbox(separation: int, parent: Node = null) -> VBoxContainer:
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", separation)
	_attach(box, parent)
	return box


## `HBoxContainer` d'espacement `separation`, ajouté à `parent` s'il est fourni.
static func hbox(separation: int, parent: Node = null) -> HBoxContainer:
	var box := HBoxContainer.new()
	box.add_theme_constant_override("separation", separation)
	_attach(box, parent)
	return box


## `MarginContainer` avec les quatre marges (gauche, haut, droite, bas).
static func margin(left: int, top: int, right: int, bottom: int, parent: Node = null) -> MarginContainer:
	var container := MarginContainer.new()
	container.add_theme_constant_override("margin_left", left)
	container.add_theme_constant_override("margin_top", top)
	container.add_theme_constant_override("margin_right", right)
	container.add_theme_constant_override("margin_bottom", bottom)
	_attach(container, parent)
	return container


## `Label` de texte `text`. `font_size` > 0 pose la taille de police, `color` (une `Color`) la
## couleur ; `wrap` active le retour à la ligne par mots ; `min_width` > 0 fixe la largeur minimale.
static func label(text: String, font_size: int = 0, color: Variant = null, wrap: bool = false, min_width: float = 0.0, parent: Node = null) -> Label:
	var node := Label.new()
	node.text = text
	if wrap:
		node.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	if min_width > 0.0:
		node.custom_minimum_size = Vector2(min_width, 0)
	if font_size > 0:
		node.add_theme_font_size_override("font_size", font_size)
	if color is Color:
		node.add_theme_color_override("font_color", color)
	_attach(node, parent)
	return node


## `Button` de texte `text`, relié à `on_pressed` si fourni.
static func button(text: String, on_pressed: Callable = Callable(), parent: Node = null) -> Button:
	var node := Button.new()
	node.text = text
	if on_pressed.is_valid():
		node.pressed.connect(on_pressed)
	_attach(node, parent)
	return node


## `Control` vide qui absorbe l'espace libre d'une rangée (`SIZE_EXPAND_FILL` horizontal).
static func spacer(parent: Node = null) -> Control:
	var node := Control.new()
	node.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_attach(node, parent)
	return node


static func _attach(node: Node, parent: Node) -> void:
	if parent != null:
		parent.add_child(node)
