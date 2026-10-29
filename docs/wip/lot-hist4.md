# Lot HIST4 : historique et UX (boucle juge 2, itération 2)

Source : `out/juge2/iter-02-histux.md`. Branche : worktree `agent-afbbd13049fc29b57`.

## État

- 2 Conseil : règle générique dans `core/crates/sim-campaign/src/council.rs` (`office_allows`) : rôles et traits exclus
  (consort, prétendant, disgracié), `holder_roles` par charge (aumônier = clerc, chancelier/trésorier = civils, connétable =
  homme de guerre), `title_keyword` (un titre homonyme postérieur à l'année courante exclut). Les charges restreintes
  choisissent en premier. Données : `data/council/offices.json`, 3 prélats anglais ajoutés (Stratford x2, Bury).
- 3 Lothian : INFIRMÉ comme erreur d'état initial (ADR 0730 acte « Lothian écossais, garnison anglaise au seul château ») ;
  le texte du lieu était contradictoire, corrigé. L'événement de 1341 reste cohérent.
- 4 Texte France « trésor » : corrigé en « revenu le plus fort » (vérifié : revenu France le plus fort).
- 6 Monnaies : `data/economy/currencies.json` + `currency` par faction (schéma, loader `GameData::currency_of`).
  Nom affiché via `data_model::currency` (monnaie d'affichage fixée par le client à la création/chargement de campagne,
  `Sim::sync_currency`), `MONEY` dans les textes du cœur, `{money}` dans les textes de données.
- 7 Déficits des petites factions : voir section dédiée ci-dessous.
- 9 Accessibilité : palette `style.rs` (or, juste, tabs, bon/mauvais), test WCAG.
- 10 Finitions : voir le rapport.

## Reste
- Contrôle seed bots rapides, clippy/tests core et client, pytest.
