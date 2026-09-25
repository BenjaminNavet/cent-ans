# ADR 0047 — Ville de siège dense et mobilier de rue solide

Date : 2026-09-25. Statut : accepté. Suite de l'ADR 0021 (kit de bâtiments, BR1/BR2) ; touche aux
sièges (ADR 0023, 0026) et aux incendies (S2).

## Contexte

Deux limites de BR2 relevées par le joueur :
1. **Mobilier décoratif** : étals, charrettes, tonneaux, bûches étaient posés au hasard en
   GDScript ; le cœur n'en savait rien, une troupe traversait un étal. Plus largement, aucune
   figurine n'évitait les bâtiments : le régiment est un point pour les règles (la grille A* garde
   son centre hors des disques des maisons), mais ses figurines débordaient dans les façades.
2. **Ville aérée** : la ville générique n'avait que 2 anneaux d'îlots (≈ 21 disques de 9 m, rues de
   12 m) ; les villes emblématiques un treillis lâche (≈ 30 disques).

## Décision

- **Données** : `data/rules/siege_town.json` (schéma `siege_town_rules.schema.json`) porte la
  disposition (îlots, anneaux, rues, ruelles, chemin de ronde, église, faubourgs), les emprises du
  mobilier (valeurs du manifeste des modèles du kit), le marché et la marge des figurines.
  Plus aucune constante de disposition dans `siege.rs`.
- **Îlots = rectangles orientés** (`House { length, depth, yaw, rows, church }`, `town::Footprint`) :
  ce que le rendu dessine, ce que contourne le cheminement (grille A*, `segment_clear`,
  `house_block`) et ce dont les figurines sont repoussées. Le disque (même aire) reste pour
  l'allumage ; la propagation du feu se mesure entre rectangles.
- **Ville générique dense** : maisons d'une rangée adossées au chemin de ronde (8 m libres au pied
  du rempart), puis 4 anneaux d'îlots de deux rangées dos à dos entre les rues principales
  (11 m) qui vont de la place à chaque pan de mur, ruelles de 3,5 m (fermées aux régiments,
  ouvertes aux figurines). **Villes emblématiques** : rangées le long du rempart, des rues du plan
  et de la voie porte-place (11 m), puis un treillis d'îlots dans le reste. L'église est choisie
  par le cœur (îlot le plus proche du fond de la ville où elle tient).
- **Mobilier généré par le cœur** (`props.rs`) de façon déterministe par hachage de l'indice et de
  la position de la maison (le `BattleRng` n'est pas consommé) : étals côté place, tonneaux,
  charrettes, bûches côté rue, dos à la façade ; place du marché avec puits et groupes d'étals en
  couronne qui laissent un passage devant chaque rue ; faubourgs ; village de bataille (dérivé
  des maisons de `field.village`, sans toucher à `site.rs`/`field.rs`). Le mobilier d'une maison
  brûlée disparaît.
- **Collision** : le mobilier de façade est dans la marge de 3 m que les régiments gardent autour
  des îlots, il ne ferme donc aucune rue ; le mobilier de la place (sauf le puits) bloque le
  cheminement avec une marge d'1 m. **Figurines** : `soldier_poses` repousse chaque figurine hors
  des rectangles proches (îlots debout, mobilier, maisons du village) vers la sortie la plus proche
  qui ne tombe dans aucun autre, marge 0,3 m ; les obstacles sont d'abord filtrés par le cercle
  englobant du régiment (aucun coût sans obstacle proche).
- **Incendie** : la ville dense brûlait entièrement en 10 min avec l'ancien réglage ; portée de
  propagation 30 → 15 m (entre rectangles), chance 0,06 → 0,025 par période.
- **Rendu** (`battle_siege.gd`, `battle_village.gd`) : pose les maisons sur les emprises du cœur
  et le mobilier à la transformation du cœur (`BuildingKit.prop_transform`) ; plus de placement
  aléatoire du mobilier.

## Mesures

Voir `docs/wip/br3-ville-dense-mobilier.md` (tableaux avant/après : îlots, incendie, assauts,
coût des figurines).

## Conséquences

- Les rues étroites canalisent les assauts : les régiments passent par les rues principales, le
  chemin de ronde et la place ; les ruelles ne servent qu'aux figurines.
- Plus d'obstacles pour la grille A* (≈ 60-85 îlots) : le coût reste borné (évaluation paresseuse
  des cellules, test du cercle englobant d'abord).
- Les sauvegardes anciennes (maisons sans emprise) retombent sur un carré autour du disque.
