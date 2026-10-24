# ADR 0789 — Conseiller de saison, annonce de la posture de l'IA, oracle d'équilibre des seed bots

Date : 2026-10-24. Statut : accepté (lot GAMEPLAY2). Complète les ADR 0752 (bandeau du conseiller), 0782 (doctrines), 0348 (seed bots).
Source : juge game design it. 4, constats C3, C5, C6, C7.

## Contexte

- **C3.** 32 colonies sans chantier n'étaient qu'un compteur Info parmi six alertes.
- **C5.** Le conseiller n'a parlé qu'une fois en six tours ; aucune règle de données ne dit quand il parle.
- **C7.** La doctrine d'un voisin (ADR 0782) change avec le souverain ou après une défaite sans que le joueur le sache.
- **C6.** `seed-bots run --quick` ne collectait pas les métriques (`balance: balance || !quick`) : `metrics: []`, l'oracle ne
  pouvait pas échouer ; de plus 9 bandes sur 14 étaient `to_initialise` et les bandes de la passe longue (150 tours) ne
  conviennent pas à 20 tours.

## Décision

1. **Conseils de saison en données** : `data/rules/advisor.json` (schéma `advisor_rules.schema.json`, `data_model::AdvisorRules`).
   Cinq conseils : trésor qui fond (danger), colonies sans chantier et épargne qui dort (occasions), bilan d'année et acte qui
   n'avance pas (fin de saison, hiver). Chaque conseil a son délai (`cooldown_seasons`) ; `max_per_season` (1) borne le
   bavardage, le danger passe avant l'occasion avant le bilan ; `quiet_seasons` laisse parler le guide d'abord. Textes français
   en données avec variables (`{sites}`, `{treasury}`, `{levy}`, `{act}`, `{score}`, `{objective}`…).
2. **Moteur dans `core/`** : `sim_campaign::advisor::update`, à la fin de chaque tour, évalue les déclencheurs sur l'état et
   garde les notes de la saison (`CampaignState::advisor`, `serde(default)`). `game-api::turn_readiness` les sert comme alertes
   `advice` (première ligne = titre). Le bandeau (`advisor_band.rs`) est inchangé : le client ne fait que rattacher le kind
   `advice` au conseil `advice` de `data/ui/next_hints.json` (une ligne dans `hint.rs`). L'alerte `free_slot` dit désormais ce que
   la caisse peut payer.
3. **Posture de l'IA.** À chaque fin de tour, la doctrine de chaque maison est comparée à la dernière vue ; si elle change pour une
   maison en guerre avec le joueur ou voisine de son domaine, une ligne de chronique (`EventKind::Diplomacy`) l'annonce :
   « {faction} change de posture : « {doctrine} ». {intent} » (textes de `advisor.json`). Les doctrines sont enregistrées en
   silence à la création de la campagne : seul un changement est une nouvelle.
4. **Oracle des seed bots opérant.** Les métriques sont toujours collectées. Quatre métriques s'ajoutent (batailles, provinces
   tenues, trésor final, score de règne de la maison du bot). Deux jeux de bandes : `seed_bots_bands.json` (passe longue, 150
   tours) et `seed_bots_bands_quick.json` (20 tours, chargé par `--quick`), toutes initialisées et bornées des deux côtés dans
   le second. `CENT_ANS_DATA_DIR` permet de jouer une copie des données (régression volontaire).
5. **Sonde du score de règne** : `seed-bots/examples/reign_probe.rs` (profils passif / IA / décisif, ADR 0788).

## Vérification de l'oracle (régression volontaire)

Passe rapide propre : 0 constat. Avec `hold_guard_percent: 0` (les acquis rapportent à nouveau des points) : 7 dérives
`reign_score` (38 à 54 pour une borne à 30). Avec `tax_efficiency: 0` : dérives `bankruptcies_per_game` (240 pour une borne à 8),
`battles_per_game` (0), `treasury_end` (−111 333).

## Conséquences

- Les bandes rapides sont celles d'une graine à 20 tours : elles servent à détecter une casse, pas à régler un équilibre fin ; la
  passe longue garde ce rôle. Une règle qui déplace légitimement une métrique hors borne exige de réviser la bande dans le même lot.
- Les textes de conseil sont en données : les modifier ne demande aucun code. Ajouter un déclencheur demande une variante de
  `AdviceTrigger` et son évaluation dans `advisor.rs`.
