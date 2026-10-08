# ADR 0202 — Une proposition diplomatique est un traité d'articles

Statut : accepté (08/10, chantier SC, lot `treaty`).

## Contexte

La diplomatie avait deux voies. L'enum `Proposal` (Peace, Truce, Alliance, Vassalage, Marriage, Obedience, Protection, Arbitration, PeaceSummons, Treaty) avait sa propre évaluation (`diplomacy::evaluate`, accord déterministe si le score dépasse 0), sa propre validation (`check_proposal`), son application (`apply_proposal`) et son texte (`offer_text`). Parallèlement, `negotiation.rs` manipulait des « articles » (traités à plusieurs articles, chance logistique, contre-propositions). L'évaluation d'un article de paix, d'alliance, de mariage ou de vassalité appelait déjà l'ancienne évaluation en lui retirant l'attitude : les deux voies étaient imbriquées. `article_value` (290 lignes), `evaluate` et `context_reasons` mêlaient les facteurs et des dizaines de constantes numériques.

## Décision

- `Proposal` disparaît. Une proposition est un `Treaty { articles: Vec<Article> }` ; une alliance simple est un traité d'un article. `Offer.proposal` est un `Treaty`.
- `Article` gagne `Mediation { turns }` (trêve par médiation du pape ou d'un héraut, avec son bonus de +30), `Obedience`, `Protection`, `Arbitration` et `PeaceSummons` (ordres d'un suzerain ou du schisme : `Article::is_imposed`, exécutés sans comptabilité de traité).
- Composite : chaque article sait `check`, `value` (raisons pondérées pour le destinataire), `apply` et `label` (modules `negotiation/{check,value,apply,label}.rs`). Le traité est la somme de ses articles ; `context.rs` porte les considérations générales, un facteur par fonction.
- Une seule voie d'évaluation (`negotiation::evaluate_treaty`, renvoie la chance, l'acceptation et une `ReasonList`), une seule voie d'application (`apply_treaty`). `diplomacy::evaluate`, `Evaluation`, `check_proposal`, `apply_proposal`, `offer_text` par variante et la paix « G5 » de repli (`peace_terms`, branche `!negotiation.enabled` de `plan_diplomacy`) sont supprimés.
- `ReasonList` : la liste des raisons (texte français, points), sans entrée nulle, avec les plus lourdes objections pour les refus.
- Constantes : `data/ai/diplomacy.json` → `treaty_weights` (paix, alliance, vassalité, mariage, échanges, contexte), schéma `ai_diplomacy.schema.json` mis à jour, struct `TreatyWeights` dans `data-model`. Les valeurs par défaut reprennent les anciens littéraux.
- Les ordres `ProposePeace`, `ProposeAlliance`, `DemandVassalage`, `ProposeFactionMarriage` restent (le GDScript les envoie) ; `Order::proposal` les convertit en traité. `Treaty::peace_terms` traduit « paix + provinces + tribut » en `Peace`, `CedeProvince` et `Gold`.

## Conséquences

- Les traités d'IA (`plan_peace`) et toute proposition passant déjà par `evaluate_treaty` ont exactement les mêmes valeurs : les facteurs ont été déplacés sans changement de nombre.
- Changements réels de mécanique, pour les propositions simples (alliance, vassalité, mariage, paix avec provinces, trêve de médiation, appels du joueur) :
  - elles passent par la chance logistique et le contexte (attitude /3 ou /5, confiance, menace, prudence −3) au lieu du score brut avec attitude /2 ; l'acceptation reste `score >= 0` pour le joueur ; entre deux IA, un tirage déterministe `answer_roll` remplace l'acceptation immédiate ;
  - la reconnaissance des conquêtes par un traité cédant des provinces, et le tribut proportionnel, disparaissent : le tribut devient un article `Gold` (versé en une fois, trésor du donneur vérifié), les provinces des `CedeProvince` ;
  - tout traité signé, même d'un article, laisse la trace « Traité signé » (+5 d'opinion plafonné), l'historique et le journal ;
  - la paix de repli G5 (`negotiation.enabled = false`) n'existe plus ; le drapeau ne gouverne plus que les buts de guerre et la fatigue.
- Format de sauvegarde : une offre s'écrit désormais comme une liste d'articles (anciennes sauvegardes abandonnées, ADR 0186).
- L'évaluation d'un article sans valeur (appel féodal) ne produit plus de raison à 0.
