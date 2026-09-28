# 0101 — Sort de la ville prise (TW2-T1)

Date : 2026-09-28. Statut : accepté. Spec : `docs/design/2026-09-28-tw2-mecaniques-total-war.md` § T1.

## Contexte
`siege::capture` remettait la place et ajoutait des troubles, sans choix. Total War propose
occuper / piller / raser. Il fallait quatre issues chiffrées en données, un choix du joueur dans
une fenêtre, un choix de l'IA, et un chemin unique pour tous les appelants (siège, marche, agents).

## Décision
- **Chemin unique** : `siege::capture` fait l'occupation (troubles `occupy.unrest_city/place`, lus
  dans `data/rules/capture.json`) puis appelle `capture::on_captured`. Les appelants (siège, affamement,
  assaut, village, marche, corruption d'agent) n'ont pas changé.
- **Issues additives** : occuper n'ajoute rien ; rançon, pillage et rasage ajoutent leurs effets à
  l'occupation (or, troubles, population, dévastation, bâtiments, expérience, piété, faveur
  pontificale, opinions). L'aperçu (`capture::preview`) calcule exactement ce qu'applique
  `apply_outcome` ; l'UI affiche ce même aperçu.
- **Or** : assiette fiscale saisonnière de la province (`economy::province_income`) × part de l'issue
  × part de la place (`place_share`, cité 1, bourg 0,6, château 0,3, abbaye 0,8, village 0,2), avec un
  plancher. Chiffres : rançon ×1,5 (plancher 60), pillage ×4 (plancher 150), rasage ×0,5 (plancher 20).
- **Pillage** : population −15 %, dévastation +25, troubles +30, bâtiment le plus récent détruit,
  expérience +1 aux unités des armées preneuses, piété du souverain −5, faveur pontificale −3,
  opinion de l'ancien tenant −25 et des autres −5 (20 tours).
- **Rasage** : interdit sur les cités de province (`raze_forbidden_kinds`) et 13 lieux emblématiques
  (Saint-Denis, Vincennes, Windsor, Mont-Saint-Michel, Cluny, Cîteaux, Clairvaux, Fontevraud, Grande
  Chartreuse, Vézelay, Rocamadour, Conques, Assise). Tous les bâtiments de la place détruits, −1
  niveau de fortification ; une place ramenée à 0 devient une **ruine** 8 tours (ni recrutement ni
  chantier, erreur `SettlementRuined`). Population −25 %, troubles +40, opinions −40/−15 (40 tours).
- **Décision en attente propre** (`CaptureState` dans `CampaignState::captures`, `serde(default)`,
  pas de changement de `STATE_VERSION`) plutôt que les `Decision` de la chronique : celles-ci
  pointent vers un événement de `data/events/` et des `EventEffect` génériques, alors qu'ici les effets
  dépendent de la place et se calculent. On réutilise en revanche le **même composant d'UI**
  (`ChronicleWindow`, enrichi d'un libellé de genre et de choix grisés avec raison) et le même motif
  (vue + ordre `ChooseCaptureOutcome`). Sans réponse, la place reste simplement occupée au début de
  la fin de tour (comme les rencontres CV3-3).
- **IA sans tirage** : somme de scores (doctrine par faction, trésor < 300, culture étrangère,
  reconquête de sa propre terre, place intenable car la cité de la province reste ennemie) ; le plus
  haut gagne, égalités dans l'ordre occuper > rançon > pillage > rasage. Pas de tirage pour ne pas
  décaler le flux aléatoire de la campagne. Doctrines : Angleterre plus encline à la rançon et au
  pillage (chevauchées), Écosse au rasage des places intenables (politique de Bruce), rebelles
  n'occupent que.

## Conséquences
- L'IA pauvre en terre étrangère rançonne ; l'Écosse rase les châteaux isolés ; une reconquête est
  toujours une occupation. L'équilibre global des tests de campagne est à surveiller (voir wip).
- `CAPTURE_UNREST` reste comme constante de référence ; la valeur lue est celle des données.
- Pont : `get_pending_captures`, `choose_capture_outcome(id, "occupy"|"ransom"|"sack"|"raze")`,
  `debug_capture_place` (tests). Godot : `CaptureController`.
