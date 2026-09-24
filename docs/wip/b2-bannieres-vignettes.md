# B2 — Bannières flottantes, cartes-vignettes, écran de fin (lot T2)

Branche : `worktree-agent-a0c31d412af478ab6`. Plan : `docs/design/2026-09-24-rapprochement-total-war.md` (B2).
Ne pas toucher : `battle_meshes.gd`, `battle_soldiers.gd`, `battle_soldier.gdshader` (lot B1).

## Plan
1. [ ] `battle_unit_markers.gd` : repères 2D projetés (Control unique, `_has_point` sur les repères),
       icône de classe, couleur du camp, barres effectif/moral, états (déroute clignotante, tir,
       charge, épuisement), sélection/survol, clic = sélection, clic droit ennemi = attaque,
       touche U = masquer, dépilement vertical des chevauchements. Label3D du nombre retirés.
2. [ ] `unit_card.gd` en vignette : illustration `res://assets/illustrations/<type>.jpg` si présente,
       sinon composition icône + blason + couleur de camp ; effectif en gros, barres fines, état en icône.
3. [ ] `battle_result_screen.gd` : titre selon pertes, écus, tableau des pertes, mentions, retour.
4. [ ] Smoke étendu, captures `docs/img/b2/`.

## État
Squelette.
