class_name EconomyIcons
extends RefCounted

## RG12 : point unique de résolution des icônes de la fenêtre Économie. Chaque notion de la
## fenêtre pointe vers un identifiant de `IconLibrary` (`assets/icons/icons.json`) ; RG14 ajoute
## les icônes de ressources `res_*` dans cette même table, donc rien à changer ici pour qu'elles
## remplacent les actuelles : `IconLibrary.resolve` retombe sur le repli de catégorie tant
## qu'une icône manque.

## Notion de la fenêtre → identifiant d'icône.
const CONCEPTS := {
	"economy": "hud_treasury",
	"treasury": "hud_treasury",
	"currency": "hud_treasury",
	"trade": "bld_market",
	"agreement": "bld_market",
}


## Identifiant d'icône d'une notion (`CONCEPTS`) ou, à défaut, l'identifiant lui-même
## (`res_wool`…).
static func icon_id(concept: String) -> String:
	return str(CONCEPTS.get(concept, concept))


## Identifiant d'icône d'un bien (`res_wool`) : le bien lui-même, l'icône éventuellement générique
## étant choisie par `IconLibrary`.
static func resource_icon_id(resource: String) -> String:
	return resource


## Pastille icône + texte (infobulle facultative) pour une notion ou un identifiant.
static func chip(concept: String, text: String, tooltip: String = "", icon_size: float = 18.0, font_size: int = 14) -> IconChip:
	return IconChip.create(icon_id(concept), text, tooltip, icon_size, font_size)
