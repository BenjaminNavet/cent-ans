class_name CountrysideLayer
extends Node3D

## Lot DN-PAYS : campagne vivante hors champs (rendu seulement, aucune règle de jeu). Squelette ;
## voir `docs/wip/dn/pays.md`.

const DATA_FILE := "map/map_countryside.json"

var config: Dictionary = {}
var enabled := true
var force_active := false
var stats: Dictionary = {"cells": 0, "visible_cells": 0, "draw_calls": 0, "instances": 0, "visible": 0, "build_ms_max": 0.0}


static func load_config() -> Dictionary:
	var parsed: Variant = DataFile.read_json(DATA_FILE)
	return parsed if parsed is Dictionary else {}
