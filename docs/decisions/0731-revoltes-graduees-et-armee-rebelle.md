# 0731 — Révoltes graduées et armée rebelle

Statut : accepté (lot B4, nuit critique du 22/10/2026). Numéro 0730 réservé au lot B3.

## Contexte

Le modèle d'EQ1 déclenchait une révolte après `revolt_seasons` (5) saisons consécutives au-dessus de
75 de mécontentement moyen, compteur remis à zéro à la moindre rémission. Constats
(`docs/wip/nuit-critiques/game-design.md` § 2-3, `docs/wip/refonte-systemes.md` § 1) :

- aucun signal gradué : on passe de « tout va bien » à la révolte, et une saison de répit efface tout ;
  les bots oscillent autour du seuil sans conséquence (0 à 4,3 révoltes par partie selon le bot) ;
- une révolte n'a pas de contre-jeu militaire : 25 % de garnison perdue, et la ville passe aux
  rebelles d'office au-dessus de 90, sans armée ni bataille.

## Décision

1. **Jauge de tension par province** (`ProvinceState::tension`, 0 à `tension_max`), cumulative.
   Chaque saison où le mécontentement moyen dépasse `revolt_unrest_threshold`
   (`population.json`), elle monte de `tension_rise_base + tension_rise_per_point × excès` ; sinon
   elle baisse de `tension_decay`. Une province déjà aux mains des rebelles ne monte pas.
2. **Avis** : au franchissement montant de chaque palier de `warning_tensions` (50, 75), un
   événement `Revolt` (« Grogne à X », sans porter atteinte à la garnison) est émis pour le
   souverain de la province ; il est distingué du soulèvement par `GameEvent::faction` et par son
   texte, et ne compte pas comme révolte dans les métriques seed-bots (nouvelle métrique
   `unrest_warnings_per_game`).
3. **Soulèvement** à `tension_max` : la garnison perd `garrison_loss_percent`, une **armée
   `fac_rebels`** (`rebel_levy` : un régiment par `population_per_regiment` habitants, entre
   `min_regiments` et `max_regiments`, paysans et bourgeois selon la structure sociale) apparaît
   devant la ville, le souverain est en guerre avec `fac_rebels`, la tension retombe à
   `tension_after_revolt`. Le soulèvement est suivi dans `ProvinceState::uprising`.
4. **Contre-jeu** : tant que l'armée rebelle tient, la tension ne monte plus ; le joueur (bataille
   3D) ou l'IA (résolution automatique) peut la combattre par le circuit habituel
   (`movement::fight`).
   - La garnison de la ville harcèle l'armée rebelle chaque saison (`garrison_sortie_*` : perte en
     proportion du rapport garnison / rebelles, plafonnée) : une ville tenue disperse seule les
     rebelles, une ville vide ne le peut pas. C'est le contre-jeu de l'IA, qui sinon combat peu
     les armées de `fac_rebels` ; le joueur peut abréger en les battant lui-même.
   - Armée détruite ou réduite sous `rebel_rout_percent` de l'effectif levé : **révolte matée**,
     tension ramenée à `repression_tension` au plus, troubles récents réduits de
     `repression_disorder_relief`, guerre avec les rebelles close.
   - Armée toujours là après `rebel_siege_seasons` saisons : la ville **passe aux rebelles**
     (ancien comportement, désormais conditionné à l'inaction).
5. Toutes les valeurs sont dans `data/rules/revolt.json`, validées par
   `data/schemas/revolt_rules.schema.json`. `revolt_seasons` et `revolt_control_threshold` quittent
   `population.json` (remplacés par ce fichier). `ProvinceState::revolt_seasons` subsiste comme
   compteur informatif de saisons consécutives au-dessus du seuil (carte, IA économique).
6. UI : seul `tension` est exposé (fiche de province de `game-api`, dump d'état) ; pas de refonte.

## Garde-fou d'équilibre

La part de guerre FR-EN doit rester dans 54-76 % (`campaign_probe`, ADR 0246) ; le seuil de
révolte (75) n'est pas touché, seuls la montée de la jauge et la reddition conditionnelle le sont.
Mesures avant/après dans `docs/wip/nuit-critiques/lots/B4.md`. Les paramètres `data/` servent à
recaler.

## Conséquences

- Les révoltes deviennent prévisibles (avis) et combattables ; la prise de ville par les rebelles
  devient rare si le souverain réagit.
- Nouvelle forme de sauvegarde : `tension` et `uprising` (champs `serde(default)`, anciennes
  sauvegardes lisibles).
- Les guerres avec `fac_rebels` sont exclues de la métrique « durée moyenne des guerres » des seed
  bots.
- Restent hors lot : contagion aux voisins, armée rebelle selon la culture (lots C/D/E/F du plan).
