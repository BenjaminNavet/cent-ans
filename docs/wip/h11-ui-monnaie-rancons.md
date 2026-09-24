# WIP — H11 interface Monnaie, Rançons, Ordre de chevalerie, Encyclopédie ↔ Codex

Conception : `docs/design/2026-09-23-histoire-et-savoir.md` §5.1-5.2 ; API : `docs/design/h5-h6-api.md`.
Modèle : `docs/wip/h9-ui-table-medecine.md` (`table_section.gd`).

## Plan
- [ ] `game/scripts/ui/coinage_section.gd` : niveau, `price_level` + jauge, seigneuriage/refonte
      (prévus, dernier tour), sélecteur 4 niveaux (infobulle riche, refus), texte Codex. Hook `faction_panel.gd`.
- [ ] Budget : lignes Seigneuriage / Refonte (`faction_panel.gd`).
- [ ] `game/scripts/ui/ransom_panel.gd` : Nos captifs (comptant / 2-6 échéances), Nos prisonniers
      (argent, province, parole, garder), dettes. Bouton d'accès depuis `faction_panel.gd`.
- [ ] `game/scripts/ui/chivalry_section.gd` : ordre fondé ou options de fondation. Hook `faction_panel.gd`.
- [ ] Genres `coinage` / `ransom` / `chivalry` : `season_report.gd`, `news_letters.gd`, `alerts.gd`.
- [ ] Encyclopédie → Codex (« Fiche historique ») et Codex → Encyclopédie (« Voir dans l'encyclopédie »).
- [ ] Smoke `_run_coinage_ransom` ; captures `docs/img/coinage.png`, `docs/img/ransoms.png`.

## Prochaine étape
Squelettes des trois scripts.
