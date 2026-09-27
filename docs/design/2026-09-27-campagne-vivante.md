# Campagne vivante — postures, rencontres, résultats nuancés, lord à l'échelle TW

Date : 2026-09-27. Statut : conception validée par le joueur, plan d'implémentation à écrire.
Origine : comparaison avec Total War: Warhammer III (annexe A). Premier des quatre axes retenus ;
les trois autres (carte stratégique, batailles historiques, batailles de ville + HUD) auront chacun leur spec.

## 0. Objectif et critères de réussite

WH3 crée beaucoup de situations **différentes** en campagne (embuscade, marche forcée, point d'intérêt,
victoire héroïque) là où Cent Ans ne connaît que « marcher, assiéger, chevaucher ». Objectif : plus de
décisions et de batailles variées sans nouveau moteur, en réutilisant la grille de navigation (M2), la
vision/brouillard (C1), la zone de contrôle, les effets typés (`data-model::common`) et `sim-battle`.

Critères :
- une partie pilote du joueur rencontre au moins une embuscade (subie ou tendue), deux rencontres et
  trois classes de résultat différentes en 20 tours ;
- `century_probe` : part de guerre France-Angleterre toujours dans la bande EQ6 (ADR 0085) ;
- aucune règle en GDScript ; tous les chiffres dans `data/` validés par schéma ;
- sauvegardes existantes chargées sans erreur (champs nouveaux en `serde(default)`).

Hors périmètre : objets/reliques équipables (axe « batailles historiques »), colonies mineures à points de
capture, rendu de la carte en vue haute, recrutement global.

## 1. Postures d'armée

### 1.1 Modèle
`Stance` (`core/crates/sim-campaign/src/state.rs`) passe de `Normal | Raid | Siege` à :
`Normal | Raid | Siege | Ambush | ForcedMarch | Entrenched`. Les variantes existantes gardent leur nom
sérialisé. Nouveau module `posture.rs` : validation du changement (`OrderError` explicite), effets de
début/fin de tour, requêtes (`is_hidden_from`, `ambush_chance`). Nouvel ordre `set_stance` (ou extension
de l'ordre existant s'il y en a un — à vérifier dans `orders.rs` au moment du plan).

Chiffres dans `data/rules/postures.json` + `data/schemas/rules/postures.schema.json` (ou le dossier de
schémas utilisé par les autres fichiers `data/rules/`).

### 1.2 Règles

**Embuscade (`Ambush`)**
- Condition : la cellule de l'armée est d'un biome couvert (forêt, bocage, marais — classes lues dans la
  grille de navigation / carte de biomes existante) et il reste au moins `ambush.min_movement_left`
  (défaut 25 %) du mouvement du tour. Passer en embuscade met `movement_left` à 0.
- Tout déplacement ou attaque fait quitter la posture (retour à `Normal`).
- Invisibilité : `vision.rs` ne révèle pas une armée en embuscade à une faction ennemie, sauf si une armée
  de cette faction est à moins de `ambush.detect_radius_army` (défaut 3 km) ou un espion (agents C6) à
  moins de `ambush.detect_radius_spy` (défaut 8 km). Les alliés et le propriétaire la voient.
- Déclenchement : quand une marche ennemie (`march.rs`) entre dans la zone de contrôle de l'embusqué, la
  marche s'arrête (règle ZdC existante) et on tire la réussite, avec le RNG déterministe de la campagne (`rng.rs`) :
  `chance = base (0,55) + bonus_terrain[biome] + 0,03 × compétence du général (Intrigue ou Cdt, à
  fixer au plan d'après skills.rs) − malus_eclaireurs × part de cavalerie légère de la victime`, bornée à
  [0,10 ; 0,90].
  - Réussite → bataille avec `opening = Ambush { victim }` (§ 1.3).
  - Échec → bataille normale, l'embusqué est révélé ; événement chronique « embuscade éventée ».
- Une armée en embuscade n'attaque pas elle-même d'armée visible, sauf si le joueur quitte la posture.
- Une armée en marche forcée qui tombe dans une embuscade subit un malus supplémentaire sur sa chance de
  détection (`forced_march.ambush_bonus`).

**Marche forcée (`ForcedMarch`)**
- À choisir avant de bouger ce tour (`movement_left` plein) ; donne immédiatement
  `+forced_march.movement_bonus` (défaut +50 %).
- Interdit : attaquer, assiéger, entrer dans une place ennemie, passer en embuscade ce tour.
- Coût : `supply −forced_march.supply_cost` (défaut 10) à la fin du tour.
- Attaquée : l'armée commence la bataille avec `forced_march.start_fatigue` (défaut 60 % de la jauge de
  fatigue de `sim-battle`) et sans phase de déploiement libre (placement automatique).
