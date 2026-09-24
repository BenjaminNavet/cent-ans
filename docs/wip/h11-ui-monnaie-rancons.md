# WIP — H11 interface Monnaie, Rançons, Ordre de chevalerie, Encyclopédie ↔ Codex

Conception : `docs/design/2026-09-23-histoire-et-savoir.md` §5.1-5.2 ; API : `docs/design/h5-h6-api.md`.
Modèle : `docs/wip/h9-ui-table-medecine.md` (`table_section.gd`).

## État
- [x] `game/scripts/ui/coinage_section.gd` : niveau, `price_level` + jauge, seigneuriage/refonte
      (prévus, saison passée), sélecteur 4 niveaux (`RichTooltip.coinage`), refus en rouge, texte
      aux liens `cdx_mutations_monetaires`, `cdx_nicole_oresme`, `cdx_livre_tournois`.
- [x] Budget : lignes « Seigneuriage (revenu) » / « Refonte (administration) » (`faction_panel.gd`).
- [x] `game/scripts/ui/ransom_panel.gd` : Nos captifs (comptant / 2-6 échéances / céder la province
      exigée), Nos prisonniers (argent, province cessible, garder ; parole), dettes et échéances ;
      portraits, noms → fiche personnage (signal `character_selected` de l'ancêtre `MapUI`), ✠ → Codex.
      Ouverte par le bouton « Captifs et rançons » du panneau de faction.
- [x] `game/scripts/ui/chivalry_section.gd` : ordre fondé (nom lié au Codex, membres, bonus, brisé)
      ou options de fondation (coût, prestige, refus). `RichTooltip.chivalric_order`.
- [x] Genres `coinage` / `ransom` / `chivalry` : `season_report.gd` (rubrique « Monnaie, rançons et
      chevalerie », `KIND_STYLES`), `news_letters.gd`, `alerts.gd` (`ransom_alerts` : captif, échéance).
- [x] Encyclopédie → Codex (« ✠ Fiche historique ») et Codex → Encyclopédie (« Voir dans l'encyclopédie »,
      groupe `encyclopedia`).
- [x] Smoke `_run_coinage_ransom` vert seul (`CENT_ANS_SMOKE_ONLY=coinage_ransom`).
- [x] Smoke complet exit 0 ; captures `docs/img/coinage.png`, `docs/img/ransoms.png`
      (`godot --path game --script res://tests/coinage_screenshot.gd`).
- [x] Panneau de faction : contenu dans un défilement vertical (sections plus longues).

## État : terminé (à fusionner)

## Limites connues
- Pas de bouton dans le bandeau (`map_ui.gd` non touché) : accès par le panneau de faction
  (clic sur le blason). Les alertes `ransom` n'ont pas d'action de clic dédiée (`campaign_map.gd`
  non touché) : il faudrait ouvrir `FactionPanel.toggle_ransoms()`.
- La fiche personnage est ouverte en émettant `character_selected` du premier ancêtre qui l'expose
  (`MapUI`), sans modifier la carte.
- Les captures et le smoke utilisent des rançons simulées (aucun captif au départ de 1337) ; les
  ordres envoyés au pont avec ces données sont refusés (« ce personnage n'est pas captif »).
- `CampaignAlerts.table_medicine_alerts` relaie désormais aussi les événements `coinage`,
  `ransom`, `chivalry` du joueur (tous les genres de `SeasonReport.KIND_STYLES`).
- Journal de `map_ui.gd` : genres en texte simple (glyphes dans `SeasonReport.KIND_STYLES`).
- Dette sans faction du captif : portrait remplacé par un carré neutre si pas de portrait peint.
