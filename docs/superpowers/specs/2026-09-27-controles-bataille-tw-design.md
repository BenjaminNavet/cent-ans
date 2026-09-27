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
- des capacités actives sobres et historiques ;
- des ordres en file, la portée affichée, une comparaison au survol, des alertes.

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
| CB5 | émission des événements d'alerte | colonne d'alertes, minicarte, cris | `rules/battle_alerts.json` |

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
| Maj + clic droit | Ajouter un point de passage (ordres en file) | libre |
| Ctrl/Cmd + A | Sélectionner toutes ses unités | libre |
| Double-clic (unité ou carte) | Sélectionner toutes les unités du même type ; sur une carte, centrer aussi la caméra | libre |
| Ctrl/Cmd + G | Verrouiller/déverrouiller le groupe sélectionné | libre |
| − sous ×1 | Ralenti ×0,5 | minimum ×1 |
| Bouton du milieu | Rotation et inclinaison | panoramique (passe sur Maj + bouton du milieu) |

H (halte), C, U, J, F1, Z X V B N, Espace, +/−, Q/E et les groupes (1-9, Ctrl/Cmd+1-9, double appui = centrer) sont inchangés. L'aide F1 est mise à jour.

## CB0 — Extraction des entrées et sélection rapide

- Déplacer la gestion des entrées de bataille (`battle_scene.gd`, environ lignes 1627-1922) vers `game/scripts/battle/battle_input.gd`. Celui-ci émet des signaux d'ordre, et la scène garde le rendu.
- Aucun changement de comportement.
- Test : une séquence d'entrées scriptée produit la même liste d'ordres avant et après.
- But : éviter les conflits entre lots, qui touchent tous les entrées.
- Une fois l'équivalence vérifiée (commit séparé), ajout de la **sélection rapide** :
  - Ctrl/Cmd + A sélectionne toutes ses unités présentes ;
  - un double-clic sur une unité ou sur une carte sélectionne toutes les unités du même type ;
  - un double-clic sur une carte centre aussi la caméra.

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
- Nouvel appel du pont `hover_context(x, z, selected_ids) -> Dictionary`. Son champ `context` prend l'une de ces valeurs : `move`, `melee`, `ranged`, `ranged_blocked`, `siege`, `forbidden`, `none`. Son champ `compare` (facultatif) sert à la comparaison au survol.
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

**Ordres en file (Maj + clic droit)**
- `core/` : un ordre `Move` ou `Attack` peut être **ajouté** à la file d'une unité au lieu de la remplacer. On borne la file à 8 ordres. L'unité enchaîne les ordres ; un clic sans Maj vide la file.
- `game/` : points de passage numérotés au sol, trajet complet segment par segment (`preview_path` depuis le dernier point de la file).
- Maj + glisser-droit ajoute un point de passage avec la formation (largeur, orientation) à l'arrivée.
- La file est sauvegardée dans le rejeu EP13, puisqu'elle passe par les ordres du cœur.

**Portée de tir au sol**
- Pour un tireur sélectionné ou survolé : arc au sol, dans le champ de tir de l'unité, à la portée effective.
- La portée effective (météo, tir tendu, relief) est fournie par le cœur via `get_units()` (champ `effective_range`).
- Les zones sans ligne de vue ne sont pas calculées au sol en v1 : c'est `ranged_blocked` au curseur qui le signale.

**Comparaison au survol**
- Avec une seule unité sélectionnée, survoler un ennemi (sur le terrain, sa bannière ou sa carte) ouvre un petit panneau face à face près du curseur.
- Il compare : effectif, mêlée, défense (armure), charge, tir et portée, moral, fatigue, bonus contre (piques contre cavalerie, etc.).
- Les chiffres viennent du cœur (`hover_context` renvoie un dictionnaire : `{context, compare}`) et passent par RuleValues : pas de calcul en GDScript.
- Les deux côtés sont affichés au même format ; les avantages nets sont signalés par une couleur (vert / rouge), sans pronostic chiffré de victoire.

## CB1 — Formation au glisser

- `core/` : l'ordre `Move` porte une largeur optionnelle.
  - `unit.rs` en déduit le nombre de rangs, borné par type selon `data/rules/formation_width.json` (par exemple piquiers ≥ 4 rangs, archers ≥ 2), et conserve l'effectif.
  - Sans largeur, la formation actuelle est gardée.
- `game/`, pendant le glisser-droit, fantôme en direct avec le rendu CB-M :
  - une unité : la longueur du glisser donne la largeur ;
  - plusieurs unités : la largeur totale est répartie au prorata de l'effectif, dans l'ordre gauche-droite courant ;
  - un clic simple garde la largeur.
