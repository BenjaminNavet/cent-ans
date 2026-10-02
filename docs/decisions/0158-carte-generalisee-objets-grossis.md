# ADR 0158 — Carte généralisée : positions vraies, objets grossis d'un facteur constant

Date : 2026-10-02. Statut : acceptée (décisions du joueur du 02/10, captures du lot GC1).
Chantier GC, suivi `docs/wip/gc-carte-generalisee.md`.

## Contexte

Depuis VT (ADR 0138, 0144), villes, hameaux, moulins et arbres sont à l'échelle 1:1 (1 unité
≈ 719 m). Le reste de la carte ne l'est pas :

- parcellaire HB3 (ADR 0143) grossi ×3 (`hb_cell_scale`) sur des tailles de base de 170 à 900 m :
  un champ affiché fait 0,7 à 6,7 km, une haie ≈ 50 m de large ;
- pion d'armée à taille écran constante ; son camp de siège fait 2,5 km au plus près, 13 km à la
  distance 100 ;
- figurants FK gardés lisibles (`figure_min_view_fraction`).

En face, une ville ordinaire fait 0,3 à 1 km et Paris ≈ 2,5 km : un champ vaut une ville. À hauteur
de jeu (distance 200), une ville de 700 m fait ≈ 5 px ; elle ne se repère que par son écu.

Le joueur (02/10) : « il y a un problème entre la taille des champs et les villes en 1:1 », puis
« est-ce qu'on peut rendre la carte moins fine ? faire Paris, la Seine, etc. agrandis et pourtant
avoir une réalité géographique relative ? », enfin « je sacrifie la vue rapprochée, carte lisible à
hauteur de jeu ». Cela revient sur la demande du 30/09 qui avait fondé l'ADR 0138.

## Décision

La carte de campagne devient une **carte généralisée**, à la manière des jeux de grande stratégie :

1. **Reste vrai** : le relief, les côtes, le tracé des fleuves et des routes, la position du centre
   de chaque lieu, donc les distances et les directions entre lieux.
2. **Est grossi** : l'emprise des villes et des hameaux, la largeur des fleuves et des routes, les
   moulins, les arbres, les camps. Le grossissement est **constant quelle que soit la distance de
   caméra** : rien ne « respire » au zoom (défaut de SZ4/SZ4b, où l'exagération allait de 1 à 125).
3. **Villes stylisées, pas un plan réel grossi.** Essai du lot GC1 : le plan réel grossi ×8 ou ×14
   reste une tache brune à moyenne distance (centaines de petites maisons sombres). Le joueur
   retient des **maquettes stylisées** : peu d'éléments, gros et clairs, silhouette lisible
   (enceinte, cathédrale, tours). Une maquette par type de lieu (ville, bourg, château, abbaye,
   village) et par famille d'architecture (Ouest, Méditerranée, Byzance, Rus', Islam, steppe),
   à **taille monde constante par type** ; les villes emblématiques gardent leur maquette propre
   (`LandmarkModel`). Tailles, familles et portées vivent dans `data/art/town_maquettes.json`.
4. **Collisions** : un lieu dont la maquette toucherait celle d'un lieu plus important est réduit,
   jusqu'à un plancher ; son nom reste.
5. **Ville et fleuve** : la ville grossie est une maquette posée sur le vrai terrain ; son fleuve
   intérieur est le fleuve élargi de la carte, pas celui du plan. Pas de déformation locale de
   l'espace autour des villes.
6. **Plancher de caméra relevé** : les paliers « vallée » et « site » disparaissent ; on ne descend
   plus assez bas pour voir une maison de 100 m comme telle.
7. **Parcellaire** : la taille des champs se règle par rapport aux villes grossies (un champ reste
   nettement plus petit qu'une ville ordinaire).
8. **Armées et camps** : taille monde fixe, de l'ordre d'une ville (ADR 0156, lot SA) ; le camp
   suit la même règle. Ce chantier ne touche pas à l'échelle des pions.

## Conséquences

- Remplace l'ADR 0138 (villes 1:1 à toutes les hauteurs) et l'ADR 0144 pour la carte de campagne.
  Les villes 1:1 (plans ZG6/VH, lointain F1/F2) quittent la carte de campagne ; les maquettes du
  kit (lot C6) et les `LandmarkModel` (L1), retirés par VT, y reviennent à taille constante.
- Le relief fin autour des villes (E4, chantier RF, ADR 0157) perd l'essentiel de son intérêt ; le
  relief E3 partout reste utile, puisque le relief reste vrai.
- Le parcellaire de près à l'échelle réelle (ZG5b), l'herbe et les figurants à 1:1 ne sont plus
  atteints par la caméra : à retirer ou à laisser dormir selon leur coût.
- Les incendies ne sont plus la seule exception grossie : `MapPropScale` redevient la table commune
  des grossissements, sans dépendance à la distance.
- Les batailles ne sont pas concernées (terrain en mètres).
