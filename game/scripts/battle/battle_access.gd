class_name BattleAccess
extends RefCounted

## Mode daltonien en bataille (audit UI/UX § 3, « Accessibilité »). Réutilise le service
## `Accessibility.colorblind()` ; palette Okabe-Ito ami (bleu) / ennemi (vermillon) à la place des
## livrées et du rouge, et un indice sans couleur : l'ennemi est hachuré (plaques) ou en losange
## (minicarte). Rendu seulement.

const FRIEND := Color(0.0, 0.447, 0.698)
const ENEMY := Color(0.835, 0.369, 0.0)


static func active() -> bool:
	return Accessibility.colorblind()


## Couleur d'un camp : celle du camp, ou bleu/vermillon en mode daltonien.
static func side_color(color: Color, is_player: bool) -> Color:
	if not active():
		return color
	return FRIEND if is_player else ENEMY


## Rouge « ennemi » des décales de survol/ciblage.
static func enemy_red(normal: Color) -> Color:
	return ENEMY if active() else normal


## Hachures diagonales sur `rect` (plaque ennemie), pour `canvas`.
static func hatch(canvas: CanvasItem, rect: Rect2, ink: Color) -> void:
	var step := 6.0
	var offset := 0.0
	while offset < rect.size.x + rect.size.y:
		var a := Vector2(rect.position.x + minf(offset, rect.size.x), rect.position.y + maxf(offset - rect.size.x, 0.0))
		var b := Vector2(rect.position.x + maxf(offset - rect.size.y, 0.0), rect.position.y + minf(offset, rect.size.y))
		canvas.draw_line(a, b, Color(ink, 0.55), 1.0)
		offset += step
