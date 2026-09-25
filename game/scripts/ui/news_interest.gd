class_name NewsInterest
extends RefCounted

## Lot U5 (audit A3, T2) : filtre d'intérêt des nouvelles (lettres scellées, bandeau du haut,
## rubrique « Le monde » du rapport de saison). Une nouvelle d'une autre faction n'est gardée que
## si elle touche le joueur, un voisin, un allié, un ennemi, un vassal ou une grande puissance.
## Aucune règle de jeu : lecture des relations (`get_diplomacy`), des propriétaires de province
## (`get_province_state`) et des voisinages de la carte (`MapData`).

## Portée des nouvelles (réglage `interface/news_filter`).
const MODE_ALL := "all"
const MODE_INTEREST := "interest"
const MODE_OWN := "own"
const MODES: Array[String] = [MODE_INTEREST, MODE_ALL, MODE_OWN]
const MODE_LABELS: Array[String] = ["Voisins, alliés, ennemis et grandes puissances", "Toute l'Europe", "Seulement mon royaume"]
## Nombre de grandes puissances (factions les plus puissantes, joueur exclu).
const GREAT_POWER_COUNT := 5
## Statuts diplomatiques qui rendent une faction « proche » du joueur.
const CLOSE_STATUSES := ["war", "alliance", "vassal", "suzerain", "truce"]

## Genres assez importants pour être suivis chez une grande puissance lointaine ; les autres
## (batailles, prises, mariages, épidémies…) exigent un voisin, un allié ou un ennemi.
const GREAT_POWER_KINDS := [
	"war_declared", "peace_signed", "alliance_formed", "alliance_broken", "succession", "faction_destroyed",
	"schism", "excommunication", "vassalage", "chronicle", "victory", "defeat", "campaign_ended"]

## Raison de l'intérêt, du plus fort au plus faible.
enum Interest { NONE, GREAT_POWER, NEIGHBOR, CLOSE, PLAYER }

var mode: String = MODE_INTEREST
var player: String = ""
## faction → `Interest` (le joueur, ses voisins, alliés, ennemis, grandes puissances).
var factions: Dictionary = {}
## province → faction propriétaire (instantané pris à `build`).
var owners: Dictionary = {}
## Provinces du joueur ou voisines des siennes.
var near_provinces: Dictionary = {}


## Instantané des relations du joueur. `sim` : `CampaignSim` (ou maquette) ; `map_data` peut
## être nul (pas de voisinage : seuls les liens diplomatiques et la puissance comptent).
static func build(sim: Object, map_data: MapData, player_faction: String, filter_mode: String = MODE_INTEREST) -> NewsInterest:
	var interest := NewsInterest.new()
	interest.mode = filter_mode if MODES.has(filter_mode) else MODE_INTEREST
	interest.player = player_faction
	if player_faction == "":
		return interest
	interest.factions[player_faction] = Interest.PLAYER
	if sim == null:
		return interest
	if sim.has_method("get_diplomacy"):
		var powers: Array = []
		for entry in sim.call("get_diplomacy", player_faction):
			var id := str(entry.get("id", ""))
			if CLOSE_STATUSES.has(str(entry.get("status", ""))):
				interest.factions[id] = Interest.CLOSE
			powers.append([int(entry.get("power", 0)), id])
		powers.sort_custom(func(a: Array, b: Array) -> bool: return a[0] > b[0])
		for pair in powers.slice(0, GREAT_POWER_COUNT):
			interest._raise(str(pair[1]), Interest.GREAT_POWER)
	if map_data != null and sim.has_method("get_province_state"):
		interest._scan_neighbors(sim, map_data)
	return interest


func _raise(faction: String, level: Interest) -> void:
	if faction != "" and int(factions.get(faction, Interest.NONE)) < level:
		factions[faction] = level


func _scan_neighbors(sim: Object, map_data: MapData) -> void:
	var own: Array = []
	for index in range(1, map_data.province_count + 1):
		var province := map_data.get_province(index)
		var id := str(province.get("id", ""))
		if id == "":
			continue
		var state: Dictionary = sim.call("get_province_state", id)
		var owner := str(state.get("owner", ""))
		owners[id] = owner
		if owner == player or str(state.get("controller", "")) == player:
			own.append(province)
	for province in own:
		near_provinces[str(province.get("id", ""))] = true
		for neighbor in province.get("neighbors", []):
			var neighbor_id := str(neighbor)
			near_provinces[neighbor_id] = true
			_raise(str(owners.get(neighbor_id, "")), Interest.NEIGHBOR)


## Intérêt d'une faction pour le joueur (`Interest`).
func faction_interest(faction: String) -> Interest:
	return int(factions.get(faction, Interest.NONE)) as Interest


## Intérêt d'un événement (`{kind, text_fr, faction?, province?}`) : le plus fort entre sa
## faction, le propriétaire de sa province et la proximité de cette province.
func event_interest(event: Dictionary) -> Interest:
	var faction := str(event.get("faction", ""))
	if faction == "" and str(event.get("faction_id", "")) != "":
		faction = str(event.get("faction_id", ""))
	var level := faction_interest(faction) if faction != "" else Interest.NONE
	var province := str(event.get("province", event.get("province_id", "")))
	if province != "":
		level = maxi(level, faction_interest(str(owners.get(province, ""))))
		if near_provinces.has(province):
			level = maxi(level, Interest.NEIGHBOR)
	if faction == "" and province == "":
		return Interest.GREAT_POWER  # nouvelle sans lieu ni faction : générale (schisme, chronique)
	return level as Interest


## Vrai si la nouvelle doit être montrée au joueur selon `mode`.
func keeps(event: Dictionary) -> bool:
	if mode == MODE_ALL or player == "":
		return true
	var level := event_interest(event)
	if mode == MODE_OWN:
		return level == Interest.PLAYER
	if level == Interest.GREAT_POWER:
		return GREAT_POWER_KINDS.has(str(event.get("kind", "")))
	return level != Interest.NONE


## Libellé court de la raison (infobulle des lettres).
static func interest_label(level: Interest) -> String:
	match level:
		Interest.PLAYER:
			return "Votre royaume"
		Interest.CLOSE:
			return "Allié, ennemi ou vassal"
		Interest.NEIGHBOR:
			return "Voisin"
		Interest.GREAT_POWER:
			return "Grande puissance"
	return "Nouvelle lointaine"
