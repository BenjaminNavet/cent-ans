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

Sonde `core/crates/sim-battle/tests/br3_assault_probe.rs` (20 graines, deux IA, brèche 40 %) :

| Ville | Îlots | Victoires assaillant | Durée médiane (s) | Maisons brûlées |
|---|---|---|---|---|
| générique | 21 → 59 | 17/20 → 17/20 | 332 → 349 | 4.5 → 6.2 |
| Paris | 32 → 61 | 10/20 → 5/20 | 733 → 706 | 16.5 → 20.9 |
| Rouen | 36 → 78 | 0/20 → 1/20 | 305 → 307 | 4.8 → 6.4 |

Paris perd la moitié de ses victoires d'assaillant : le résultat y suit surtout l'incendie ; élargir
les rues, le chemin de ronde ou la rue de la place n'y change rien de mesurable (détail dans
`docs/wip/br3-ville-dense-mobilier.md`). Point laissé à l'équilibre des sièges.

Figurines : +178 µs par image pour 10 423 figurines en plein assaut (57 régiments).

## Conséquences

- Les rues étroites canalisent les assauts : les régiments passent par les rues principales, le
  chemin de ronde et la place ; les ruelles ne servent qu'aux figurines.
- Plus d'obstacles pour la grille A* (≈ 60-85 îlots) : le coût reste borné (évaluation paresseuse
  des cellules, test du cercle englobant d'abord).
- Les sauvegardes anciennes (maisons sans emprise) retombent sur un carré autour du disque.

## Addendum BR3b (2026-09-25) — équilibre du siège de Paris

Wip : `docs/wip/br3b-equilibre-paris.md` (diagnostic, balayages, tableaux complets).

**Constat.** Le 5/20 de Paris mesuré par BR3 n'existe plus sur main : SG3 (PV des ouvrages dans
`siege_works.json`) l'a effacé (en remettant les PV d'avant SG3, la sonde redonne exactement les
chiffres de BR3). Sur main, l'assaillant gagne 17/20 à Paris ; SG3 sans BR3 donnait 11/20. BR3 a
rendu Paris trop facile, par un seul mécanisme : les engins visent le pan de mur tenu par les
arbalétriers près de la porte ; leurs projectiles incendiaires (dépassement de 18 m) allument la
rangée de maisons adossée au rempart (qui n'existait pas avant BR3) ; la chaleur fait fuir les
arbalétriers, qui ne tuent plus l'équipage du bélier, et la porte tombe vers 250 s. Aucun régiment
bloqué, chemins A* normaux ; le moral perdu à la chaleur par l'assaillant est faible sur main.

**Décision.**
- `heat.wall_walk_factor` = 0,3 (`data/rules/siege_fire.json`) : sur le chemin de ronde, au-dessus
  de la rue et derrière le parapet, un régiment ne reçoit que 30 % de la chaleur des maisons en
  feu. La chaleur des rues est inchangée.
- Défaut d'IA révélé et corrigé (`plan_siege_attack`) : un régiment de l'assaillant déjà passé
  par-dessus le mur, sans ouverture, était renvoyé à son point d'échelle (là où il se tenait) et y
  restait jusqu'à la nuit ; il marche maintenant vers la place.

**Mesures** (sonde BR3, 20 graines, brèche 40 %, fortification 2) :

| Ville | Victoires assaillant avant → après | Durée médiane (s) | Pertes assaillant | Pertes garnison | Maisons brûlées |
|---|---|---|---|---|---|
| générique | 17 → 18/20 | 438 → 441 | 55 → 56 | 197 → 197 | 10.9 → 10.4 |
| Paris | 17 → 12/20 | 501 → 560 | 85 → 78 | 194 → 166 | 6.7 → 8.9 |
| Rouen | 2 → 0/20 | 307 → 305 | 91 → 70 | 148 → 157 | 5.6 → 4.0 |
| Avignon | 18 → 18/20 | 452 → 452 | 154 → 154 | 175 → 175 | 5.0 → 5.0 |
| Bordeaux | 18 → 12/20 | 463 → 470 | 70 → 85 | 196 → 212 | 17.4 → 19.9 |
| Bruges | 19 → 19/20 | 413 → 406 | 71 → 69 | 159 → 167 | 5.0 → 4.7 |
| Calais | 6 → 6/20 | 329 → 334 | 78 → 90 | 152 → 167 | 3.4 → 2.9 |
| Londres | 18 → 16/20 | 501 → 501 | 81 → 77 | 208 → 169 | 4.3 → 5.3 |

Sonde SG3 (armées de la démo, 10 graines) : Paris 10 → 10/10, Avignon 8 → 10, Bruges 10 → 10,
Calais 10 → 10, Rouen 8 → 10/10 (sans le correctif d'IA, Rouen tombait à 4/10 par des nuls à 1800 s).

**Limite.** Bordeaux passe de 18 à 12/20 : la même règle y soulage la garnison du rempart. Aucun
facteur global ne sépare Paris de Bordeaux (0,6 : 15 et 15 ; 0,3 : 12 et 12). Revenir à 0,6 (toutes
les villes à ±3, Paris 15/20) ou à 1,0 (Paris 17/20) est un changement d'une ligne de données.
