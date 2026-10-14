# CC — carte composée façon Total War (spécification)

Date : 2026-10-14. Demande du joueur : après le relief exagéré (RE, ADR 0353), « pour plus
ressembler à Total War, ne faut-il pas carrément une réinterprétation de la géographie (sélection
de quelques villes, repositionnement, simplification) ? » ; choix « C » (réinterprétation), puis :

| Question | Choix du joueur |
|---|---|
| Emprise | **Europe + Méditerranée** : Atlantique → Russie occidentale, Maghreb, Levant ; Oural et steppe lointaine retirés |
| Lieux jouables | **≈ 400** (≈ 150-200 provinces), entre Rome II (183) et Warhammer III (≈ 550) |
| Fidélité | **Reconnaissable, caricaturée** : silhouettes, grands fleuves, chaînes et villes au bon endroit relatif ; tailles, distances, cols et vallées repensés pour le jeu |

Décisions déjà prises qui restent : relief des batailles **réel** (site historique), montagnes
infranchissables hors des cols (ADR 0353), 0 $ par défaut.

## 1. Point de départ (inventaire du 14/10)

- Carte actuelle : EPSG:3035, 7 168 × 6 144 px à 719 m/px, 443 provinces, 1 656 lieux (443 cités,
  269 villes, 289 châteaux, 116 abbayes, 539 villages), 3 480 hameaux, 175 factions possédantes,
  `data/map` 6,5 Go dont la pyramide de relief fin (4,7 Go, hors dépôt).
- Chaîne de fabrication (`docs/geo.md`, `tools/cent_ans_tools/geo/build.py`) : une étape 0
  (ETOPO, Natural Earth → `heightmap.png`, `land_mask.png`, `rivers.geojson`, `coastline.geojson`,
  `map.json`) puis ≈ 20 étapes qui ne lisent que ce contrat (provinces, graphe des lieux, navgrid,
  splat, lacs, fleuves rendus, colormap, biomes, voies maritimes…), ≈ 10 min hors pyramide.
- Projection : un point canonique Python (`geo/project.py`, `MapGrid`), mais des copies dans
  `decor_bake.py`, `hydro_fine.py`, `fine_anchors.py`, `battle_site.py`… et à l'exécution dans
  `data-model/src/map.rs:38` (qui a déjà une branche « autre CRS »), `vegetation/src/stands.rs:88`,
  `fauna_layer.gd:186`.
- Règles qui lisent la vraie latitude/longitude : météo (`weather.rs`), peste (`plague.rs`),
  distances féodales (`feudal.rs`), sécheresse de la végétation, éligibilité des peuplements.
- Références aux identifiants : titres (605 réf. province), factions (317), personnages (255),
  événements (148), naval/commerce/fleuves (≈ 190 réf. lieu) ; ≈ 100 tests Rust et 147 tests
  GDScript nomment des lieux, surtout le cœur France-Angleterre.

## 2. Principe : composer, pas redessiner au crayon

« Réinterpréter à la main » ne veut pas dire peindre 7 000 × 6 000 pixels : un Total War part
d'une carte peinte par des graphistes, nous n'en avons pas. La carte composée est **une
description vectorielle d'auteur** (données versionnées, schémas) **compilée** en le même contrat
que l'étape 0 actuelle ; tout l'aval de la chaîne reste.

La description est produite en trois couches, de la plus automatique à la plus manuelle :

1. **Déformation (cartogramme)** : une fonction lisse et inversible `réel (lon, lat) → carte (x, y)`.
   Densité visée = densité des ≈ 400 lieux retenus, lissée et mêlée à une densité uniforme : les
   lieux se retrouvent **régulièrement espacés à l'écran** (6 à 10 villes visibles à hauteur de
   jeu), ce qui agrandit mécaniquement France, Angleterre, Flandre, Italie et comprime steppe,
   Maghreb, Scandinavie du Nord. Méthode : cartogramme par diffusion (Gastner-Newman / flux
   rapide 2018), puis épingles d'auteur (points de contrôle) pour corriger une forme.
2. **Généralisation** : côtes lissées et simplifiées (une île de moins de N km² disparaît ou
   grossit à une taille minimale lisible), ≈ 40 grands fleuves seulement, élargis, quelques lacs
   majeurs ; chaînes de montagnes réduites à des **lignes de crête d'auteur** (polyligne, largeur,
   hauteur, cols = entailles) tirées des crêtes réelles déformées puis simplifiées.
3. **Retouches d'auteur** (petits fichiers JSON) : positions de lieux recalées sur un rôle de jeu
   (gué, col, embouchure, carrefour), crêtes ajoutées/supprimées, détroits élargis, cols ajoutés.

Le relief n'est plus le MNT réel : une **hauteur procédurale** (plaines plates, collines par
zones, massifs bâtis sur les crêtes avec bruit « ridged », vallées et cols creusés) — c'est ce que
fait la carte de Rome II. Les rasters thématiques (couverture du sol, Köppen, forêts historiques,
zones humides, paysages agraires) sont **rééchantillonnés par la déformation inverse** puis
généralisés (filtres majoritaires) : pas de climat inventé.

## 3. Choix techniques

- **Nouveau CRS « composed »** dans `map.json` : la déformation est stockée (grille de
  déplacement + paramètres) dans `data/map/compose/warp.*` ; une seule fonction de projection
  par langage (Python `MapGrid`, Rust `map.rs`, GDScript `MapProjection`) ; toutes les copies
  actuelles y sont ramenées. À l'exécution, on évite de projeter : tout ce qui se pose sur la
  carte est précuit en px par la chaîne.
