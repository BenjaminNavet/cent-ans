# Lot HIST4 : historique et UX (boucle juge 2, itération 2)

Source : `out/juge2/iter-02-histux.md`. Branche : worktree `agent-afbbd13049fc29b57`. ADR : `docs/decisions/0798-hist4-conseil-monnaies-contrastes.md`.
REPRISE faite : rebase sur main OK ; vérifications terminées (voir ci-dessous).

## Fait
- 2 Conseil : `council::office_allows` (rôles/traits exclus, `holder_roles` aumônier=clerc, `preferred_roles`, `title_keyword` pour les
  dates de charge) ; charges restreintes attribuées d'abord ; 3 prélats anglais + maison « Église d'Angleterre » (PNG ajouté).
  Tests `rg13a_council` verts (17).
- 3 Lothian : infirmé (ADR 0730), texte du lieu corrigé.
- 4 Texte France : « revenu le plus fort ». Accroches non calculées (reporté).
- 6 Monnaies : `data/economy/currencies.json`, `currency` par faction, `data_model::currency` (MONEY, set_display), client
  `Sim::sync_currency`, `{money}` dans les textes de données.
- 7 Déficits : cause = ponction d'opulence sur trésor de 4 saisons ; `treasury_max_income_seasons` 4 -> 1, `starting_fit.json` régénéré.
  Nets mesurés : Mamelouks +662, Byzance +1251, Lituanie +161, Teutons -1.
- 9 Palette (style.rs) + test WCAG.
- 10 : élision (Île-de-France), « 1 an/2 ans », avis frontière (`text_foreign`), An-/Al-Nasir, capitales (Vienne, Rendsburg),
  Leulinghen/trêve de 28 ans, Bulle d'or (hiver 1355), codex objectifs. Apostrophes des voix : par conception (clé de cache audio).
  Étoile : correct. Sanudo/Blois : laissés (incertains).

## Vérifications finales
- core : fmt, clippy -D warnings, test --workspace : verts. Client : clippy vert ; tests 1507 ok, 1 échec `markers::dn::real_table_and_glbs`
  (GLB city_west absent du worktree, sans lien). pytest schémas/conseil/heraldry : 2422 ok. Seed bots rapides : 0 constat.
- Non traité : entête « Acte 1 sur 3 » identique pour toutes les factions ; accroches de choix non calculées depuis les données.
