class_name PossessionText
extends RefCounted

## Lot RJ-c (ADR 0175) : textes de possession et d'occupation (survol de province, panneaux de
## province et de colonie). Le statut vient du cœur (`CampaignSim.province_possession` /
## `settlement_possession`) : `own`, `own_occupied`, `occupied_by_viewer`, `foreign`,
## `foreign_occupied`. Aucune règle ici : seulement la mise en mots et la couleur de position
## (ADR 0155, `StanceCues`).

const OWN := "own"
const OWN_OCCUPIED := "own_occupied"
const OCCUPIED_BY_VIEWER := "occupied_by_viewer"
const FOREIGN := "foreign"
const FOREIGN_OCCUPIED := "foreign_occupied"

## Encre lisible sur parchemin par catégorie de position (le doré des frontières, trop clair
## sur le papier, est foncé ici) ; « other » : encre ordinaire.
const INK := {
	"self": Color("#7a5a0c"),
	"friend": Color("#2a6a2a"),
	"enemy": Color("#8f1510"),
}


## Statut d'une province ou d'une place en une ligne. `owner_label`, `controller_label` : noms
## courts des factions propriétaire et détentrice.
static func status_line(status: String, owner_label: String, controller_label: String) -> String:
	match status:
		OWN:
			return "À vous"
		OWN_OCCUPIED:
			return "À vous — occupée par %s" % controller_label
		OCCUPIED_BY_VIEWER:
			return "Occupée par vous (propriétaire de droit : %s) — rendue à la paix si non cédée" % owner_label
		FOREIGN:
			return "À %s" % owner_label
		FOREIGN_OCCUPIED:
			return "À %s — occupée par %s" % [owner_label, controller_label]
	return ""


## Catégorie de position (`self`, `friend`, `enemy`, `other`) qui colore le statut : celle du
## détenteur actuel (comme les noms de ville, ADR 0155) ; une place occupée par le joueur est
## la sienne pour la couleur.
static func cue_of(status: String, owner: String, controller: String, player: String, stances: Dictionary) -> String:
	match status:
		OWN, OCCUPIED_BY_VIEWER:
			return StanceCues.SELF
		OWN_OCCUPIED, FOREIGN_OCCUPIED:
			return StanceCues.category_of(controller, player, stances)
	return StanceCues.category_of(owner, player, stances)


## Encre d'un statut sur parchemin ; `ordinary` pour une faction neutre.
static func ink(cue: String, ordinary: Color) -> Color:
	return INK.get(cue, ordinary)


## Phrase d'aide du panneau de province : la cité donne le contrôle, le traité la possession.
static func province_help(city_name: String) -> String:
	return "La cité de %s donne le contrôle de la province ; la possession s'obtient par traité (cession)." % city_name


## Ligne « places tenues » du panneau de province : `held` sur `total`, et le bonus de province
## complète (`whole_holder` : faction qui les tient toutes, "" si aucune).
static func held_line(held: int, total: int, whole_holder: String, player: String) -> String:
	var line := "Places tenues : %d sur %d" % [held, total]
	if whole_holder != "" and whole_holder == player:
		line += " — province complète : bonus acquis"
	elif held > 0 and held < total:
		line += " — encore %s pour le bonus de province complète" % FrText.count(total - held, "place")
	return line


## Mention d'une place dans la liste des colonies : "" si elle n'est pas occupée.
static func occupied_mention(status: String, owner_label: String, controller_label: String) -> String:
	match status:
		OWN_OCCUPIED, FOREIGN_OCCUPIED:
			return "occupée par %s" % controller_label
		OCCUPIED_BY_VIEWER:
			return "occupée par vous (à %s de droit)" % owner_label
	return ""


## Phrase d'aide du panneau de colonie (`is_city` : la place est la cité de sa province).
static func settlement_help(is_city: bool, province_name: String, city_name: String) -> String:
	if is_city:
		return "Cité de %s : qui la tient contrôle la province. La possession ne change que par traité (cession)." % province_name
	return "Place de %s : la tenir ne donne pas la province (c'est la cité de %s qui la donne) ; elle compte pour le bonus de province complète." % [province_name, city_name]
