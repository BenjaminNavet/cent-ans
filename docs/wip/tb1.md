# TB1 — saisons visibles à tous les zooms (note de reprise)

Spec : `docs/design/2026-10-02-campagne-tob.md` § 3 « TB1 ». Branche `feat/tb1`, worktree
`/Users/jean_hubert/dev/gp-tb1`. Rendu Godot seulement (`core/` intact).

## État
- [x] Squelette du test `game/tests/tb1_seasons_test.gd`.
- [ ] 1. Delta saisonnier par-dessus la carte de couleur (`satellite_ground`, `hb_ground`), neige de plaine lisible en vue large.
- [ ] 2. Mer selon la saison (hiver sombre et gris, écume de tempête).
- [ ] 3. Étalonnage par saison (`data/ui/` + schéma).
- [ ] 4. Neige sur les toits des villes 1:1 (uniform `snow`).

## Prochaine étape
Point 1 : calculer la couleur procédurale « été » et la couleur de saison, appliquer leur écart
après `sg_apply` / `hb_apply`.

## Points ouverts
(aucun pour l'instant)
