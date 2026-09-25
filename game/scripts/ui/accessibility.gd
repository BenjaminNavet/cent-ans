class_name Accessibility
extends RefCounted

## Lot U12 (audit A3 § 6) : lecture des réglages d'accessibilité (onglet « Accessibilité » des
## réglages, autoload `Settings`) et petits outils partagés : motifs du mode daltonien, symboles
## de relation et de moral, encre renforcée du mode contrasté. Aucune règle de jeu.

const KEY_COLORBLIND := "access/colorblind"
const KEY_REDUCE_MOTION := "access/reduce_motion"
const KEY_HIGH_CONTRAST := "access/high_contrast"

## Symbole de chaque relation diplomatique (carte, panneau, légende), lisible sans les couleurs.
const RELATION_SYMBOLS := {
	"self": "♔", "war": "⚔", "truce": "⌛", "peace": "·", "alliance": "⚭", "vassal": "⚜",
	"suzerain": "⚜", "enemy": "⚔", "ally": "⚭", "neutral": "·",
}
const RELATION_NAMES := {
	"self": "Votre royaume", "war": "Guerre", "truce": "Trêve", "peace": "Paix", "alliance": "Alliance",
	"vassal": "Vassal", "suzerain": "Suzerain",
}
## Motif de hachure par relation pour la carte diplomatique : 0 aucun, 1 diagonales, 2 croisillons,
## 3 points, 4 horizontales, 5 verticales.
const RELATION_PATTERNS := {
	"self": 0, "war": 2, "truce": 3, "peace": 0, "alliance": 1, "vassal": 4, "suzerain": 5,
}


static func _settings() -> Node:
	var tree := Engine.get_main_loop() as SceneTree
	return tree.root.get_node_or_null("/root/Settings") if tree != null else null


static func _flag(key: String) -> bool:
	var settings := _settings()
	return settings != null and bool(settings.call("get_value", key))


## Mode daltonien : motifs et symboles en plus des couleurs (relations, moral, jauges).
static func colorblind() -> bool:
	return _flag(KEY_COLORBLIND)


## « Réduire les animations » : pas de fondu, pas de secousse ni de travelling de caméra.
static func reduce_motion() -> bool:
	return _flag(KEY_REDUCE_MOTION)


## Contraste renforcé : encre plus sombre, bords plus épais, parchemin plus clair.
static func high_contrast() -> bool:
	return _flag(KEY_HIGH_CONTRAST)


## Symbole d'une relation (`war`, `alliance`…), vide si inconnue.
static func relation_symbol(relation: String) -> String:
	return str(RELATION_SYMBOLS.get(relation, ""))


## Symbole d'un niveau 0..1 (moral, attitude) : ▲ bon, ■ moyen, ▼ bas.
static func level_symbol(ratio: float) -> String:
	if ratio >= 0.6:
		return "▲"
	if ratio >= 0.3:
		return "■"
	return "▼"


## Texte d'une attitude signée, avec symbole en mode daltonien (« ▲ +35 »).
static func signed_with_symbol(value: int) -> String:
	var text := ("+%d" % value) if value > 0 else str(value)
	if not colorblind():
		return text
	return "%s %s" % ["▲" if value > 0 else ("▼" if value < 0 else "■"), text]
