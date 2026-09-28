# 0095 — Contrôles de bataille façon Total War (lots CB)

Date : 2026-09-27, finalisée le 2026-09-28. Statut : accepté (tous les lots de code fusionnés, main 526853a4).
Spec : `docs/superpowers/specs/2026-09-27-controles-bataille-tw-design.md`.
Plan : `docs/superpowers/plans/2026-09-27-controles-bataille-tw.md`.

## Contexte

La revue comparative avec Total War: Warhammer 3 a fait des contrôles de bataille la priorité.
Les lots CB touchent tous les entrées de bataille, le pont et les ordres du cœur (donc le rejeu EP13).

## Décision

1. **Nouveau schéma de touches aligné sur WH3.** F = tir à volonté, T = cycle de formation,
   G = garde, R = marche/course, K = escarmouche, M = mêlée, Alt/Option+1-4 = capacités,
   Tab = vue tactique, Maj + clic droit = ordre en file, Ctrl/Cmd+A, Ctrl/Cmd+G, ralenti ×0,5,
   bouton du milieu = rotation/inclinaison (panoramique sur Maj + milieu), Alt+Maj+1…6 = formations
   de groupe (CB6). Groupes inchangés. Entrées extraites dans `game/scripts/battle/battle_input.gd`
   (CB0) ; **une table unique** `game/scripts/battle/battle_hotkeys.gd` aiguille les touches et
   génère l'aide F1 et les libellés (CB2). Ordres du chef : Z X V B (« Pas de quartier » passe de N à B).
2. **Deux requêtes du pont en lecture seule** : `preview_path(unit, x, z)` et
   `hover_context(x, z, selected)`. Aucune ne modifie l'état simulé (`state_digest` inchangé).
   `preview_path` et l'ordre réel partagent `plan_route`, qui renvoie la chaîne complète de points
   de passage (ligne droite + gué/pont en rase campagne, A* de siège) ; `route()` n'en garde que le
   premier point. Il n'y a pas de navgrid en bataille : la navgrid R3 reste propre à la campagne.
3. **Ordres de chef (armée) ≠ capacités (unité).** Cri de guerre, rallier, pied à terre, pas de
   quartier restent des ordres de chef ; le pavois devient la capacité des arbalétriers (CB4).

## Écarts à la spec, tranchés dans le plan

- Pas de brouillard de guerre existant : champ cœur `spotted`, utilisé par la vue tactique seulement.
- Pas d'état « blessé » : l'alerte couvre « général tué ou capturé ».
- Journal non typé : flux typé `BattleAlert` ajouté à côté du journal, qui reste inchangé.
- Largeur de formation : `line_files: Option<u32>` sur l'unité, pas de nouveau type de formation.
- Rejeu : nouveaux champs de `Command` en `#[serde(default)]`, variantes additives ;
  `REPLAY_FORMAT` inchangé tant que les anciens rejeux se relisent.

## Addenda par lot

- **CB-M2 — invariant aperçu = ordre.** Pas d'itinéraire stocké (l'ordre garde sa seule destination).
  L'aperçu suit exactement l'ordre en ligne droite et au pont ; il s'en écarte d'au plus ≈ 3 m au gué
  et ≈ 13 m en siège, car l'unité recalcule son chemin en marchant. Accepté par le joueur (09-27).
  L'aperçu n'apparaît qu'au clic droit maintenu (coût). Sans chemin, l'ordre n'est pas envoyé.
- **CB-M3 — file d'ordres.** Borne de 8 dans un fichier propre `data/rules/battle_queue.json`.
  `preview_path_from` ajouté plutôt que changer la signature de `preview_path`. Une attaque en file
  s'arrête sur une cible en fuite seulement si un ordre attend ; sinon la poursuite reste l'ancienne.
- **CB-M4 — portée et comparaison.** Le cœur n'a pas de champ de tir (les tireurs pivotent) :
  `range_arc.fire_half_angle_deg` (60°) sert au dessin seulement.
