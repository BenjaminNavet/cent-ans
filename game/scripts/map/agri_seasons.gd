class_name AgriSeasons
extends RefCounted

## Lot ME8 (DN) : variantes saisonnières d'arbres (sapin enneigé, chêne doré, hêtre cuivré,
## mélèze doré) déclarées dans `data/map/agri_landscapes.json` (`seasonal_variants`). Tant que le
## modèle n'est pas ingéré (`data/art/dn_manifest.json`), `model_for` rend "" : le feuillage reste
## celui de l'essence, teinté par la saison dans `foliage_common.gdshaderinc`. Brancher un
## nouveau modèle = une ligne de données, aucune règle ici.

const FILE := "map/agri_landscapes.json"
const MANIFEST := "art/dn_manifest.json"


## Identifiant du modèle saisonnier ingéré pour (classe d'essence, saison), "" sinon.
## `variants` : `seasonal_variants` du fichier ; `ingested` : identifiants du manifeste.
static func model_for(variants: Array, ingested: Dictionary, tree_class: String, season: String) -> String:
	for variant in variants:
		var entry: Dictionary = variant
		if str(entry.get("tree_class", "")) == tree_class and str(entry.get("season", "")) == season:
			var id := str(entry.get("model_id", ""))
			return id if ingested.has(id) else ""
	return ""


static func load_variants(data_dir: String) -> Array:
	var text := FileAccess.get_file_as_string(data_dir.path_join(FILE))
	var parsed: Variant = JSON.parse_string(text) if not text.is_empty() else null
	return (parsed as Dictionary).get("seasonal_variants", []) if parsed is Dictionary else []


