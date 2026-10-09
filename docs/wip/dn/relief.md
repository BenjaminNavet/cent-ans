# DN-RELIEF : roches générées sur la carte de campagne

Branche worktree-agent-ae2942e4f7bfcd2df. ADR 0218 (`docs/decisions/0218-roches-dn-campagne.md`).

## Décision de départ
La couche existe déjà : `RockOutcrops` (HB5, ADR 0143), catalogue `data/art/rock_outcrops.yaml`,
schéma `art_rock_outcrops.schema.json`. Pas de couche concurrente : on étend le catalogue avec des
entrées pointant vers les modèles DN (`dn/rocks/*`, en mètres réels) et on remplace les 6 modèles HB.

## Plan
1. Schéma : champs `model` (stem DN), `model_length_m` (normalisation), `coast` (distance à la côte,
   roche de côte par géologie `coast_types.json`).
2. Code : chargement DN (normalisation du maillage à 1 m), terme côte dans `suitability`.
3. Données : entrées DN par biome / altitude / pente / côte / mégalithes.
4. Tests : `game/tests/dn_relief_test.gd`, smoke, mesure perf.

## État (DONE, non fusionné)
- Schéma + code (`model`, `model_length_m`, `coast`) + catalogue 26 entrées DN (+ aiguille alpine HB).
- Tests : `hb5_rocks_test.gd` (adapté), `dn_relief_test.gd` (catalogue, géologie de côte, Alpes, coût), smoke OK.
- Réglages : max_visible_triangles 450 k, lod 30/120, far_scale 2.6, shadow_max_lod 0.
- Coût (Alpes d = 75, banc --bench-map, machine chargée) : frame p50 11,5 ms sans roches -> 13,5 ms (ombres LOD0 seules) ; 18,5 ms avec ombres LOD1. Appels de dessin +50 à +230 selon la distance.
- Ouvert : roches parfois près des villes (marge de colonie), densité plafonnée par le budget de triangles (part 0,2-0,3 dans les Alpes), mégalithes très rares, sel/travertin/serpentine non utilisés, LOD distances à régler sur machine calme.
