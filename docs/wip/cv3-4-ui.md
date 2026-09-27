# CV3-4 — Interface de campagne : postures, rencontres, classes de résultat

Branche : `feat/cv3-4-campaign-ui`. Spec : `docs/design/2026-09-27-campagne-vivante.md` § 4 (UI).
Pont utilisé : `get_stance_options`, `get_last_battle_outcome`, `get_encounter_sites`,
`get_pending_encounters`, `choose_encounter_option` (lots CV3-1, CV3-3).

## Plan / état
- [ ] Squelette (fichiers, API publique).
- [ ] Postures : rangée de boutons à icônes dans le sceau (`StanceBar`), raison du refus en infobulle.
- [ ] Icônes encre : `stance_ambush` (renard), `stance_forced_march` (pas doublés), `stance_entrenched`
      (pavois), `encounter` (rouleau) — composées depuis les sources DA5 existantes, sans dépense.
- [ ] Étendard : pastille de posture (`army_marker.gd`, bloc isolé en fin de `setup`), figurines
      semi-transparentes pour le propriétaire en embuscade.
- [ ] Rencontres : marqueurs (calque 2D projeté), bulle au survol, clic = déplacement ;
      `EncounterWindow` (gabarit `ChronicleWindow`) ouverte quand une rencontre attend.
- [ ] Classes de résultat : bandeau `OutcomeBand` sur l'écran de fin de bataille et notice sur la carte.
- [ ] Bulles : `RichTooltip.HUD_TEXTS` (postures, rencontres) ; Codex `cdx_jeu_postures`, `cdx_jeu_rencontres`.
- [ ] Tests `game/tests/cv3_4_ui_test.gd` ; captures `docs/img/cv3/ui-*.png` (≤ 3).

## Prochaine étape
Squelette.
