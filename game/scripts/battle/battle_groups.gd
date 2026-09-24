class_name BattleGroups
extends RefCounted

## Regroupements d'interface du HUD de bataille (F5b) : les trois « batailles » de l'ost
## (avant-garde, corps de bataille, arrière-garde) qui rangent les cartes d'unités, et les
## groupes de sélection Ctrl+1..9 (enregistrer) / 1..9 (rappeler). Pure présentation : aucune
## règle, les ordres passent toujours par `BattleSim.issue_command`.
##
## Assignation par défaut selon le rôle : cavalerie → avant-garde ; tireurs et infanterie →
## corps de bataille ; réserve (clé `reserve` si la simulation l'expose) et engins de siège →
## arrière-garde.

const ORDER := ["vanguard", "main", "rear"]
const LABELS := {"vanguard": "Avant-garde", "main": "Bataille", "rear": "Arrière-garde"}
const MAX_GROUPS := 9

## numéro de groupe (1..9) → ids d'unités
var groups: Dictionary = {}


static func default_battle(unit: Dictionary) -> String:
	if bool(unit.get("reserve", false)):
		return "rear"
	match str(unit.get("category", "")):
		"cavalry":
			return "vanguard"
		"siege":
			return "rear"
	if ["ram", "tower"].has(str(unit.get("render", ""))):
		return "rear"
	return "main"


## Enregistre la sélection sous le numéro `number` (1..9) ; une sélection vide efface le groupe.
func save(number: int, ids: Array) -> void:
	if number < 1 or number > MAX_GROUPS:
		return
	if ids.is_empty():
		groups.erase(number)
		return
	var copy: Array[int] = []
	for id in ids:
		copy.append(int(id))
	groups[number] = copy


## Ids du groupe `number` encore présents sur le champ ([] si le groupe est vide).
func recall(number: int, units: Array) -> Array[int]:
	var result: Array[int] = []
	var ids: Array = groups.get(number, [])
	for unit in units:
		if ids.has(int(unit["id"])) and bool(unit["present"]):
			result.append(int(unit["id"]))
	return result


## Numéros des groupes contenant `unit_id` (pour les pastilles des cartes).
func numbers_of(unit_id: int) -> Array[int]:
	var result: Array[int] = []
	for number in groups:
		if (groups[number] as Array).has(unit_id):
			result.append(int(number))
	result.sort()
	return result
