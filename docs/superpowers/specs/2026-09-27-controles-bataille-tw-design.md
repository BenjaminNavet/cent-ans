# Contrôles de bataille façon Total War (lots CB) — conception

Date : 2026-09-27. Statut : validée par le joueur, avant plan d'implémentation.

## Contexte

Revue comparative avec Total War: Warhammer 3 (session du 27/09). Trois écarts sont apparus :
1. le modèle de combat (bloc agrégé contre entités individuelles) ;
2. le rythme et l'IA (2 min 30, P1 « trop facile ») ;
3. les **contrôles**.

Le joueur a choisi de traiter d'abord les contrôles. Les points 1 et 2 sont hors périmètre ; le point 2 reste au lot EQ.

Existant réutilisé :
- les bannières flottantes B2/B7 (`game/scripts/battle/battle_unit_markers.gd`) ;
- le déploiement libre avec glisser-droit (centre et orientation) ;
- les groupes Ctrl+N et les 5 ordres de chef (`data/battle_orders/`) ;
- la navgrid R3 ;
- la ligne de vue de bataille.

À remplacer : l'anneau jaune de sélection (`battle_scene.gd`, `_rings`).

## Objectifs

Retrouver la grammaire de contrôle d'un Total War :
- des marqueurs de sélection lisibles ;
- l'aperçu du trajet et un curseur contextuel ;
- une formation réglable au glisser ;
- des modes d'unité ;
- une caméra inclinable et une vue tactique ;
- des capacités actives sobres et historiques.

Non-objectifs :
- changer le modèle de mêlée ;
- corriger la difficulté ;
- ajouter de la magie ou des capacités de héros façon WH3.

## Répartition des couches

Règle projet : toute règle vit dans `core/`, `game/` ne fait que le rendu et les entrées, les chiffres vivent dans `data/`.

| Lot | `core/` | `game/` | `data/` |
|---|---|---|---|
| CB0 | — | extraction des entrées dans `battle_input.gd` | — |
| CB-M | `preview_path`, `hover_context` (pont, lecture seule) | contours, trajets, fantômes, curseurs | — |
| CB1 | largeur de formation dans l'ordre `Move`, rangs déduits | aperçu au glisser, répartition multi-unités | `rules/formation_width.json` |
| CB2 | états de mode, effets, usage par l'IA | touches, icônes | `rules/unit_modes.json` |
| CB3 | — | caméra, vue tactique | — |
| CB4 | moteur de capacités, usage par l'IA | boutons de carte, recharge | `battle_abilities/*.json` + schéma |

Une ADR (numéro libre suivant, 0095 à ce jour) consigne trois choix : le nouveau schéma de touches, les deux requêtes du pont en lecture seule, et la séparation entre ordres de chef (armée) et capacités (unité).

## Raccourcis

Ils sont alignés sur WH3, avec le changement d'habitudes accepté par le joueur.

| Touche | Action | Avant |
|---|---|---|
| F | Tir à volonté | cycle de formation |
| T | Cycle de formation | libre |
| R | Bascule marche/course | libre |
| G | Garde (tenir sa position) | tir à volonté |
| K | Escarmouche | libre |
| M | Mode mêlée (tireurs) | libre |
| Alt+1 à Alt+4 (Option sur Mac) | Capacités de l'unité sélectionnée | libre (1-9 = rappel de groupe, Ctrl/Cmd+1-9 = création : inchangés, comme WH3) |
| Tab | Vue tactique | libre |
| Bouton du milieu | Rotation et inclinaison | panoramique (passe sur Maj + bouton du milieu) |

H (halte), C, U, J, F1, Z X V B N, Espace, +/−, Q/E et les groupes (1-9, Ctrl/Cmd+1-9, double appui = centrer) sont inchangés. L'aide F1 est mise à jour.

## CB0 — Extraction des entrées

- Déplacer la gestion des entrées de bataille (`battle_scene.gd`, environ lignes 1627-1922) vers `game/scripts/battle/battle_input.gd`. Celui-ci émet des signaux d'ordre, et la scène garde le rendu.
- Aucun changement de comportement.
- Test : une séquence d'entrées scriptée produit la même liste d'ordres avant et après.
- But : éviter les conflits entre lots, qui touchent tous les entrées.

