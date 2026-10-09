# 0283 — Non-agression, alliance défensive, ultimatums de l'IA (lot WH `diplob`)

## Contexte
Rapport WH `docs/wip/wh/diplomatie.md` § 3, points 8 et 9 : un seul `Article::Alliance` (appel aux armes dans les deux sens),
aucun pacte de non-agression, et l'IA ne s'adresse au joueur que pour la paix, l'alliance ou le commerce : jamais d'exigence avant la guerre.

## Décision
- **Alliance défensive.** `Article::Alliance` reste l'alliance militaire (et se lit ainsi dans les sauvegardes et les tests existants :
  le « serde default militaire » est le variant inchangé). `Article::DefensiveAlliance` forme la même alliance et marque les deux
  camps dans `ledger.defensive_allies` (`#[serde(default)]`). Une alliance défensive répond à l'appel aux armes d'un allié attaqué
  mais ne suit pas ses guerres offensives (`ally_war_to_join` l'ignore) ; son engagement pèse `alliance.defensive_commitment` (−5)
  contre −15 ; elle se transforme en alliance militaire en proposant `Alliance`. L'IA propose une alliance défensive quand la militaire serait refusée.
- **Pacte de non-agression.** `Article::NonAggression { turns }`, durée bornée par `non_aggression.min_turns..max_turns`, stocké dans
  `ledger.non_aggression` (échéance, des deux côtés, purgé chaque saison, effacé par `start_war`). Il lie comme une trêve :
  déclarer la guerre à un signataire est un parjure (−40 d'opinion partout, −30 de prestige, historique `Perjury` avec l'article
  `non_aggression`) ; le texte de déclaration est possible mais coûteux, jamais interdit. L'IA n'élit plus un signataire comme cible
  (`war_target`, `ally_war_to_join`). Attitude mutuelle `non_aggression.attitude` tant qu'il tient.
- **Ultimatum.** Un ultimatum est un traité d'exigences seules (`Treaty::is_ultimatum` : cession de province ou tribut à la charge du
  destinataire, rien en échange, aucune paix) : aucun nouveau type. Quand une IA assez forte (`ultimatum.min_power_ratio` en
  puissance de coalition) élit le joueur comme cible, `plan_diplomacy` envoie au lieu de déclarer la guerre : une province réclamée et frontalière
  qui n'est pas la capitale, sinon un tribut de `ultimatum.tribute_income_percent` % du revenu pendant `tribute_seasons`. L'offre dure `OFFER_LIFETIME`
  saisons. Refus ou expiration : `ledger.ultimatum_refused` (casus belli « ultimatum refusé », opinion `ultimatum.refused_attitude`), et la
  saison suivante l'IA déclare la guerre (fenêtre de 4 saisons). Tant que `ultimatum.cooldown_turns` ne sont pas écoulées depuis l'envoi, elle ne
  redemande rien et ne déclare pas la guerre au joueur (l'ultimatum accepté vaut apaisement). Trop faible pour intimider, elle déclare la guerre comme avant.

## Conséquences
- Toutes les valeurs sont dans `data/ai/diplomacy.json` (blocs `league`, `ally_call`, `non_aggression`, `ultimatum`, `treaty_weights.join_war`).
- Les guerres contre le joueur sont désormais précédées d'une exigence quand l'agresseur domine : le joueur peut payer pour les éviter.
- Pas encore : l'IA ne propose pas de pactes de non-agression ; ni durée ni renouvellement de l'alliance.
