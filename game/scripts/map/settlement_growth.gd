class_name SettlementGrowth
extends RefCounted

## Lot CV1 : colonies qui grandissent avec leur niveau (rendu seulement). Niveau visuel :
## 0 village, 1 bourg (ouvert, halle), 2 ville (murailles), 3 cité (cathédrale, faubourgs,
## château). Calculé depuis le type, la fortification (`CampaignSim.settlements()`) et la
## population de la province (`get_province_state`). Paris (monument L1) est exclu.

const LEVEL_NAMES: Array[String] = ["village", "bourg", "ville", "cité"]
## Colonies exclues de la croissance générique (monuments dédiés, lot L1).
const EXCLUDED: Array[String] = ["set_paris"]


## Niveau visuel d'une colonie, -1 si elle n'en a pas (château, abbaye) ou est exclue.
static func level_of(entry: Dictionary, province_population: float) -> int:
	return -1
