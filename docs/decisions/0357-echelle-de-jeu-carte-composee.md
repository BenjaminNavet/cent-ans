# 0357 — Échelle de jeu de la carte composée

Statut : accepté (14/10, lot CC C6). Complète l'ADR 0355.

## Contexte
Le cartogramme de la carte composée grossit l'Ouest et comprime le reste : la distance en pixels
x `meters_per_px` nominal (719) vaut 1,2 à 1,8 fois la distance réelle selon la région (médiane
des arêtes terrestres : 1,78 France/Angleterre, 1,52 Europe centrale, 1,45 Italie, 1,32 Ibérie
ouest, 1,26 Est, 1,18 Ibérie est). `meters_per_px` sert aussi de métrologie du rendu (figures 1:1,
champs, textures, grille de navigation, 54 usages côté Godot) : le changer déplacerait tout le
monde 3D.

## Décision
- **`meters_per_px` reste nominal (719).** Le km de jeu est `px x 0,719` : une notion de jeu, pas
  une distance géographique ; une seule source, aucune régénération de la chaîne de carte.
- Les seuils exprimés en km de jeu sont recalés par **mesure croisée avec main**, pas par une
  conversion globale :
  - `points_per_step` 70 -> **155**. Deux mesures : (1) parité des durées de campagne : rapport
    médian des coûts de graphe entre les mêmes 20 paires de villes (Paris-Rouen, Paris-Lyon,
    Bordeaux-Toulouse...) cc / main = 1,75, soit 122 ; (2) portée de voisinage : les places sont deux
    fois moins denses, et à 122 aucune place d'une province voisine de Paris n'est à portée d'une
    saison (Orléans 218 km de grille, Château-Gaillard 225 km, pour 1,5 x points_per_step de
    plaine). 155 atteint les deux ; le prix est une campagne environ un quart plus rapide que sur
    main (trajet cité à cité entre provinces voisines : médiane 1,1 saison contre 1,2). Les rayons de
    repli suivent la densité (voir ci-dessous) ;
  - `max_muster_km` 200 -> **350** (x 1,75, Ouest) ; `character_travel.km_per_turn` 150 -> **260** ;
  - rayons de repli `friendly_radius_steps` 2 -> **3**, `neutral_radius_steps` 1 -> **2** : le graphe
    compte deux fois moins de places par unité de longueur (400 lieux contre 1656), le rayon ami garde
    environ deux arêtes médianes (140 km pour une arête de 58 sur main) ;
  - `data/movement/rivers.json` : abscisses `km` mises à l'échelle de jeu fleuve par fleuve (rapport
    distance de jeu / distance réelle entre les extrémités) ; script rejouable
    `cent-ans compose c6 rivers`.
- Les seuils **locaux** (zones de contrôle, vision, engagement, rayons de rencontre, cellule de la
  grille 1,44 km) restent inchangés : ils sont attachés à la taille d'une cellule et au rendu, pas
  à la géographie.

## Conséquences
- Un trajet vaut en jeu 1,2 à 1,8 fois son équivalent réel : l'Ouest dispose de plus de place
  qu'à l'échelle réelle, c'est le but du cartogramme. Les infobulles affichent des km de jeu.
- Toute nouvelle valeur en km de jeu se justifie par un rapport à main (mesure), non par la
  longueur réelle.
