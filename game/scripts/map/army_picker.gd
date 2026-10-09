class_name ArmyPicker
extends RefCounted

## Sélection d'une armée sous un point écran (SC MC8) : étendard, figurines ou plaque. Fonction
## pure sur les marqueurs et plaques d'`ArmyMarkers`.

## Distance (pixels) sous laquelle un point proche de la silhouette compte comme « presque ».
const PICK_NEAR_PX := 10.0


## Q2 / SA (ADR 0160) : armée la plus proche de `screen_position`, avec `score` et `direct`
## pour départager une armée et une colonie sous le même point. `direct` : le point est dans la
## silhouette projetée de l'armée (`ArmyMarker.screen_rect`) ou sur sa plaque ; `score` vaut alors
## 0 sur la plaque, sinon la distance normalisée au centre de la silhouette (0 à 1). À moins de
## `PICK_NEAR_PX` de la silhouette : `direct` faux, `score` entre 1 et 2. {} si rien.
## `exclude` : armée ignorée (la sélection, quand on vise une cible pour elle).
static func pick_scored(camera: Camera3D, markers: Dictionary, plates: Dictionary, screen_position: Vector2, exclude: String = "") -> Dictionary:
	if camera == null:
		return {}
	var best := ""
	var best_score := INF
	for id in markers:
		var marker: ArmyMarker = markers[id]
		if id == exclude or not marker.is_visible_in_tree():
			continue
		var plate: PanelContainer = plates.get(id)
		if plate != null and plate.visible and plate.get_global_rect().has_point(screen_position):
			return {"id": id, "score": 0.0, "direct": true}
		var rect := marker.screen_rect(camera)
		if rect.size == Vector2.ZERO:
			continue
		var score := INF
		if rect.has_point(screen_position):
			var offset := (screen_position - rect.get_center()).abs() / (rect.size * 0.5)
			score = maxf(offset.x, offset.y)
		else:
			var outside := (screen_position - rect.get_center()).abs() - rect.size * 0.5
			var gap := Vector2(maxf(outside.x, 0.0), maxf(outside.y, 0.0)).length()
			if gap < PICK_NEAR_PX:
				score = 1.0 + gap / PICK_NEAR_PX
		if score < best_score:
			best_score = score
			best = id
	return {"id": best, "score": best_score, "direct": best_score <= 1.0} if best != "" else {}
