# WIP — B1 Infra bulles (T universel)

Branche : `feat/b1-bulles-infra` (worktree agent). Spec : `docs/design/2026-09-25-bulles-partout.md`.

## État
- [x] T universel (`CodexBubbles.pin_current`) : bulle non épinglée (ou en attente) > infobulle riche > infobulle simple du contrôle survolé (après le délai d'infobulle)
- [x] Chaîne parent → enfant (méta `parent`, `parent_of`) : épingler épingle les ancêtres, détacher détache les descendantes, une bulle retirée rattache ses filles à sa parente
- [x] Pieds « T : maintenir ouverte » / « Clic droit : détacher » / « Clic : lire la fiche » ; pied des infobulles riches
- [x] `gameplay` : encadré « En jeu » (fenêtre, `gameplay_box`) + ligne « En jeu : » (1re phrase) dans la bulle
- [x] Titres d'infobulles riches liés via `entity` (`RichTooltip.entity_name`, `title_entry`), édits compris
- [x] Conversions : army_strip (cartes de régiment, en-tête), battle_hud (ordres, retraite), pre_battle_dialog (boutons, renforts, composition, commandement), map_ui (recherche via `RichBox`, choix du général). province_panel et faction_panel étaient déjà riches.
- [x] Smoke complet vert (étape seule : `CENT_ANS_SMOKE_ONLY=codex_bubbles`), pytest vert (418)
- [x] Capture `docs/img/bulles-imbriquees.png` (`godot --path game --script res://tests/bubbles_screenshot.gd`)

## Prochaine étape
Terminé ; reste à fusionner (voir limites dans le rapport : T sur un contrôle survolé exige le délai d'infobulle, sceau du chef et boutons de vitesse restés simples mais verrouillables).
