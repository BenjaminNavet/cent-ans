# ADR XXXX — La cause au moment de l'effet (XP-POURQUOI, R4)

Date : 2026-10-30. Statut : accepté (lot XP-POURQUOI, boucle juge 2). Amende l'ADR 0281 (le rapport de saison `auto` devient le
Récit) et l'ADR 0754 (journal des causes). Numéro à attribuer à la fusion.
Source : `docs/audits/2026-10-29-experience.md` § R4 et § « Lot XP-POURQUOI ».

## Contexte

Au tour 3, `journal_len = 0` : le journal des causes (ADR 0748) ne connaissait que les évènements rares (bataille perdue, banqueroute,
révolte…). Les variations quotidiennes — le trésor qui fond, une province qui s'échauffe, un voisin qui change d'humeur — n'avaient
aucune cause visible. Le rapport de saison `auto` ouvrait une fenêtre modale qui coupait le jeu sans dire pourquoi.

## Décision

1. **Trois genres de causes** dans `sim_campaign::causes::CauseKind` : `TreasurySwing`, `UnrestSwing`, `AttitudeShift`. Seuils en
   données (`data/rules/cause_thresholds.json`, schéma `cause_thresholds.schema.json`, `data_model::CauseThresholds`) : trésor
   ±10 % (et au moins 300 livres, contre le bruit d'une caisse presque vide), troubles d'une province ±10 points, paliers d'attitude
   `[-50, -20, 20, 50]`, au plus 6 causes de variation du joueur par saison (les plus lourdes d'abord), 3 causes dans le Récit,
   3 termes sous un chiffre.
2. **Détection dans le cœur** : `sim_campaign::season_marks`. `Baseline::capture` note, au début de la résolution du tour, ce que le
   joueur voyait ; `record` compare à la fin et verse les causes dans le journal. Les facteurs d'un `TreasurySwing` sont les lignes du
   budget de la saison (`BudgetRecord`) : un terme qui va dans le sens du changement pèse son poids, un terme qui le contrarie un
   quart ; ceux d'un `UnrestSwing` sont les termes de mécontentement de la province (comme pour une révolte) ; ceux d'un
   `AttitudeShift` sont les raisons de l'attitude (libellés français déjà écrits par la diplomatie, clé de facteur
   `attitude:<libellé>`). Seul le royaume du joueur est surveillé.
3. **Repères de saison** (`CampaignState::season_marks`, `serde(default)`, sans migration) : les chiffres de la barre du haut tels
   que le joueur les voyait pendant la saison résolue. `season_marks::trends` en tire, pour chaque chiffre (trésor, solde, hommes,
   prestige, mécontentement), l'écart depuis la saison passée et au plus `top_terms` termes ; `top_terms` ferme toujours la somme
   avec un terme « Autres éléments » (le total est exact, jamais approché).
4. **API `game-api`** : `hud::TopBar.trends` (`FigureTrend` : `delta_since_last_season`, `top_terms`, `terms_last_season`) ;
   `causes::season_story` / `season_story_with` (`SeasonStory { changed, why, next, act_delta }`) ; `causes::season_causes(n)` choisit
   les causes de la saison (plus ancienne jusqu'à 4 saisons pour compléter), au plus deux d'un genre tant que d'autres peuvent
   remplir. `act_delta` donne le renom (ADR 0805) gagné pendant la saison avec ses faits. Textes dans `data/ui/causes.fr.json`
   (`{amount}` : montant signé à la française ; bloc `terms` : libellés des termes).
5. **Client** : le mode `auto` du rapport de saison devient le Récit (`hud/story.rs`), panneau non modal replié en un clic dans la
   colonne gauche ; `always` et `off` restent. Le trésor et le solde portent leur écart (▲/▼) à côté du chiffre, chaque chiffre un
   bloc « Depuis la saison passée » en tête d'infobulle. Le fil ne coupe plus au milieu d'un nombre (`feed::shorten`).
6. **Invariant seed bots** `swing_causes_per_season` (`sim_campaign::invariants`).

## Conséquences

- Les termes de **prestige** et de **mécontentement** sous la barre décrivent ce qui pèse sur la saison à venir
  (`terms_last_season = false`), pas la saison écoulée : le cœur ne garde pas l'histoire de ces jauges et ne mesure les hommes
  (`men`) que par la prévision, qui clone l'état par phase. Les hommes n'ont donc pas de termes dans la barre ; l'infobulle de
  prévision existante les donne.
- Le journal gagne jusqu'à 6 enregistrements par saison pour le joueur (300 au plus, les plus anciens sortent) : le fil de causes
  « Mon royaume » est plus dense, la liste « Toute l'Europe » inchangée (le cœur ne surveille que le joueur).
- XP-AUDIENCE lit `season_causes` / `SeasonStory.why` et `CauseRecord.id` (champ `cause_id` d'une Affaire) ; ajouter un genre
  de cause demande une variante de `CauseKind`, un constructeur dans `cause_builders.rs` et les textes `kinds`/`factors`.
- Le panneau Récit remplace le modal et le toast du mode `auto` : une perte ou une prise n'ouvre plus de fenêtre d'elle-même.