- **CB1 — largeur.** Une largeur passe le régiment en Ligne ; répartition des largeurs de groupe dans
  le cœur (intervalle `group_gap_m`) ; `match_speed` plafonne à l'allure en terrain ouvert du plus lent ;
  bornes de rangs par catégorie dans `data/rules/formation_width.json` (premières valeurs).
- **CB2 — modes.** Les drapeaux de mode ne sont pas hachés dans `state_digest`, seuls leurs effets le
  sont (sinon l'ancien échantillon de rejeu divergeait dès 10 s sans différence visible). L'IA ne met
  sa ligne en garde que tant qu'elle attend (la garde en combat faisait tomber Poitiers à 6/20).
  « Battre en brèche » (engins, murs seulement) est un mode et non une capacité (relecture historique).
- **CB3 — `spotted`.** Fonction pure (`BattleSim::spotted_by`), hors empreinte et hors rejeu ; portée
  reprise de `missile_arc.spotter_range_m` (350 m). Assombrissement de la vue tactique par un calque
  d'interface, sans toucher aux shaders.
- **CB5 — alertes.** Flux de sortie pur, hors empreinte ; front montant du flanc suivi par
  `flanked_alerted` (le champ `flanked` est remis à zéro à chaque pas) ; mur et porte émis en un point
  unique (`record_siege_transitions`).
- **CB6 — formations de groupe** (demande du joueur, 09-27). Six préréglages en données
  (`data/rules/group_formations.json`), relus par l'historien (`docs/research/cb6-formations.md`).
  « Ligne de bataille » reproduit au bit près le placement initial et le déploiement IA d'avant
  (golden `cb6_deploy_golden.json`) : tireurs derrière l'infanterie, par non-régression d'équilibrage.
  Aucun nouvel ordre : un préréglage émet des `Move` individuels.
- **CB4 — capacités.** Cinq capacités après relecture historique (`docs/research/cb4-capacites.md`) :
  Tir tendu, Dresser les pavois, Se rallier à la bannière, Serrer les rangs, Piques plantées.
  L'ordre de chef pavois est retiré, mais sa règle reste dans le cœur pour relire les anciens rejeux.
  `UseAbility` passe par le filtre de scénario EP7 (comme l'ancien ordre, qui de ce fait ne se
  déclenchait jamais à Crécy). L'IA n'emploie le tir tendu que pour un camp à l'attaque. Les pieux
  restent un passif dès 1337 (datation à 1415 = chantier d'équilibrage séparé).

## Conséquences

- Rejeu : `REPLAY_FORMAT` inchangé ; tous les champs nouveaux sont omis à leur valeur par défaut, et
  un rejeu enregistré avec file, largeur, modes ou capacités se rejoue exactement (tests `cb_queue`,
  `cb1_width`, `cb2_modes`, `cb4_abilities`). L'échantillon EP13 d'origine diverge depuis des lots
  d'équilibrage antérieurs à CB (attendu : EP13 signale la divergence) ; seule sa lecture est testée.
- Marges de référence (release, mêmes graines) :

  | Bataille | Bande | Avant CB | Après CB2 | Après CB4 |
  |---|---|---|---|---|
  | Crécy | 14-19/20 | 16 | 16 | 18 |
  | Azincourt | 14-19/20 | 18 | 18 | 18 |
  | Poitiers | 11-18/20 | 14 | 14 | 16 |
  | EQ7 cavalerie | ≥ 12/16 | 15 | 15 | 15 |

  Crécy n'a plus qu'une victoire de marge haute : à surveiller au prochain lot d'équilibrage.
- Interface : carte d'unité 112 px et bandeau 140 px (boutons de capacité). Icônes : glyphes dessinés
  en code en repli, icônes DA5 générées en un lot.
- Suites : sélecteur de formations repliable, cris d'alerte parfois doublés, bornes de rangs CB1 à
  faire relire, partie pilote du joueur.
