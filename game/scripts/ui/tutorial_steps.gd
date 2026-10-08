class_name TutorialSteps
extends RefCounted

## Étapes du tutoriel des premiers tours (textes dans `data/tutorial/steps.json`) : 14 étapes,
## chacune avec un objectif vérifié par `TutorialController` (identifiant `id`), une cible
## (`target`, résolue par le contrôleur en contrôle d'interface ou point de la carte) et un
## conseil historique propre à la faction jouée (France, Angleterre, Bourgogne ; `generic_advice`
## sinon, avec la description de la faction en introduction). Les noms (dirigeant, capitale,
## suzerain, objectifs) viennent de `data/factions`, `data/characters` et de la simulation via
## le contexte passé à `steps` : `{army}`, `{capital}`, `{welcome}`, `{faction}`,
## `{objectives_title}`, `{objectives}`, `{liege}`.
##
## `manual` : étape validée par le bouton « Continuer » (introduction, conclusion).

const DATA_FILE := "tutorial/steps.json"


## Étapes pour `faction_id` ; `context` : `{faction, ruler, capital, objectives, end_year,
## objectives_title, liege, description}` (tout est facultatif).
static func steps(faction_id: String, context: Dictionary = {}) -> Array[Dictionary]:
	var ruler := str(context.get("ruler", ""))
	var liege := str(context.get("liege", ""))
	var values := {
		"army": str(_data()["army_names"].get(faction_id, "votre armée principale")),
		"faction": str(context.get("faction", "votre royaume")),
		"welcome": "Bienvenue, %s" % ruler if ruler != "" else "Bienvenue",
		"capital": str(context.get("capital", "votre capitale")),
		"objectives": str(context.get("objectives", "• Survivre et prospérer.")),
		"objectives_title": str(context.get("objectives_title", "Vos objectifs historiques (avant %s)" % str(context.get("end_year", "1453")))),
		"liege": liege,
	}
	var advice: Dictionary = _data()["advice"].get(faction_id, {})
	if advice.is_empty():
		advice = _data()["generic_advice"].duplicate()
		advice["intro"] = str(context.get("description", ""))
		if liege != "":
			advice["diplomacy"] = advice["diplomacy_vassal"]
	var result: Array[Dictionary] = []
	for step_id in step_ids():
		var base: Dictionary = _data()["steps"][step_id]
		result.append({
			"id": step_id,
			"title": str(base["title"]).format(values),
			"text": str(base["text"]).format(values),
			"objective": str(base["objective"]).format(values),
			"advice": str(advice.get(step_id, "")).format(values),
			"target": str(base.get("target", "")),
			"manual": bool(base.get("manual", false)),
			"modal_ok": bool(base.get("modal_ok", false)),
		})
	return result


## Identifiants des étapes, dans l'ordre du tutoriel.
static func step_ids() -> Array:
	return _data()["step_ids"]


static func count() -> int:
	return step_ids().size()


static func _data() -> Dictionary:
	return DataFile.load_cached(DATA_FILE)
