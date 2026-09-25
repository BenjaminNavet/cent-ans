# WIP — B1 Infra bulles (T universel)

Branche : `feat/b1-bulles-infra` (worktree agent). Spec : `docs/design/2026-09-25-bulles-partout.md`.

## État
- [ ] T universel (bulle non épinglée > infobulle riche > infobulle simple du contrôle survolé)
- [ ] Chaîne parent → enfant (méta `parent`, épingler un enfant épingle ses ancêtres)
- [ ] Pieds « T : maintenir ouverte » / « Clic droit : détacher » / « Clic : lire la fiche »
- [ ] `gameplay` : encadré « En jeu » (fenêtre) + ligne dans la bulle
- [ ] Titres d'infobulles riches liés via `entity`
- [ ] Conversion infobulles simples → riches (map_ui, province_panel, faction_panel, battle_hud, pre_battle_dialog, army_strip)
- [ ] Tests smoke + capture `docs/img/bulles-bg3.png`

## Prochaine étape
Implémenter `codex_bubbles.gd`.
