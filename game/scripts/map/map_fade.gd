class_name MapFade
extends RefCounted

## ZF-A : fondus de zoom de la carte de campagne. Remplace les bascules `visible` à seuil par une
## rampe smoothstep sur la distance du rig, appliquée par `GeometryInstance3D.transparency`
## (0 = opaque, 1 = invisible). `visible` ne passe à faux qu'une fois l'objet invisible
## (aucun coût de rendu inutile).

## Au-delà de cette transparence, l'objet est masqué (`visible = false`).
const HIDE_TRANSPARENCY := 0.99
## Bande de fondu par défaut en amont d'une portée : [0,8 × portée, portée].
const RANGE_BAND := 0.2


## Opacité [0, 1] : 1 en deçà de `start`, 0 au-delà de `end`, smoothstep entre les deux.
static func alpha_below(distance: float, start: float, end: float) -> float:
	return 1.0 - smoothstep(start, maxf(end, start + 0.0001), distance)


## Opacité d'une portée `range_end` avec fondu sur les derniers `RANGE_BAND` de la portée.
static func range_alpha(distance: float, range_end: float) -> float:
	return alpha_below(distance, range_end * (1.0 - RANGE_BAND), range_end)


## Applique l'opacité à une instance (transparence + masquage une fois invisible).
static func apply(instance: GeometryInstance3D, alpha: float) -> void:
	var transparency := 1.0 - clampf(alpha, 0.0, 1.0)
	instance.transparency = transparency
	instance.visible = transparency < HIDE_TRANSPARENCY


## Applique l'opacité à toutes les instances géométriques sous `root`.
static func apply_tree(root: Node3D, alpha: float) -> void:
	var transparency := 1.0 - clampf(alpha, 0.0, 1.0)
	root.visible = transparency < HIDE_TRANSPARENCY
	_set_tree(root, transparency)


static func _set_tree(node: Node, transparency: float) -> void:
	if node is GeometryInstance3D:
		(node as GeometryInstance3D).transparency = transparency
	for child in node.get_children():
		_set_tree(child, transparency)
