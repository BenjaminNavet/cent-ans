# DN-PAYS : campagne vivante hors champs (worktree agent-a3f5b4976ca8a4548)

Périmètre : haies/clôtures (bocage), puits, croix de chemin et calvaires, moulins à vent, salines côtières, ruines, piloris, bord des routes (charrettes, chariots, caravanes). ADR provisoire : `docs/decisions/0220-campagne-vivante-hors-champs.md` (à renuméroter).

## État (fait)
- `game/scripts/map/countryside_layer.gd` + `game/shaders/countryside.gdshader` : semis par cellule de 96 px, un MultiMesh par modèle et par cellule, taille tenue à l'écran (`size_k`, `max_mult` par modèle), enclos qui s'écartent (`spread_exponent`), fondu par modèle, amincissement par modèle (`max_instances`), chargement des glb en tâche de fond, albédo unique réduit à 512 px par modèle.
- `data/map/map_countryside.json` (25 règles, 36 modèles, 23 régions) + schéma `data/schemas/map_countryside.schema.json` + `tools/tests/test_map_countryside_schema.py`. Modes : `scatter`, `paddock`, `village`, `road`. Les salines reprennent les sites de `map_freshwater.json`.
- Câblé dans `CampaignLife._setup_countryside` (`--no-countryside`, `--life-off=countryside`).
- Tests : `game/tests/dn_pays_test.gd` (données, semis sur toute la carte ~1,2 s, localisation, rendu, parchemin), capture `game/tests/dn_pays_shot.gd`.
- Troupeaux : rien à faire, `FaunaLayer` charge déjà les `dn/fauna/animal_*`.

## Mesures
- Semis de toute la carte : 37 000 instances en ~1,2 s (cellule la plus lente 28 ms), déterministe.
- Vues rapprochées (Bretagne) : 38 à 83 appels de dessin, ~1 900 instances affichées ; à d = 40, 57 appels. Construction d'une cellule : ≤ 75 ms (hors premier modèle) ; préparation d'un modèle ≤ ~170 ms en test (les chargements sont lancés en tâche de fond à la mise en place, un modèle prêt par image).
- Machine chargée (builds parallèles) : pas de temps d'image fiable.

## Écarts / points ouverts
- Charrettes FK2 (`merchant_cart`, `peasant_cart`) non remplacées : animées (AS1) et `slots` calés sur leur géométrie ; les chariots DN sont posés en plus.
- Les albédos du paquet sont importés sans perte en 1024 px (~4 Mo en mémoire chacun) ; la couche les réduit, mais un import VRAM à la source serait mieux.
- Puits et piloris sont posés à 160-380 m du point du village : calage sur la place du marché à faire quand les villages 1:1 l'exposeront.
- Densités et tailles d'écran (`size_k` 0,02, `max_mult`) réglées à l'œil sur 5 captures (Bretagne seulement vue) ; autres régions non vérifiées visuellement. Enclos de bocage encore discrets.
- La saison d'une règle (`seasons`) n'a pas été vérifiée visuellement (foin, bouviers, cabanes d'alpage).
- Le nom de la saison dans la capture sans simulation est forcé ; le test de fumée complet exige `core/build.sh` du worktree.
