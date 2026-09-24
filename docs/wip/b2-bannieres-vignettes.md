# B2 — Bannières flottantes, cartes-vignettes, écran de fin (lot T2)

Branche : `worktree-agent-a0c31d412af478ab6`. Plan : `docs/design/2026-09-24-rapprochement-total-war.md` (B2).
Ne pas toucher : `battle_meshes.gd`, `battle_soldiers.gd`, `battle_soldier.gdshader` (lot B1).

## Plan
1. [x] `battle_unit_markers.gd` : repères 2D projetés (Control unique plein écran, `_has_point` sur
       les repères), icône de classe, couleur du camp, barres effectif/moral, pastilles d'état
       (déroute clignotante, tir, charge, mêlée, épuisement), étoile du général, sélection (or) et
       survol (liseré blanc + nom), clic = sélection, clic droit sur un ennemi = attaque, touche U =
       masquer, décalage des chevauchements (côtés puis empilement, trait de rappel). Les Label3D
       (glyphe + nombre) sous le drapeau 3D sont retirés ; mât et drapeau restent.
2. [x] `unit_card.gd` en vignette 64×94 : illustration `res://assets/illustrations/<type>.jpg` si
       présente (recadrée en portrait), sinon composition couleur du camp + blason + icône de
       classe ; effectif en gros, barres fines moral/fatigue/munitions, pastille d'état, étoile,
       groupes ; voile + croix si anéantie. Infobulle riche (état, effectif, formation, groupes).
3. [x] `battle_result_screen.gd` : verdict (Victoire décisive / chèrement acquise / Défaite
       écrasante / honorable…), écus, bilan engagés/pertes, tableau par régiment, mentions
       (anéantis, général tombé/capturé → rançon, pas de quartier, déroute), retour campagne.
       L'ancien `end_panel` du HUD est supprimé. Option `--result-shot` pour la capture.
4. [ ] Smoke étendu (`_check_battle_markers_b2`, écran de fin) : en cours de validation.
5. [x] Captures `docs/img/b2/` : `bannieres_vue_ensemble.png`, `bannieres_rapprochees.png`,
       `bannieres_detail.png`, `cartes_vignettes.png`, `ecran_fin.png`.

## État / prochaine étape
Smoke à repasser après la correction du dépilement (vue lointaine du smoke : 14 repères se
chevauchaient). Les illustrations `game/assets/illustrations/unit_*.jpg` n'existent que dans la copie
principale (non suivies) : copiées localement pour les captures, non commitées ; sans elles la carte
retombe sur la composition blason + icône.