- **Les règles gardent la vraie géographie quand elle a un sens** : latitude réelle pour la
  météo, la peste, la sécheresse (chaque lieu garde `lonlat` réel + reçoit `px` composé) ;
  distances de marche lues sur la carte composée (c'est le jeu), `meters_per_px` nominal fixé
  pour que les vitesses restent en km de jeu.
- **Batailles réelles** : le relief de bataille et l'horizon restent tirés du site réel
  (`battle_site`, `horizon`) par le `lonlat` réel du lieu.
- **Pyramide de relief fin E1-E4 abandonnée** pour la carte composée (elle décrit le relief réel) :
  détail rapproché procédural dans le shader (bruit, roche, strates) ; LiDAR des 34 zones de
  détail abandonné. Les maquettes de villes Meshy et les modèles restent.
- **Identifiants conservés** quand une province ou un lieu survit (`prov_ile_de_france`,
  `set_paris`…) ; une **table de correspondance** ancien → nouveau (`data/map/compose/id_map.json`)
  pilote un outil de réécriture automatique des titres, factions, personnages, événements, codex,
  naval, commerce, fleuves. Les provinces fusionnées gardent l'identifiant de la plus importante.
- **Sauvegardes incompatibles** : nouvelle version de format, pas de migration.
- **Développement sur une branche longue** (`cc-carte-composee`, worktree dédié), rebasée
  régulièrement ; la chaîne étant générée, un conflit se règle en régénérant. Fusion dans main
  seulement quand la partie complète tourne (smoke, seed bots) et après regard du joueur.

## 4. Lots

| Lot | Contenu | Sortie | Porte |
|---|---|---|---|
| **C0 — Maquette 2D** | Sélection des ≈ 400 lieux (score `settlement-cap` + quotas d'espacement + lieux protégés), proposition de fusion en ≈ 180 provinces, cartogramme par diffusion, côtes/fleuves généralisés ; **planche 2D** (parchemin simple : côtes, crêtes, fleuves, lieux, frontières) à deux niveaux de déformation | `tools/cent_ans_tools/compose/` (prototype), planches hors dépôt | **le joueur valide la forme sur image** |
| C1 — Compilateur | `cent-ans geo compose` : déformation stockée, `land_mask`, côtes, fleuves, lacs, `map.json` CRS composé ; projection unique Python/Rust/GDScript | contrat étape 0 | tests de contrat |
| C2 — Relief composé | crêtes d'auteur → hauteur procédurale, cols creusés, plaines plates ; `height/`, `heightmap_render`, ombrage ; profil RE réaccordé ; pyramide coupée, détail proche procédural | relief | `zg8`, `sz1` adaptés, 1 planche |
| C3 — Provinces, lieux, factions | table d'identifiants, fusion des provinces, ≈ 400 lieux, hameaux sélectionnés, factions de l'Oural/steppe retirées avec leurs données ; réécriture automatique des références | `data/provinces`, `data/settlements`, `titles`, `factions`… | `GameData::load` sans erreur, `title_check` |
| C4 — Couches dérivées | rasters rééchantillonnés (couverture, biomes, forêts, zones humides, agraire), colormap, navgrid + massifs, fleuves rendus, voies maritimes, routes calculées | `data/map/*` | tests tools |
| C5 — Exécution | lecture du CRS composé, latitude réelle pour les règles, retrait des copies de projection, détail proche | `core/`, `game/` | `cargo test`, smoke |
| C6 — Tests et équilibre | tests à identifiants/coordonnées, `starting_fit`, `points_per_step`, km des fleuves, objectifs de victoire, nouvelles bandes seed bots (passe longue) | | seed bots verts |
| C7 — Finition | regard du joueur, lisibilité (étiquettes, signes), perf sur machine calme | | joueur |

C0 est court (une session) et décide de tout le reste ; C1 → C7 s'enchaînent ensuite sans porte
intermédiaire hors C7. Ordre de grandeur : 8 à 12 jours-agent, 0 $.

## 5. Ce qu'on perd, ce qu'on garde

- Perdu : fidélité kilométrique, relief fin réel en vue rapprochée (pyramide, LiDAR, fleuves
  fins TOPAGE), ≈ 1 250 lieux mineurs (deviennent décor/hameaux ou disparaissent), ≈ 20 factions
  de l'Est, compatibilité des sauvegardes, une partie des réglages calés sur l'ancienne échelle.
- Gardé : toutes les règles (`core/`), les maquettes de villes, les textures régionales, la
  climatologie et la couverture du sol (rééchantillonnées), le relief réel des batailles, les
  identifiants du cœur France-Angleterre, l'historique du dépôt (l'ancienne carte reste dans git).

## 6. Risques

- Forme « pas assez Total War » après tout ce travail → C0 tranche sur image avant tout.
- Branche longue face aux sessions parallèles sur la carte (GR, CV…) : prévenir avant de lancer
  d'autres chantiers carte ; les données générées se régénèrent après rebase.
- Tests GDScript à coordonnées px (62 fichiers) : les recaler par la déformation ou les réécrire.
- Équilibre : la carte change la durée des conquêtes ; bandes seed bots à recalibrer (passe longue).
