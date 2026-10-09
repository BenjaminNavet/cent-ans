# 0224 — La campagne vivante hors champs : une couche à règles en données, albédo partagé et réduit

Numéro provisoire (lot DN-PAYS) : l'orchestrateur renumérote à la fusion. Date : 2026-10-09
(`docs/wip/dn/pays.md`).

## Contexte

Le joueur veut que les textures plates de la carte de campagne cèdent la place aux glb générés (ADR
0210 à 0212) partout où c'est possible. Pour la campagne vivante hors champs cultivés (bocage, puits,
croix, moulins à vent, salines, ruines, piloris, routes), deux couches existent déjà : `FaunaLayer`
(DN-ME2, troupeaux, charge déjà les `dn/fauna/animal_*_lod*.glb`) et les figurines FK (ADR 0122,
charrettes `folk/` animées par AS1).

## Décision

1. **Une couche `CountrysideLayer`** (`game/scripts/map/countryside_layer.gd`, shader
   `countryside.gdshader`) pour les objets fixes, sur le modèle de `FaunaLayer` : semis par cellule de
   96 px, déterministe, un `MultiMesh` par modèle et par cellule, taille tenue à l'écran, fondu de
   distance propre à chaque modèle, `--no-countryside` pour l'A/B. Elle réutilise `FaunaLayer` pour les
   lacs et marais affichés plutôt que de les dupliquer.
2. **Toutes les règles de placement sont en données** (`data/map/map_countryside.json`, schéma
   `map_countryside.schema.json`) : quatre modes (`scatter`, `paddock` enclos de clôtures, `village` à
   côté des colonies et hameaux, `road` au bord des routes), régions en ellipses lon/lat (ou lues dans
   un autre fichier : les salines reprennent les sites de `map_freshwater.json`), filtres de site
   (biome de `biomes.png`, prairie/forêt/cultures du splat, altitude, pente, côte, marais), poids par
   saison. Brancher un modèle ou une région = une ligne de données.
3. **Les troupeaux ne sont pas reprogrammés** : `FaunaLayer` utilise déjà les glb DN. Les charrettes
   FK2 (`merchant_cart`, `peasant_cart`) restent : elles sont animées (roues et bêtes, AS1) et leurs
   emplacements de figurines (`slots`) sont calés sur leur géométrie ; les chariots DN, statiques,
   sont posés *en plus* au bord des routes (règles `road`).
4. **Un albédo par modèle, réduit à 512 px.** Les trois niveaux de détail d'un modèle ont un albédo
   identique (même fichier, vérifié par somme de contrôle) en 1024 px sans compression : 108 textures
   pèseraient ~450 Mo. La couche garde une seule texture par modèle, redimensionnée (`texture_px`),
   avec mipmaps (~40 Mo pour les 36 modèles). Les props font quelques dizaines de pixels à l'écran.
5. **Chargement en tâche de fond** : `ResourceLoader.load_threaded_request` à la mise en place, un
   modèle préparé par image une fois chargé (le décodage de texture coûte 5 à 20 ms par glb).

## Conséquences

- Le coût de rendu est borné par `max_visible_instances` (2 500) et un appel de dessin par modèle et
  par cellule (45 à 85 en vue rapprochée en Bretagne, plafond testé à 100).
- Ni la simulation ni les règles ne sont touchées. Désactiver : `--no-countryside`,
  `--life-off=countryside`.
- Suites possibles : réduire à la source l'import des `*_Image_0.jpg` du paquet (mode VRAM au lieu de
  sans perte) ; calage des puits et piloris au centre des villages 1:1 quand ceux-ci exposeront leur
  place du marché.
