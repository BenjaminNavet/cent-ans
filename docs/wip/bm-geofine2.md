# bm-geofine2 : couche géographique fine dans le cadre composé

## État (16/10)
- Outil `cent-ans geo compose-fine` (`tools/cent_ans_tools/compose/fine_geo.py`, tests
  `tools/tests/test_compose_fine_geo.py`). Il ne refait pas `geo hydro-fine` / `geo anchors-fine`
  (chaînes bâties sur le cadre EPSG:3035 réel, sources nationales, recalage de plusieurs heures) : il
  **transporte** leurs sorties réelles dans le cadre composé avec la déformation de la composition
  (`ComposedWarp.forward_px`, découpe à l'emprise − 2 px et densification tous les 8 px réels, comme
  `geo.project.composed_geometry` pour fleuves et côtes). Pas de nouvelle projection.
  - Tuiles CAFV fleuves (`hydro_fine`) et routes drapées (`roads_fine`) : chaque polyligne décodée,
    déformée, recoupée sur les tuiles E2 du cadre composé ; `z` et `w` (mètres) conservés
    (le relief composé est le MNT réel déformé). `features.json` copié tel quel.
  - Colonies : position de règle composée + décalage réel déformé
    (`rule_c + fwd(ancre_r) − fwd(règle_r)`), règles réelles lues à `918b04e52`.
  - Hameaux : dans l'ordre de `hamlets.json` composé (3494, la longueur exigée par Godot) ; ancre réelle
    reprise pour 708 hameaux (même nom, règle réelle déformée à ≤ 3 px) ; les autres (hameaux
    régénérés par CC) restent à leur position de règle, altitude lue sur `heightmap.png`.
  - Passages (ordre de `crossings_px.json` composé, 335) : ancre réelle déformée si même id et à
    ≤ 5 px (113 ; les ids `road_<n>` ne sont pas stables), sinon recalage sur les fleuves fins
    composés (fleuve nommé ≤ 2 km, sinon tout fleuve ≤ 500 m, comme `fit_crossing`) : 174 ;
    48 non recalés (pas de fleuve fin à portée).
- Sorties :
  - manifestes versionnés réécrits : `data/map/rivers_fine.json`, `data/map/fine_anchors.json`
    (Godot et Bevy les lisent dans `data/map`) ;
  - tuiles hors git : `data/map/compose/derived/fine_geo/pyramid/{hydro_fine,roads_fine}/E2/*.bin`
    + `hydro_fine/features.json` (91 Mo, ≈ 30 s). `data/map/compose/derived/` est déjà ignoré.
    Écart à la consigne : pas sous `game/assets` (le jeu lit les tuiles sous `<racine relief>/pyramid/`,
    jamais sous `game/assets`).

## Commande exacte
```
cd tools && uv run cent-ans geo compose-fine
# options : --real-root ~/dev/cent-ans-raw/pyramid-reelle (défaut) --out <dossier> (défaut ci-dessus)
```
Source : `~/dev/cent-ans-raw/pyramid-reelle/{hydro_fine,roads_fine}` (tuiles réelles du 02/10, qui
correspondent aux manifestes de `918b04e52`, relus par `git show` : la commande est rejouable).

## Vérification
- Ancrages ↔ colonies composées : médiane 0,0 px, p95 0,26, max 0,71 (400/400) ; Paris (1651,3 ; 2730,8).
- Hameaux : médiane 0 px ; passages : médiane 0,66 px, p95 3,3 px de leur position de règle.
- `z` des fleuves − relief composé (`heightmap.png` 719 m/px, échantillon) : médiane −11 m, p5 −66,
  p95 +43 (relief grossier).
- pytest : nouveaux tests + test_schemas, test_hydro_fine, test_river_render, test_relief_hosting,
  test_zg7a_widths (1262) verts.
- Dump Bevy (`CENT_ANS_FINE_GEO_ROOT` sur la sortie, `CENT_ANS_CAMERA=xz:1651,2731,60`, état Campaign,
  600 images, hors écran, muet) : `campaign_fine_geo` `enabled: true`, 5244 tuiles fleuves / 5245 routes
  indexées, ancrages 400 / 3494 / 335, 10 tuiles voulues = 10 maillées et visibles, 9858 points de
  fleuve, 26 663 de route, zone rayon 78 poids 1, maillage 1,7 ms en moyenne ; ponts fins 287 ancrages,
  278 reprofilés.

## Publication (non faite, aucune release)
- Pour que le jeu s'en serve sans crochet : copier
  `data/map/compose/derived/fine_geo/pyramid/{hydro_fine,roads_fine}` dans
  `~/dev/cent-ans-raw/compose/pyramid/` (cible du lien `data/map/pyramid`), puis les inclure dans le
  paquet de relief (`geo relief-update`, ADR 0149) quand le joueur l'autorise.
- En attendant : `CENT_ANS_FINE_GEO_ROOT=<worktree>/data/map/compose/derived/fine_geo` (Bevy).

## Points ouverts
- Ancres de colonies / hameaux non réajustées au relief composé (seulement transportées) ; un vrai
  `anchors-fine` composé demanderait de porter `fine_anchors.Frame` sur `ComposedGrid`.
- Routes fines = routes historiques réelles déformées : `roads.geojson` composé peut différer (index
  `feature` = carte réelle).
- Rivières de vallée ajoutées par CC (`compose valley-rivers`) et Nil/Euphrate CS2 : absents du réseau
  fin réel ; contraintes d'auteur (crêtes, cols) peuvent décaler `z` localement.