- Fin de posture automatique au début du tour suivant (retour à `Normal`).

**Camp retranché (`Entrenched`)**
- Condition : l'armée n'a pas bougé ce tour (`movement_left` plein), hors place forte. Met
  `movement_left` à 0 ; reste active tant qu'on ne donne pas d'ordre de mouvement.
- Défense en bataille 3D : ouvrages prêts au début de la bataille (pieux devant les tireurs, palissade
  basse sur le front), en réutilisant les pieux existants et `battle_crest_defence.json`.
- Résolution automatique : `BattleDefense +entrenched.auto_defense` (défaut +15 %) pour le camp retranché.
- Ravitaillement : consommation réduite de `entrenched.supply_saving` (défaut 30 %).

`Raid` (chevauchée) et `Siege` sont inchangés.

### 1.3 Bataille d'embuscade (`sim-battle`)
- `BattleSetup` gagne `opening: BattleOpening` (`Standard` par défaut, `Ambush { victim: SideId }`),
  en `serde(default)` pour ne pas casser les batailles et replays existants.
- Placement de la victime : ses régiments en **colonne de marche** le long de la route ou du chemin qui
  traverse le champ de bataille (réseau de routes de `battle_decor`/B5 ; à défaut, l'axe long de la
  carte), ordre de marche avant-garde → bataille → arrière-garde, formation `column`, pas de phase de
  déploiement pour elle.
- Placement de l'embusqué : zone de déploiement sur un ou deux flancs de la colonne, dans les lisières
  et haies du site ; il garde la phase de déploiement habituelle (F5c).
- Pas de nouvelle règle de combat : le choc vient du placement, des flancs et de la fatigue.
- Résolution automatique d'une embuscade : bonus `ambush.auto_attack` (défaut +25 % `BattleCharge`) à
  l'embusqué.
- Libellé avant-bataille : « Embuscade ! » dans le dialogue de bataille et le bandeau du haut.

## 2. Rencontres sur la carte

### 2.1 Données
`data/encounters/*.json`, schéma `data/schemas/encounter.schema.json`. Une rencontre :
- `id`, `title`, `text` (français, sourcé : champ `sources` comme pour les événements de chronique) ;
- `spawn` : conditions (années, saisons, biomes, régions/provinces, état de guerre, présence de
  compagnies après Brétigny…), poids, durée de vie en saisons (`lifetime: [min, max]`) ;
- `options` (2-3) : libellé, conditions éventuelles (or, piété), liste d'effets typés existants
  (`Stat` + `Add`/`Percent` : or, Prestige, Piety, ArmyMorale, Supply, ArmyExperience…) et au plus une
  issue spéciale :
  - `battle` : troupe neutre générée (liste de `unit_types` + effectifs, faction `fac_rebels`) ; la
    bataille suit le circuit habituel (3D ou auto) ; effets `on_win` / `on_loss` ;
  - `join` : une liste de régiments rejoint l'armée (déserteurs, routiers soudoyés), dans la limite de
    taille d'armée ;
- `ai_weights` par option.

Contenu de départ : environ 12 rencontres (Grandes Compagnies, écorcheurs, pèlerins de Compostelle, péage
de pont, marchands lombards, déserteurs gallois, loups en hiver, reliquaire en procession, village qui
demande protection, gué gardé, moines hospitaliers, héraut d'un tournoi…). Relecture historique comme pour
les événements.

### 2.2 Règles (`core/crates/sim-campaign/src/encounter.rs`)
- `CampaignState.encounter_sites: Vec<EncounterSite>` (`serde(default)`) : id de rencontre, cellule,
  province, saison d'expiration.
- Début de saison : expiration puis apparition jusqu'à `encounters.max_active` (défaut 6, dans
  `data/rules/encounters.json`) ; cellule tirée sur terre praticable de la province choisie, hors place
  forte, loin des autres sites ; RNG déterministe.
- Vision : un site n'est visible que si sa cellule l'est pour la faction (brouillard C1).
- Déclenchement : une armée dont la marche se termine sur la cellule du site ou dans un rayon
  `encounters.trigger_radius` (ordre « aller au site » = déplacement normal vers la cellule).
  - Joueur : `PendingEncounter` (comme les batailles en attente) ; le choix se fait par l'ordre
    `choose_encounter_option` ; ce qui reste en attente au tour suivant prend l'option par défaut.
  - IA : choix pondéré immédiat ; l'IA de campagne peut viser un site proche quand elle n'a pas
    d'objectif plus urgent (poids faible, lot L6).
