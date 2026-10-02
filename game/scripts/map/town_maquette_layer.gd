class_name TownMaquetteLayer
extends Node3D

## Lot GC1 (ADR 0158), prototype : villes stylisées. Chaque colonie est une maquette du kit
## (`assets/models/settlements/`) posée à sa vraie position, à taille monde constante par type ;
## les villes emblématiques reprennent leur maquette `LandmarkModel` (monuments grossis, fleuve
## propre). Activé par `--town-style=maquette` ; `--maquette-scale=<m>` multiplie les tailles du
## kit. Purement visuel.

## Largeur visée (unités monde, 1 unité ≈ 719 m) d'une maquette du kit, par type de colonie.
const KIT_WIDTH := {"city": 8.0, "town": 4.5, "castle": 2.6, "abbey": 2.6, "village": 2.4}
const KIT_MODEL := {"city": "city", "town": "town", "castle": "castle", "abbey": "abbey", "village": "village"}
## Largeur des modèles du kit à l'échelle 1 (unités Blender).
const KIT_NATIVE := {"city": 3.6, "town": 2.0, "castle": 1.9, "abbey": 2.2, "village": 2.0}

var stats: Dictionary = {}


static func enabled() -> bool:
	return "--town-style=maquette" in OS.get_cmdline_user_args()


func setup(map_data: MapData, terrain: TerrainBuilder, layer: SettlementLayer) -> void:
	var factor := 1.0
	for argument in OS.get_cmdline_user_args():
		if argument.begins_with("--maquette-scale="):
			factor = maxf(argument.get_slice("=", 1).to_float(), 0.1)
	var landmarks := 0
	var kit := 0
	for i in layer.data.settlements.size():
		var entry: Dictionary = layer.data.settlements[i]
		var id := str(entry["id"])
		var plan := LandmarkLibrary.for_settlement(id)
		if not plan.is_empty():
			var landmark := LandmarkModel.create(plan, terrain)
			if landmark != null:
				add_child(landmark)
				landmarks += 1
				continue
		var kind := str(entry.get("kind", "village"))
		if not KIT_MODEL.has(kind):
			kind = "village"
		var variant := "a" if absi(id.hash()) % 2 == 0 else "b"
		var model_scale: float = float(KIT_WIDTH[kind]) / float(KIT_NATIVE[kind]) * factor
		var model := ModelLibrary.instantiate("settlements/%s_%s" % [KIT_MODEL[kind], variant], model_scale)
		if model == null:
			continue
		var px := layer.model_px(i)
		model.name = "Maquette_" + id
		model.position = Vector3(px.x, map_data.surface_world_at(px.x, px.y), px.y)
		model.rotation.y = float(absi(id.hash()) % 360) * PI / 180.0
		add_child(model)
		kit += 1
	stats = {"landmarks": landmarks, "kit": kit, "factor": factor}
	print("TownMaquetteLayer: %s" % JSON.stringify(stats))
