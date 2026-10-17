# Lot overlays (correctifs post-playtest 17/10)

Branche `fix/overlays`. ADR 0446.

## Fait (code, tests unitaires verts)
- 1/2/3 : avis et sous-titres du conseiller sous les panneaux et modales (`UiTier::Toast` = 5) ; tutoriel, guide féodal, offre de mission attendent la file de modales ; guide féodal attend le tutoriel ; conseiller muet (réplique mise en file) pendant modale / pause / carte du tutoriel.
- 4 : `KeyMap::set_suspended` pendant la Pause (`menu/pause.rs::suspend_shortcuts`).
- 5 : `modal_escape` consomme l'arête Échap, `pause_escape` passe après.
- 6 : `court::shortcuts` branche J (Conseil) et C (fiche du souverain).
- 7 : `{key:ACTION}` (`KeyHints::expand`) dans `next_hints.json` et `steps.json`.
- 8 : règle du core = réserve plafonnée (`research_reserve_turns`), puis perdu ; textes alignés.

## À vérifier par dump
Voir la fin de la note une fois les scénarios passés.

## Hors lot
- Textes `data/ui/tooltips.json` (hud_court / hud_technologies / hud_diplomacy) avec touches C/T/P codées en dur : non utilisés par le client (les infobulles de la barre lisent la keymap).