## CB-M — Marqueurs, trajet, curseur

**Contour de formation**
- Une décale Godot par régiment, projetée sur le relief. Elle a les dimensions du front et de la profondeur renvoyés par `get_units()` et suit le cap de l'unité.
- États :

| État | Rendu |
|---|---|
| Sélectionnée | Trait plein, couleur du camp |
| Survolée | Trait pâle |
| Ennemie survolée | Rouge, trait pointillé |
| Ennemie ciblée | Rouge pulsé |
| En déroute | Rien |

- L'anneau jaune est supprimé ; les bannières B2 sont inchangées.

**Aperçu du trajet au survol, avant le clic**
- Pour l'unité sélectionnée, trajet prévu vers le point sous la souris : pointillés aux couleurs du camp, fantôme de la formation d'arrivée.
- Nouvel appel du pont `preview_path(unit_id, x, z) -> PackedVector3Array`. Il fait le même calcul que l'ordre réel et ne modifie aucun état.
- Recalcul seulement si le curseur a bougé de plus de 5 m, au plus 10 fois par seconde.
- Au-delà de 6 unités sélectionnées, un seul trajet, depuis le centre du groupe.
- Destination inaccessible : trajet rouge et curseur « interdit ».
- Après l'ordre, trajet, fantôme et flèche d'attaque restent visibles tant que l'unité est sélectionnée.

**Curseur contextuel**
- Nouvel appel du pont `hover_context(x, z, selected_ids) -> String`. Il renvoie l'une de ces valeurs : `move`, `melee`, `ranged`, `ranged_blocked`, `siege`, `forbidden`, `none`.
- La portée, la ligne de vue et le caractère attaquable d'une porte ou d'un mur sont évalués dans le cœur :

| Sous le curseur | Sélection | Contexte |
|---|---|---|
| Sol accessible | toute | `move` |
| Ennemi | mêlée, ou tireur en mode mêlée | `melee` (épées croisées) |
| Ennemi | tireur | `ranged`, ou `ranged_blocked` si hors portée ou sans ligne de vue |
| Porte, muraille, tour | bélier, engin, échelles | `siege` |
| Porte, muraille | autre unité | `melee` |
| Infranchissable, hors déploiement | toute | `forbidden` |
| Allié, ou rien de sélectionné | — | `none` |

- Icônes au style DA5 dans `game/assets/ui/cursors/` (32 px, point chaud centré), posées avec `Input.set_custom_mouse_cursor`.

## CB1 — Formation au glisser

- `core/` : l'ordre `Move` porte une largeur optionnelle.
  - `unit.rs` en déduit le nombre de rangs, borné par type selon `data/rules/formation_width.json` (par exemple piquiers ≥ 4 rangs, archers ≥ 2), et conserve l'effectif.
  - Sans largeur, la formation actuelle est gardée.
- `game/`, pendant le glisser-droit, fantôme en direct avec le rendu CB-M :
  - une unité : la longueur du glisser donne la largeur ;
  - plusieurs unités : la largeur totale est répartie au prorata de l'effectif, dans l'ordre gauche-droite courant ;
  - Maj + glisser conserve l'espacement relatif (Alt est réservé aux capacités) ;
  - un clic simple garde la largeur.
- Vaut aussi pendant le déploiement.

## CB2 — Modes d'unité

États dans `core/`, chiffres dans `data/rules/unit_modes.json`.

| Mode | Effet |
|---|---|
| Marche/course (R) | Bascule persistante ; la course est plus rapide et fatigue plus. Le double clic droit reste une course ponctuelle |
| Garde (G) | Ne poursuit pas, ne se laisse pas entraîner, tient sa position au contact |
| Escarmouche (K) | Tireurs légers : recul automatique devant une mêlée qui approche. Remplace la capacité passive `Skirmish` |
| Mêlée (M) | Un tireur cesse de tirer et engage au corps à corps |
| Charge | Pas de bouton : un ordre d'attaque lancé à pleine vitesse donne le bonus, comme aujourd'hui |