- Vaut aussi pendant le déploiement.

**Verrouillage de groupe (Ctrl/Cmd + G)**
- Un groupe verrouillé garde sa disposition relative (positions et orientations) et se déplace comme un bloc. Il avance à la vitesse de son unité la plus lente.
- Au glisser, un groupe verrouillé est tourné et translaté, pas redistribué.
- Le verrouillage est un état de `game/` (sélection) traduit en ordres `Move` individuels. La vitesse commune est une option de l'ordre dans `core/` (`match_speed`).
- Un cadenas s'affiche sur les cartes du groupe.

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

**Icônes d'état** sur les bannières B2 et les cartes, au style DA5 :
- déjà présentes : moral bas, épuisement ;
- ajoutées : charge en cours, hésitation (moral sous le seuil d'avertissement), sous le feu, au contact, garde, escarmouche, course, mêlée (tireur), capacité active.
- Les états viennent de `get_units()` (le cœur expose `charging`, `under_fire`, `engaged` et les modes) ; les seuils vivent dans `data/`.
- Au plus 3 pastilles par bannière, selon un ordre de priorité fixe : déroute > hésitation > sous le feu > charge > mode.
- L'IA met ses tireurs en escarmouche et sa ligne en garde en posture défensive.

## CB3 — Caméra et vue tactique

**Vitesse** : le − descend jusqu'au ralenti ×0,5 (paliers ×0,5, ×1, ×2, ×4). Le rejeu garde ses propres paliers.

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

## CB5 — Alertes de bataille

- Les événements viennent du cœur, qui les émet déjà en partie pour le journal : unité en déroute, général blessé ou tué, flanc ou dos attaqué, renforts (alliés ou ennemis), munitions épuisées, mur ou porte tombé.
- `game/` : une colonne de 5 alertes au plus, à gauche.
  - Chaque alerte a une icône et un texte court, et s'efface après 8 s.
  - Un clic centre la caméra sur le lieu ; un repère pulse sur la minicarte.
  - Les alertes de même type dans la même zone sont regroupées dans les 5 s.
- Des cris de soldats courts (déroute, général tombé) sont joués avec l'audio AU1 existant, s'ils sont disponibles ; sinon un son d'alerte neutre.
- Importance, durée et fusion des alertes sont réglées dans `data/rules/battle_alerts.json`.

## Erreurs et cas limites

- Si `preview_path` ne trouve aucun chemin, le curseur passe à `forbidden` et l'ordre n'est pas envoyé.
- Une capacité dont les conditions ne sont plus remplies s'arrête. Son bouton est grisé, et l'infobulle en donne la raison.
- Une largeur hors bornes est ramenée aux bornes ; le fantôme montre la largeur retenue.
- File d'ordres pleine (8) : le Maj + clic suivant est refusé, avec un curseur `forbidden` et une infobulle.
- Groupe verrouillé contenant une unité en déroute : l'unité sort du groupe et le reste du groupe continue.
- Si la vue tactique est ouverte pendant le déploiement, les contraintes de zone restent appliquées (curseur `forbidden` hors zone).

## Vérification

- `cargo test` :
  - largeur vers rangs (bornes, effectif) ;
  - effets des modes ;
  - moteur de capacités (recharge, conditions, déterminisme du rejeu) ;
  - `preview_path` identique au chemin de l'ordre réel ;
  - `hover_context` (portée, ligne de vue, murs, portes, chiffres de comparaison) ;
  - file d'ordres (ajout, enchaînement, vidage, rejeu) ;
  - `match_speed` ;
  - émission des alertes.
- `smoke.gd` ; test d'équivalence des entrées pour CB0.
- Scripts headless `game/tests/cb*_shot.gd` ; au plus 3 captures par lot (règles CLAUDE.md).
- Batailles de référence avant/après CB2 et CB4 : les marges EP/EQ7 ne doivent pas bouger au-delà du bruit.

## Ordre et exécution

- CB0, puis CB-M, puis CB1, en série.
- Puis CB2, CB3 et CB5 en parallèle (worktrees séparés, cible cargo privée), puis CB4.
- CB-M est le lot le plus lourd ; son plan le découpera en sous-étapes (marqueurs, trajet et curseur, file, portée et comparaison).
- Agents Sonnet pour les lots mécaniques ; la vérification visuelle revient à la session principale.
- Chaque lot a sa note `docs/wip/cb*.md` et ses commits `wip:`, au plus toutes les 15 minutes.
