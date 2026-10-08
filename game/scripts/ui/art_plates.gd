class_name ArtPlates
extends RefCounted

## Lot AR1 — habillage illustré (`data/ui/illustrations.json`, schéma
## `data/schemas/illustrations.schema.json`) : écrans de chargement par contexte (bataille,
## siège, bataille navale, campagne) avec citation de chroniqueur, vignettes des événements
## (par genre d'événement du cœur ; l'ordre du fichier fait la priorité) et écrans de fin.
## Aucune règle : lecture et choix d'illustration seulement.

const DATA_PATH := "ui/illustrations.json"

static var _lookup := JsonLookup.new(DATA_PATH)


static func data() -> Dictionary:
	return _lookup.data()


## Durée d'affichage minimale d'un écran de chargement (secondes).
static func min_seconds() -> float:
	return float((data().get("loading", {}) as Dictionary).get("min_seconds", 3.0))


static func max_seconds() -> float:
	return float((data().get("loading", {}) as Dictionary).get("max_seconds", 4.0))


## Écrans de chargement d'un contexte (`battle`, `siege`, `naval`, `campaign`).
static func loading_screens(context: String) -> Array:
	var result: Array = []
	for screen in (data().get("loading", {}) as Dictionary).get("screens", []):
		if context in (screen as Dictionary).get("contexts", []):
			result.append(screen)
	return result


## Un écran tiré au hasard pour `context` ({} si aucun).
static func random_loading_screen(context: String) -> Dictionary:
	var screens := loading_screens(context)
	return screens.pick_random() if not screens.is_empty() else {}


## Vignette d'un genre d'événement ({} si aucune).
static func vignette_for_kind(kind: String) -> Dictionary:
	for vignette in data().get("vignettes", []):
		if kind in (vignette as Dictionary).get("kinds", []):
			return vignette
	return {}


## Vignette du genre le plus prioritaire parmi `events` (dictionnaires avec `kind`).
static func vignette_for_events(events: Array) -> Dictionary:
	var kinds := {}
	for event in events:
		if event is Dictionary:
			kinds[str(event.get("kind", ""))] = true
	for vignette in data().get("vignettes", []):
		for kind in (vignette as Dictionary).get("kinds", []):
			if kinds.has(kind):
				return vignette
	return {}


## Écran de fin (`battle_victory`, `battle_defeat`, `campaign_victory`, `campaign_defeat`).
static func ending(outcome: String) -> Dictionary:
	for entry in data().get("endings", []):
		if str((entry as Dictionary).get("outcome", "")) == outcome:
			return entry
	return {}


static func texture(entry: Dictionary) -> Texture2D:
	var path := str(entry.get("image", ""))
	return PortraitLoader.load_texture(path) if path != "" else null
