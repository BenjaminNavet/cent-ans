# WIP — B4 Bâtiments et ressources (bulles partout)

Branche : `b4-batiments`. Conception : `docs/design/2026-09-25-bulles-partout.md`.

## Plan
- 30 bâtiments (`data/buildings/`), 10 ressources (`data/resources/`), 7 repères (`data/landmarks/`).
- Fiches existantes enrichies (entity + gameplay) plutôt que dupliquées :
  `cdx_apothicaires` (bld_apothecary), `cdx_jardin_des_simples` (bld_herb_garden),
  `cdx_hotel_dieu` (bld_hotel_dieu, au lieu de tech_hospital_reform), `cdx_ost_feodal` (bld_muster_field),
  `cdx_enluminure` (bld_scriptorium), `cdx_universite_paris` (bld_university),
  `cdx_gabelle` (res_salt), `cdx_tin_stannaries` (res_tin).
- Nouvelles fiches : une par autre bâtiment/ressource + `cdx_londres` (repère sans fiche).
- Liens vers les fiches mécaniques B2 déclarés dans `data/codex/_b4_links.md` (pas dans `_todo.md` :
  le validateur refuse un `_todo.md` qui liste des fiches déjà écrites, ce qui casserait la fusion avec B2).
- Audit : `docs/histoire/audit-2026-09-25-batiments.md`.

## État
- [x] squelette
- [ ] fiches bâtiments
- [ ] fiches ressources
- [ ] repères
- [ ] corrections des JSON + liens dans les descriptions
- [ ] validateur + pytest + cargo test

## Prochaine étape
Rédiger les fiches bâtiments (ordre alphabétique des ids).
