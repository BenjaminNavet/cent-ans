class_name AlertRoutes
extends RefCounted

## RG2 : cible d'un clic sur une pastille d'alerte de la cloche, jamais vide. Les genres de
## `ROUTES` ont une action dédiée (exécutée par `HudController._on_alert_activated`) ; les autres
## retombent sur le personnage / l'armée / la province de l'alerte (« focus »), à défaut sur le
## journal (genre inconnu compris).
const ROUTES := {
	"chronicle_decision": "chronicle", "debt": "faction", "research_idle": "tech",
	"mission_due": "mission", "mission": "mission", "diplomacy_offer": "diplomacy",
	"ransom": "ransom", "herbarium": "herbarium", "trade": "trade", "crusade": "crusade",
	"royal_act": "faction", "level_up": "court", "loyalty": "court", "captain": "court",
}


## Itinéraire d'une alerte : `ROUTES`, sinon « focus » si elle porte une cible, sinon « journal ».
static func route_of(alert: Dictionary) -> String:
	var kind := str(alert.get("kind", ""))
	if ROUTES.has(kind):
		return str(ROUTES[kind])
	for key in ["character_id", "army_id", "province_id"]:
		if str(alert.get(key, "")) != "":
			return "focus"
	return "journal"
