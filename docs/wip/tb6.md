# TB6 — lumière et atmosphère (note de reprise)

Spec : `docs/design/2026-10-02-campagne-tob.md` § 3 « TB6 ». Branche `feat/tb6`, worktree
`/Users/jean_hubert/dev/gp-tb6`. Rendu Godot seulement (`core/` intact).

## État
- [x] Squelette du test `game/tests/tb6_light_test.gd`.
- [ ] 1. Taches sombres aux bords nets par temps clair (cause à trouver : ombres de nuages, masque météo, nuées).
- [ ] 2. Lumière dorée par défaut (soleil bas, ombres longues, teinte chaude ; hiver bleuté conservé).
- [ ] 3. SSIL mesuré (`ss_shot.gd --bench`, 400 et 90) ; SDFGI seulement si SSIL adopté.
- [ ] 4. Brume du matin dans les vallées (météo « brume », masque B), jamais sur la province sélectionnée.
- [ ] ADR 0156, `game/tests/tb6_shot.gd`.

## Prochaine étape
Mesure de référence (`--stats` à 1100 et 400, bench) puis point 1.
