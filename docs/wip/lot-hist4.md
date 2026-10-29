# Lot HIST4 : historique et UX (boucle juge 2, itération 2)

Source : `out/juge2/iter-02-histux.md`. Branche : worktree `agent-afbbd13049fc29b57`. ADR : `docs/decisions/XXXX-hist4-conseil-monnaies-contrastes.md`.
PAUSE demandée par le joueur : travail arrêté, état committé.

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

## Reste (prochaine étape)
- Lancer : `cd core && cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo test --workspace` (clippy core OK ;
  tests workspace non terminés : régénérations de tests de graine possibles).
- Client : `cargo clippy --features dev --all-targets -- -D warnings && cargo test --features dev` (jamais compilé depuis les
  changements : `fr::symbol`, hint `foreign_provinces`, `currency_title`, `StartSheet.currency_symbol`).
- `uv run --project tools pytest` : tout passait sauf heraldry (corrigé, 5 passed) ; relancer test_schemas (next_hints corrigé).
- Seed bots rapides ; entête « Acte 1 sur 3 » identique pour toutes les factions (non traité).
