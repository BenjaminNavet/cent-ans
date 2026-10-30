# XP-SOBRE (lot D, boucle juge 2, iter. 3)

ADR : `docs/decisions/0830-xp-sobre-rappels-appris.md`.

## État
- Mémoire des rappels : `sim-campaign/src/reminders.rs` (champ `reminders` de l'état, serde default).
- `game_api::note_season_end` (appelé par `client/src/campaign/hud/flow.rs` avant `end_turn_with`),
  `game_api::recall_reminder`, filtrage dans `agenda()` (`learn_after`, `snooze_seasons`, cadence
  `push_first_seasons`/`push_every_seasons` pour `ambition_choice`).
- Jalon de règne : `progress::reign_milestone`, source `reign_milestone`, jalons dans `agenda.json`.
- Textes : raisons du trésorier = lignes projetées du budget (somme = solde net), ligne `treasury_swing_plain`.
- Test : `core/crates/game-api/tests/xp_sobre.rs`.

## Points ouverts
- Bouton « Rappeler » (client) : `Agenda.snoozed` et `recall_reminder` existent, la ligne « Rappels en
  sourdine » s'affiche dans `news_summary`, mais aucun bouton n'est câblé (panneaux livres : XP-R3b).
- Scores attendus des jalons (2/4/6, marge 1) à recalibrer avec les seed bots ; `enemy_army`,
  `mission_offer`, décisions de chronique se répètent toujours (hors périmètre, M4/M5).
- Reste à la fusion : clippy complet, smoke, schéma JSON des données.
