# M5b — équilibrage du mouvement libre et documentation (état)

Spec : `docs/design/2026-09-24-mouvement-libre.md` § 7-8. Suivi : `docs/wip/mouvement-libre.md`.
Branche : `worktree-agent-a3c4f4bdfe00c9af3`, depuis main fa7efb3c (M1-M4).

## État

- [x] Mesure de référence (`settlements_probe 50 1..8`, release) : identique au M3 de `m3-tour-ia.md`
- [x] Débandades : refuge (règle « neutre » C7a sur la grille) + défaite lourde (`heavy_defeat_losses_percent` 42) → 1,2 / graine
- [ ] Passages de fleuves (croisements Itiner-e)
- [ ] Saône en aval de Chalon
- [ ] Somme / Blanchetaque
- [ ] Portée (Paris → Orléans, Paris → Bordeaux)
- [ ] Écosse
- [ ] Manuel, codex, tutoriel
- [ ] Fusion de main, vérifications finales

## Prochaine étape

Passages de fleuves : rayon des croisements Itiner-e (`road_crossing_radius_km`), Saône (tronçon « Sane » de rivers.geojson = Saône aval, encodage cassé), Somme.
