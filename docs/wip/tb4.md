# TB4 — conséquences visibles de la guerre et des fléaux (note de reprise)

Spec : `docs/design/2026-10-02-campagne-tob.md` § 3 « TB4 ». Branche `feat/tb4`, worktree
`/Users/jean_hubert/dev/gp-tb4`. Rendu Godot seulement (`core/` intact).

## État
- [x] Squelette du test `game/tests/tb4_scars_test.gd`.
- [ ] 1. Brûlis (`terroir_burn`) par-dessus la carte de couleur (SS2) et les matières (HB3).
- [ ] 2. Peste : fosses, croix sur les portes, charrette des morts (3D, palier proche).
- [ ] 3. Champ de bataille marqué quelques tours (tertre, corbeaux, débris), durée dans `data/`.
- [ ] 4. Siège : engins en construction (N7) visibles dans le camp au fil des tours.

## Prochaine étape
Point 1 : déplacer l'appel `terroir_burn` après `sg_apply` / `hb_apply` dans `terrain.gdshader`.

## Points ouverts
- (à compléter)