- Le site disparaît une fois résolu.

## 3. Résultats nuancés

`core/crates/sim-campaign/src/battle_outcome.rs` + `data/rules/battle_outcome.json`.
- Entrée : `BattleResult` (auto) ou issue de `sim-battle` convertie (3D), par camp : effectif engagé,
  pertes, déroute, général tué/capturé.
- Classes, testées dans l'ordre (seuils dans les données) :
  - **Héroïque** : victoire en infériorité ≤ 1:1,5 ;
  - **Décisive** : victoire, ennemi ≥ 70 % de pertes ou anéanti, pertes propres < 25 % ;
  - **À la Pyrrhus** : victoire avec pertes propres ≥ 50 % ;
  - **Victoire** : autres victoires ;
  - **Défaite honorable** : défaite en infligeant des pertes ≥ aux siennes ;
  - **Désastre** : défaite avec ≥ 70 % de pertes ou général tué/capturé ;
  - **Défaite** : autres défaites.
- Conséquences par classe (données) : multiplicateur d'XP du général, prestige de la faction, et
  modificateur de moral de l'armée pendant N tours. Le moral passe par un nouveau champ
  `Army.morale_modifiers: Vec<(i8, u8 tours)>` en `serde(default)`, lu par `battle_request` qui le
  transmet au moral de départ des régiments et par la résolution automatique. S'y ajoute une ligne de
  chronique (« Victoire héroïque de … à … »).
- Exposé au pont (`get_outcome()` / dernier résultat) : clé de classe + libellé.

## 4. Lord à l'échelle TW et zone atteignable

- Core : `reachable_cells(state, army) -> (Vec<Cell> ce tour, Vec<Cell> tour suivant)` dans
  `navigation.rs`/`march.rs`, Dijkstra borné par `movement_left` puis par le mouvement plein (bonus de
  marche forcée compris), mêmes coûts que la marche réelle (ZdC incluse). Exposé par le pont en
  image de masque ou en liste compacte.
- Godot : à la sélection d'une armée, surface colorée à deux tons (ce tour / tour suivant) posée sur le
  terrain par shader. Masquée pendant la fin de tour.
- Marqueur d'armée : la figurine fine du général (FG, corps et monture existants) mise à l'échelle
  (`map.army_figure_scale` en données), tenant l'étendard de l'ost. Icône de posture sur l'étendard ; en
  embuscade, figurine semi-transparente pour le propriétaire. Le compteur d'effectif reste présent. LOD :
  au-delà du zoom régional, retour au marqueur actuel.