- Icônes de mode sur les cartes d'unité et dans la barre d'ordres.
- L'IA met ses tireurs en escarmouche et sa ligne en garde en posture défensive.

## CB3 — Caméra et vue tactique

**Caméra**
- Bouton du milieu maintenu : le glisser horizontal fait tourner, le glisser vertical incline, avec des bornes de 5° à 85°.
- L'inclinaison automatique reste le défaut. Elle est suspendue après une inclinaison manuelle, jusqu'au prochain recentrage (C, double appui de groupe).

**Vue tactique (Tab)**
- Caméra du dessus sur tout le champ, terrain assombri.
- Unités sous forme d'icônes : on réutilise les pastilles B7 et les contours CB-M.
- La simulation continue ; Espace met en pause.
- Ordres, glisser, trajets et curseur restent actifs.
- Seuls les ennemis repérés, selon la ligne de vue de bataille, sont affichés.
- Tab ou Échap rend la caméra précédente.

## CB4 — Capacités actives

**Cœur**
- Définition : temps de recharge, durée, modificateurs de stats ou d'état, conditions (immobile, pas au contact, munitions > 0).
- L'IA les utilise selon une règle simple par capacité.
- Déterministe, donc compatible avec le rejeu EP13.

**Données** : `data/battle_abilities/*.json`, validé par `data/schemas/`. 1 ou 2 capacités par type. Liste initiale, indicative :

| Unité | Capacité | Effet |
|---|---|---|
| Archers anglais | Tir tendu | Portée −40 %, pénétration et précision + à courte distance |
| Arbalétriers | Derrière le pavois | Immobile, protégés du tir (l'ordre de chef « pavois » passe ici) |
| Chevaliers | Charge en haie | Charge +, cohésion − après le choc |
| Hommes d'armes à pied | Rangs serrés | Défense +, vitesse − (branche le `ShieldWall` inutilisé) |
| Piquiers | Hérisson | Immobile, bonus contre la cavalerie sur toutes les faces |
| Engins de siège | Tir de rupture | Dégâts aux murs +, cadence − |

- La liste est relue par l'historien avant l'implémentation, avec ses sources dans `docs/research/`.
- Les ordres de chef (cri de guerre, rallier, pied à terre, pas de quartier) restent à l'échelle de l'armée.

**Interface** : 1 à 3 boutons par carte d'unité, avec un cadran de recharge et une infobulle chiffrée qui passe par RuleValues. Alt+1 à Alt+4 agissent sur l'unité sélectionnée ; avec plusieurs unités, sur toutes celles qui ont la capacité.

## Erreurs et cas limites

- Si `preview_path` ne trouve aucun chemin, le curseur passe à `forbidden` et l'ordre n'est pas envoyé.
- Une capacité dont les conditions ne sont plus remplies s'arrête. Son bouton est grisé, et l'infobulle en donne la raison.
- Une largeur hors bornes est ramenée aux bornes ; le fantôme montre la largeur retenue.
- Si la vue tactique est ouverte pendant le déploiement, les contraintes de zone restent appliquées (curseur `forbidden` hors zone).

## Vérification

- `cargo test` :
  - largeur vers rangs (bornes, effectif) ;
  - effets des modes ;
  - moteur de capacités (recharge, conditions, déterminisme du rejeu) ;
  - `preview_path` identique au chemin de l'ordre réel ;
  - `hover_context` (portée, ligne de vue, murs, portes).
- `smoke.gd` ; test d'équivalence des entrées pour CB0.
- Scripts headless `game/tests/cb*_shot.gd` ; au plus 3 captures par lot (règles CLAUDE.md).
- Batailles de référence avant/après CB2 et CB4 : les marges EP/EQ7 ne doivent pas bouger au-delà du bruit.

## Ordre et exécution

- CB0, puis CB-M, puis CB1, en série.
- Puis CB2 et CB3 en parallèle (worktrees séparés, cible cargo privée), puis CB4.
- Agents Sonnet pour les lots mécaniques ; la vérification visuelle revient à la session principale.
- Chaque lot a sa note `docs/wip/cb*.md` et ses commits `wip:`, au plus toutes les 15 minutes.
