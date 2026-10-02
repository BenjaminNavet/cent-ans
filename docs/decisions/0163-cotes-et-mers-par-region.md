# ADR 0163 — Côtes et mers par région : polygones dans `data/map/`, matières procédurales

Date : 2026-10-02. Lot TB5 (`docs/design/2026-10-02-campagne-tob.md` § 3).

## Contexte

- Le lot TB5 demande des falaises de craie (Douvres, pays de Caux) et de granite (Bretagne,
  Cornouailles) « choisies par la pente et la région », des plages de sable et de galets, des mers
  différenciées (mer du Nord et Manche sombres, Atlantique houleux, Méditerranée claire) et un
  ressac animé. Le chantier TB se fait sans service payant (ADR 0152).
- La carte fait 719 m par pixel : une falaise réelle est bien plus étroite qu'un pixel carte. La
  bande côtière est donc un signe de carte élargi, pas une géométrie.
- Le shader de terrain porte déjà 24 échantillonneurs ; TB4 et TB6 travaillent en parallèle sur
  `satellite_ground` / `hb_ground` et sur l'atmosphère.
- La mer suit déjà la saison (TB1, ADR 0150) : à conserver.

## Décision

1. **La région donne la géologie, la pente choisit falaise ou plage.** `data/map/coast_types.json`
   liste des régions (polygones en pixels carte) avec leur roche (`chalk`, `granite`, `rock`) et
   leur plage (`sand`, `shingle`). La pente n'est pas une donnée : le shader lit l'altitude à
   `cliff.probe_px` de la côte vers l'intérieur (plage sous `min_m`, falaise au-dessus de `max_m`).
   Les seuils (36 / 52 m sur le relief de rendu) séparent le cordon de dunes des Landes (≤ 36 m)
   des falaises du pays de Caux (≥ 46 m). La ville de Douvres, dans la vallée de la Dour, sort en
   plage ; les falaises commencent de part et d'autre (South Foreland), comme en vrai.
2. **Polygones en pixels carte, cuits au chargement.** Pas de projection EPSG:3035 en GDScript :
   les polygones sont en pixels carte comme les autres fichiers `*_px` ; les tests pytest
   projettent des lieux de référence (lon/lat) et vérifient la région obtenue. `CoastLook` et
   `SeaBasins` remplissent une petite texture au chargement (448 × 384 et 224 × 192, balayage par
   lignes, quelques millisecondes) : pas de PNG cuit à tenir à jour ni de commande d'outil en plus.
3. **Matières procédurales.** Stries, strates et grain viennent du bruit du shader ; les couleurs
   sont dans les données. Aucune texture ajoutée (ni CC0, ni fal.ai), un seul échantillonneur de
   plus dans le terrain (`coast_types`). Des textures de roche pourront remplacer le grain plus
   tard sans changer les données.
4. **Un seul crochet dans le terrain.** `coast_band.gdshaderinc` (bande côtière) et
   `coast_common.gdshaderinc` (géologie, règle de pente, ressac) ; `terrain.gdshader` ne reçoit
   qu'un `#include` et un appel `coast_band(...)` après l'habillage du sol.
5. **Mers par bassin : poids dans une texture, réglages en tableaux.** `data/map/sea_basins.json`
   décrit quatre bassins au plus (un canal chacun, limites fondues) ; hors bassin, la mer par
   défaut garde le rendu d'avant (mer Noire, Caspienne). Les réglages (teinte, clarté, clapot,
   longue houle, écume, moutons, part du gris de saison) sont des tableaux d'uniformes : réglables
   sans recuire la texture. La longue houle a une longueur d'onde fixe : la faire varier par
   bassin déphaserait les crêtes là où les poids changent.
6. **La saison passe après le bassin.** La teinte du bassin s'applique à la couleur de
   profondeur, puis le gris de saison TB1 (dont la part est modulée par `grey_scale` : la
   Méditerranée reste bleue l'hiver) et la teinte de saison.
7. **Le ressac est une seule fonction pour la mer et la plage.** `coast_swash` (include commun)
   est appelée par `water.gdshader` côté eau et par la bande côtière côté plage : la lame est
   continue au trait de côte même quand le plan d'eau passe sous le terrain. Elle monte peu au
   pied des falaises et se fige en liseré au dézoom (pas de scintillement).

## Conséquences

- Ajouter une côte typée ou déplacer une limite de bassin se fait dans `data/map/` ; les lieux de
  référence des tests (`tools/tests/test_coast_types.py`, `test_sea_basins.py`) sont à compléter.
- Quatre bassins au plus et trois roches, deux plages : au-delà, il faut une seconde texture ou
  d'autres canaux.
- La bande côtière est fine (≈ 1 px carte) : lisible de près et en vue moyenne, réduite à un trait
  clair au dézoom (`band.far_keep`). Une falaise tournée à l'opposé de la caméra est cachée par
  le relief.
- La mer peinte sur le terrain (maillage au-dessus du plan d'eau) et le fond marin ne reçoivent
  pas la teinte du bassin.
- Deux nouvelles classes globales (`CoastLook`, `SeaBasins`) : relancer
  `godot --headless --path game --import` après fusion.