- UI de campagne : boutons de posture dans le panneau d'armée (avec la raison du refus en infobulle), site
  de rencontre = marqueur illustré (icône à l'encre DA5) avec bulle au survol, fenêtre de choix au
  gabarit de la chronique, bandeau de classe de résultat sur l'écran de fin de bataille (B2).

## 5. IA

- Embuscade : une armée IA plus faible qu'une armée ennemie en marche vers ses terres, sur une cellule
  couverte à portée de la route prévisible de cette armée, passe en embuscade (poids par personnalité de
  faction).
- Marche forcée : pour secourir une place assiégée ou rejoindre un siège hors de portée normale.
- Camp retranché : armée immobile en infériorité sur une frontière menacée.
- Rencontres : choix pondérés, détours seulement sans objectif urgent.
- Équilibrage : `century_probe` et sonde de difficulté (DF1) relancées ; bande EQ6 conservée.

## 6. Tests et vérification

- Rust : chaque posture (conditions, refus, effets de tour), invisibilité et détection (armée, espion),
  tirage d'embuscade déterministe à graine fixe, placement en colonne dans `sim-battle`, apparition,
  expiration et résolution des sites, seuils de classification des résultats, `reachable_cells` égal au
  coût réel d'une marche, chargement d'une sauvegarde antérieure.
- Python : validation des nouveaux schémas et données (`uv run --project tools pytest`).
- Godot : étapes smoke (changer de posture, voir la zone atteignable, résoudre une rencontre, afficher un
  bandeau de résultat) ; captures dans `docs/img/cv3/` (nom de lot à confirmer au plan).
- Une ADR : postures et ouverture d'embuscade (`docs/decisions/NNNN-postures-embuscade.md`).

## 7. Découpage indicatif (≤ 6 agents)

| Lot | Contenu | Dépend de |
|---|---|---|
| L1 | Postures (core, données, ordre, vision) + résultats nuancés (core) | — |
| L2 | Ouverture d'embuscade dans `sim-battle` + libellés Godot | contrat `BattleOpening` de L1 |
| L3 | Moteur des rencontres (core) + schéma + ~12 rencontres sourcées | — |
| L4 | UI campagne : boutons de posture, marqueurs et fenêtre de rencontre, bandeau de résultat | L1, L3 |
| L5 | `reachable_cells` + surface colorée + figurine du lord | L1 (bonus de marche forcée) |
| L6 | IA des postures et rencontres, équilibrage, sondes | L1-L3 |

Coût cloud : 0 $.

## Annexe A — Comparaison avec Total War: Warhammer III (27/09)

Captures du jeu prises au commit bb3b070e (15 vues campagne, bataille, siège, panneaux) ; captures et
relevé WH3 issus d'un guide UI (gamepressure.com) et de la page Steam. Images non versionnées (droits tiers).

### Graphismes
| # | WH3 | Cent Ans | Écart |
|---|---|---|---|
| G1 | Terrain 3D visible à la vue la plus haute, couleur de faction en frontières et voile | Plateau parchemin à teintes plates, villes en icônes, nuages dessinés partout | fort (choix d'identité : piste = parchemin en filtre MF1) |
| G2 | Colonie = diorama fortifié lisible + bulle (bannière, nom, garnison, croissance) | Maquettes basses, noms en texte flottant | fort |
| G3 | Masses forestières denses stylisées | Arbres « sucettes » clonés isolés | moyen |
| G4 | Lord géant + étendard, zone atteignable colorée | Petit drapeau + compteur, chemin et texte | moyen → **traité ici (§ 4)** |
| G5 | Panneau de province en bas, colonies côte à côte, bâtiments en grille d'icônes | Panneaux parchemin très textuels ; lettrine qui ne réserve pas sa place (« iplomatie ») | moyen ; bug rapide |
| G6 | Post-traitement de bataille (étalonnage, ombres de nuages, poussière) | Herbe uniforme, lumière plate en vue d'ensemble | moyen |
| G7 | HUD de bataille compact, capacités sous les cartes | Journal ≈ ¼ d'écran, ordres du chef en bloc séparé ; rectangle de déploiement affiché pendant le combat | moyen ; bug rapide |

### Défauts de la carte de campagne relevés sur les captures (hors comparaison WH3)
- Distance de caméra maximale (1500) trop courte : on ne cadre pas toute la France depuis Paris, le Midi reste hors champ.
- Brouillard gris-vert qui assombrit nettement l'Angleterre en vue large, au point de gêner la lecture.
- Nuages de météo dessinés sur presque chaque province en vue large : bruit visuel.
- Pluie en vue rapprochée rendue en longues aiguilles blanches, peu naturelles.
- Plaine entre Paris et Orléans peu lisible en relief en vue régionale.
- Chemin de déplacement peu contrasté sur le terrain.
- Étiquettes qui se chevauchent au pied de l'armée (Saint-Denis, Paris, compteur 620).
- « Aucune recherche » affiché deux fois (barre du haut et alerte en bas à droite).
- Panneau de faction plus haut que l'écran : le bas du cadre est coupé (défilement présent).
- Lettrine des titres de panneau qui ne réserve pas sa place (« iplomatie », « le-de-France »).

### Mécaniques
| # | WH3 | Cent Ans | Suite |
|---|---|---|---|
| M1 | Postures embuscade, marche forcée, camp, raid | Normal, chevauchée, siège | **ici (§ 1)** |
| M2 | Équipement de siège construit tour par tour | Engins et brèches ; construction par tour à vérifier | axe batailles de ville |
| M3 | Objets et suite | Absents | axe batailles historiques |
| M4 | Batailles de quête scénarisées | Absentes | axe batailles historiques (Crécy, Poitiers, Azincourt) |
| M5 | Victoire décisive/héroïque/à la Pyrrhus | Victoire/défaite + faits notables | **ici (§ 3)** |
| M6 | Rencontres, points d'intérêt | Chronique au niveau faction | **ici (§ 2)** |
| M7 | Recrutement global | Écarté (Medieval II) | éventuellement mercenaires |
| M8 | Colonie mineure : points de capture, barricades, tours achetées | Siège murs ou bataille rase | axe batailles de ville |
| M9 | Capacités par unité | 5 ordres du chef, pieux | axe batailles de ville + HUD |
| M10 | Arcs de flanc visibles, infobulle d'état | Règles présentes, peu visibles | axe batailles de ville + HUD |

Écarté (fantastique ou refusé) : magie, unités volantes/monstres, corruption, confédération instantanée,
batailles navales 3D.
