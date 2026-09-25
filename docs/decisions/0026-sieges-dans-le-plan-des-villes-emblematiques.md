# ADR 0026 — Sièges dans le plan des villes emblématiques

Date : 2026-09-25. Statut : accepté. Lot L3 (suite de l'ADR 0015, villes emblématiques).

## Contexte

Jusqu'à L2, un siège de Paris se jouait dans l'octogone générique de `SiegeWorks::generate`
(rayon 150 m, une porte face à l'assaillant), le plan de Paris posé derrière en toile de fond ;
les six villes de L2 n'avaient même pas de toile de fond. Le joueur veut assiéger *Paris* :
son enceinte, ses portes, ses rues. Toute règle vit dans `core/` : la géométrie de siège
(murailles, portes, brèches, maisons-obstacles, cheminement A*) doit donc venir du cœur, pas
d'un décor Godot.

Contraintes : le champ de bataille fait 1 200 × 800 m, la ville est centrée en (600, 560), les
zones de déploiement de l'assaillant sont à 215-280 m devant le front ; l'IA de siège, le
déploiement, le feu et les rendus supposent une porte unique face à −z, un anneau
« étoilé » autour de la place et des pans de mur rectilignes.

## Options

- **A** : décor seul — l'octogone générique reste la règle, Godot habille avec le plan.
  Incohérent : on verrait des murailles que la simulation ignore.
- **B** : enceinte à l'échelle réelle. L'enceinte de Philippe Auguste rive gauche fait ≈ 2,6 km
  de tour : elle ne tient pas sur le champ, et les combats y seraient dilués.
- **C** : **plan ramené à l'échelle du champ** (comme Total War réduit Rome ou Constantinople) :
  la forme de l'enceinte, les portes et les rues viennent des données ; le cœur les ramène au
  rayon moyen de la ville générique et les oriente.

## Décision

**C**.

- **Données** : bloc `siege.battle` de `data/landmarks/<id>.json` (schéma
  `landmark.schema.json`) : enceintes enchaînées (`walls`), porte attaquée (`gate`), rues gardées
  (`streets`), place (`square`, défaut : centroïde) et rayon (`radius_m`, défaut 150 m). Une
  enceinte ouverte sur un fleuve (Paris rive gauche, Londres) est fermée par un quai droit.
- **Chargement** : `data-model` lit `data/landmarks/` (`GameData.landmarks`, sous-ensemble
  `Landmark` : murs, portes, rues, bloc de siège ; les autres champs du plan sont ignorés).
- **Campagne** : `battle_request.rs` joint un `SiegeLayout` (plan en mètres) au `BattleSetup`
  (`siege_layout`, optionnel, sérialisé) quand la colonie assiégée est celle d'un plan. Le champ
  est dans `BattleSetup`, pas dans `SiegeSetup`, pour ne pas casser les littéraux de siège des
  tests existants (SG1 compris).
- **Géométrie** (`sim-battle/src/siege_layout.rs`) : `SiegeWorks::from_layout` — similitude
  plan → champ (le mur de la porte attaquée tourné face à −z, place au centre, échelle = rayon
  moyen visé, bornée à 240 m devant, 200 m derrière, 380 m sur les flancs), simplification
  Douglas-Peucker (4 m), pans d'au plus 55 m avec tours aux jonctions, porte découpée dans le pan
  le plus proche, autres portes en **châtelets** (tours plus larges, nom gardé), maisons sur un
  réseau hexagonal hors des rues, de la place, de l'allée porte → place et de la bande des murs.
  Brèches de campagne : même règle que la ville générique. Aucun tirage aléatoire hors brèches :
  déterministe.
- **Repli** : si l'anneau n'est pas utilisable (place hors de l'anneau, pan qui ne « regarde »
  pas vers l'extérieur depuis la place, aucun mur face à l'assaillant), la ville générique est
  construite (`SiegeWorks::for_battle`).
- **Pont** : `BattleSim.get_siege_landmark()` (`id`, `name`, `gate_name`, `gatehouses`,
  `streets`, `quay`, `plan_scale`) ; `get_siege()` inchangé (les murailles et maisons du plan
  passent par les mêmes pièces que la ville générique, donc par les rendus existants).
- **Rendu** : `LandmarkSiegeTown` (rues pavées du plan) ; `LandmarkBackdrop` pose la toile de
  fond désignée par le `siege_layout` du cœur (plus par la seule province : le Boulonnais a
  pour ville Boulogne). La toile de fond montre ce qui est au-delà du mur du fond : Seine, Cité
  et rive droite (Paris, assiégée par le sud comme en 1360), Tamise, pont et Southwark (Londres),
  Rhône et Villeneuve (Avignon), Seine et Saint-Sever (Rouen), Garonne (Bordeaux), havre et
  Rysbank (Calais), cœur de la ville au nord (Bruges).

## Conséquences

- Nouvelle ville assiégeable « dans son plan » = un bloc `siege.battle` ; test
  `sim-battle/tests/l3.rs` (toutes les villes donnent un anneau utilisable, porte face à −z,
  châtelets, maisons hors des rues, déterminisme, aller-retour JSON du pont).
- Échelle : la ville du plan est ≈ 5 à 9 fois plus petite que nature (`plan_scale`) ; les
  monuments intérieurs ne sont pas posés dans la ville assiégée (ils seraient des jouets à
  cette échelle) ; seule la toile de fond, à l'échelle réelle, les montre.
- Les armées de la campagne n'assiègent pas encore toutes ces villes au départ (Avignon,
  Bruges : pas de guerre en 1337) ; la démo `--siege-province=<prov>` met en scène les autres.
