# DATA3 — petites corrections de règles et de données

## 1. Maîtres d'ordres militaires (vœu de chasteté)
- `mariage_de_notables` n'existe pas ; la règle vit dans `dynasty::marriage_blocker`, partagé par
  `check_marriage`, `propose_marriage` et `marriage_candidates` (donc par l'IA : `arrange_marriage`
  / `marriage_partner`, sans toucher à `core/crates/ai`).
- Aucun champ existant ne repère un maître d'ordre de façon fiable (maison « Ordre … » pour trois,
  « Villeneuve » pour l'Hôpital) : champ optionnel `celibate: true` (schéma `character`,
  `data_model::Character`, `CharacterState`), nouvelle erreur `MarriageError::Celibate`.
- Fiches marquées : Juan Núñez de Prado (Calatrava), Hélion de Villeneuve (Hospitaliers), Eberhard
  von Monheim (Teutoniques), Dietrich von Altenburg (Livonien). Alcántara, Santiago, Templiers : pas de
  fiche en données.
- Test : `campaign_life::m4::military_order_masters_take_a_vow_of_chastity`.

## 2. Murcie
Voie la moins invasive : affichage de `prov_murcia` renommé « Andalousie et Murcie »
(local « Andalucía y Murcia »), la capitale restant Séville. Aucune colonie ni province ajoutée,
`starting_fit` et `data/map` inchangés.

## 3. Tributs de traité : refusé
`ledger.tributes` est payé directement trésorerie à trésorerie (`negotiation.rs`), annulé par la
guerre / la disparition du créancier, lu par l'IA (`ai/campaign.rs`, zone d'un autre lot), par
`treaty_explain` et des tests de traités. `ObligationKind::Tribute` n'a aucun chemin de
crédit au créancier (`creditor` n'est qu'affiché). Brancher le producteur sans migrer tout cela
doublerait le paiement ; migrer exige de toucher `core/crates/ai`. Non fait ; à traiter en un lot
dédié (migration de `ledger.tributes` vers `finance.obligations` + crédit du créancier).

## 4. Élisions manquantes
`events::de` / `ui::kit::fr::de_place` appliqués aux libellés dynamiques : armées, vassal, Maison,
Siège, Captif, Prisonnier, traversée, parti, garnison, appel aux armes, trône, autorité, Assaut/Sortie,
gouverneur, offre expirée, nouvelle armée. Aucune chaîne statique « de Angleterre » trouvée.
