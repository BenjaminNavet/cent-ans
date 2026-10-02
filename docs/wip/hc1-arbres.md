# HC1 — arbres généralisés sur la carte de campagne

Worktree `../gp-hc1`, branche `feat/hc1` (issue de `feat/hc`). ADR 0161. Aucun changement Rust.

## État
- [x] Squelette : `map.tree_style` (JSON + schéma), `MapPropScale.tree_style()` /
      `--tree-style=`, champs `generalised_*` du `.tres`, test, cette note.
- [x] Rendu généralisé dans `vegetation.gd` : échelle `campaign_prop_scale` =
      `generalised_tree_height / generalised_reference_height`, pas `generalised_spacing`, portée
      `generalised_max_distance`, disque de dessin centré en avant du point visé, paliers et ombres
      par distance à la caméra, forêt dense et cartes proches éteintes.
- [x] Dégagements `tree_clearance.gd` : houppiers hors fleuves, lacs, mer, routes principales
      (filtre des tampons après le semis natif, en tâche de fond), houppiers élargis.
- [x] `hc_forest_test.gd` OK, `gc_maquettes_test.gd` OK ; planche `hc_shots.gd` (2 lues sur 3) ;
      sonde `hc_density_probe.gd` (densités par milieu sans image).
- [x] Tests 1:1 existants épinglés sur le style `real` (vt3, sz4b, settlements_render, fc2,
      ga3_l2, sz6) : à relancer.
- [x] smoke OK ; 7 tests de végétation existants OK (vt3, sz4b, settlements_render, fc2, ga3_l2,
      sz6, hb4).
- [x] Mesures (`hc_shots.gd --bench`, 1280 × 800, machine très chargée le 02/10 : charge moyenne
      28-136, autres fenêtres Godot sur le GPU, image de base 17-25 ms au lieu de ≈ 7) :
      Paris d = 90 : 96 173 instances soumises, +7,9 ms ; d = 300 : 199 986, +5,0 ms ; d = 700 :
      236 367, +8,2 ms. Maillages bas à la place des imposteurs (`--no-fc2`) : +11,4 / +9,4 /
      +15,7 ms (les imposteurs restent le palier le moins cher). Premier banc (plafonné à 145
      images/s, avant réglages) : image complète ≤ 6,9 ms à d = 300 avec 323 610 instances.
      **À refaire sur machine calme** pour juger la cible (< 2 ms à d = 300).
- [x] Dernière planche lue : 3 sur 3 (`hc_board_3.jpg`, hors dépôt).

## Réglages (map_prop_scale.tres)
Hauteur 0,8 (échelle 0,571 ; hauteurs monde p10 0,44 / médiane 0,86 / p90 1,12), pas 0,9 px,
houppiers × 1,25, variation ± 25 %, portée 900 (fondu sur le dernier quart), rayon de dessin
120 + 1,6 × d (plafond 900) centré 0,5 d en avant du point visé, ombres sous d = 70, part gardée
150 / d bornée à 0,35 (arbres grossis de 1/√part au-delà de d = 150), gains hors forêt : isolés
× 8, vergers × 1,3, ripisylves × 1,3, bosquets (seuils 0,2 / 0,36, cœur 0,45), bocage × 6.
Sonde à Paris (rayon 120 px) : 0,98 arbre/px² en forêt, ≈ 0,14 hors forêt.

## Choix
- **Haies** (consigne GC, 02/10) : en style généralisé, pas de buissons alignés sur la trame du
  parcellaire (`VegetationFields`), que GC réduit à des enclos d'environ 1,4 px (`field_scale`) :
  des arbres d'environ 0,8 unité ne peuvent pas les dessiner. Les emplacements `Kind.HEDGE` du
  semis sont vidés (`VegetationTileJob.drop_hedges`) ; le bocage se lit par des arbres épars plus
  nombreux (rôle « isolé », poids `generalised_hedge_boost` sur le canal de haies du masque) et
  les rares arbres de haie du semis (2,5 % des points de haie). Style `real` inchangé.
- **Emprises des lieux** : prises telles que `SettlementLayer.vegetation_exclusions()` les renvoie,
  élargies du rayon nominal d'un houppier (`MapPropScale.generalised_crown_radius()`), sans rayon
  en dur.
- **Taille** : hauteur monde en réglage `.tres` (`generalised_tree_height`, départ 0,7), à tenir
  nettement sous la largeur d'un village (3,2 unités, GC2).
- Aucune donnée ni règle touchée ; `tree_species.json` (compilé) non modifié : les probabilités
  hors forêt sont multipliées par les gains `generalised_*_gain` du `.tres`.

## Prochaine étape
HC3 (session principale) : relecture visuelle, banc sur machine calme, fusion dans `feat/hc`.

## Points ouverts
- Éclaircissement au dézoom : au-delà de d = 150 la taille n'est plus strictement constante
  (× 1,41 à 300, × 1,69 à partir de 430) ; `generalised_far_density = 1` la rend constante, au
  prix du nombre d'instances.
- Grille grossière des masques (`VegetationTileJob._sample_coarse`, GDScript) : 300-600 ms par
  tuile en tâche de fond ; une vue à d = 300 sème 25 tuiles (3-4 s pour remplir après un saut).
- Préréglages de qualité : `veg_max_distance` (500-1100) plafonne la portée généralisée.
