# 0216 — Forêts de campagne : modèles générés, peuplements par massif, forêt dominante

Date : 2026-10-09 (lot DN-FORET, `docs/wip/dn/forets.md`). Numéro provisoire (renuméroté à la fusion).

## Contexte

Retour du joueur : « aucun asset créé » dans les forêts, et « aucune diversité inter-forêts ». Les
arbres de la carte venaient de planches d'images (fal, ADR 0143) ; les ~30 essences générées par la
nuit DN (`dn/vegetation/tree_*`, ADR 0212) n'étaient utilisées nulle part. Toutes les forêts d'un
même biome avaient le même mélange d'essences. Le XIVe siècle est très boisé, la carte l'était trop peu.

## Décision

1. **Atlas d'imposteurs recuit depuis les glb DN** (`ga3_vegetation_l2.py sheets` puis `atlas`) :
   40 essences (17 nouvelles : charme, frêne, orme, tilleul, aulne, tremble, sorbier, if, chêne
   pubescent, chêne-liège, cèdre du Liban, sapin / mélèze / pin / épicéa de Sibérie, pin cembro, pin
   de Calabre) ; `dn_id` dans `data/art/tree_species.yaml`. Teinte propre au modèle, ramenée vers
   le vert (`foliage_chroma`, les textures TRELLIS sont beiges) et à la luminance de la famille.
   Kermès, lentisque et pommier gardent leur planche d'image (pas de glb exploitable ; le glb du
   pommier est cassé, celui de l'épicéa de Sibérie aussi : l'épicéa commun le remplace).
2. **Modèles décimés dans une zone autour du point visé** (`DnTreeModels`, `foliage_model.gdshader`) :
   glb `lod1` (≈ 1 200 triangles) normalisé (hauteur 1, houppier 1), un MultiMesh par essence et par
   partie, tampons découpés par ligne d'atlas et par rayon côté Rust (`VegetationScatter.split_rows`).
   Les imposteurs sont dégénérés dans la zone (`model_zone`, shader d'imposteur). Rayon = 0,6 × distance
   du rig, borné à [8, 40], éteint au-delà de 150. La zone avance par paliers d'un quart de rayon.
   Sans paquet de modèles (ADR 0212) : imposteurs seuls. `--no-dn-trees` : désactivé. Pourquoi une
   zone : une partie de tuile porte 10 à 20 k arbres (62 M de primitives mesurées partie entière).
3. **Peuplements** (`core/crates/vegetation/src/stands.rs`, `data/art/forest_stands.json`, schéma
   `art_forest_stands`) : 23 types (chênaie-charmaie, chênaie atlantique, hêtraie, hêtraie-sapinière,
   sapinière-pessière, mélézin-cembraie, pinède sylvestre, pinède maritime des Landes, châtaigneraie,
   chênaie verte, pinède d'Alep, suberaie, pinède noire, aulnaie-boulaie, frênaie-ormaie, chênaie
   continentale, taïga d'épicéa et de pin, taïga sibérienne, steppe boisée, cédraie, pinède de Calabre,
   pinède à parasols, lande). Chaque peuplement : multiplicateur par essence, teinte, densité, hauteur.
   Les 146 massifs nommés de `historical_forests.json` reçoivent le leur (`regions`) ; ailleurs, une
   cellule d'écorégion (Voronoi de 36 px) tire parmi les peuplements admissibles (biome, altitude,
   latitude, longitude, part de résineux). Optionnel : sans table, semis HB4 inchangé.
4. **Forêt dominante** : `data/map/landcover_params.json` (`cleared_scale` 0,55) réduit le
   défrichement KK10 retenu par `landcover.py` : forêt 40 % → 51 % des terres ; `splat.png`,
   `forest_kind.png`, `wetlands.png` régénérés. Compatible avec DN-SOL (ADR 0213) : les champs
   perdus deviennent prés et landes.
5. **Arbres grossis** : `generalised_tree_height` 0,8 → 1,5 unités, `generalised_spacing` 0,9 → 1,3
   (canopée fermée, moins d'instances). Lisibles comme une maquette entre 120 et 220.

## Conséquences

- Coût mesuré (banc `tools/dn_forest_bench.sh`, 1280x720 fenêtré, p50) : d 45 18,4 → 16,8 ms
  (`--no-dn-trees` / modèles), d 120 16,8 → 20,3 ms ; 4,7 M primitives contre 2,6 M.
- Landes de Gascogne : massif « lande » dans les données, donc presque sans arbres ; les pins
  maritimes ne se voient que sur ses franges boisées.
- Le semis GDScript de repli ignore les peuplements.
