# TS-T3 — atmosphère de la carte de campagne (ADR 0710)

Branche `ts/t3`, worktree `../gp-ts-t3`. Plan : `docs/wip/ts-style-total-war.md`.

## Conception
- La passe plein écran `terrain/post.rs` (après tonemapping, caméra de campagne seulement, l'UI a sa propre caméra) devient une passe maison qui lit aussi la profondeur de la vue : position monde par pixel → brume de vallée selon l'altitude, perspective atmosphérique gris-bleu, froid, puis étalonnage désaturé. S'applique au monde entier (arbres compris) sans toucher `terrain.wesl` (T1) ni `vegetation/` (T2).
- Données : `data/fx/campaign_post.json` (schéma `fx_campaign_post.schema.json`), interrupteur global `enabled` + un par bloc.
- Flocons (`weather/precip.rs`) : quads face caméra, disque flou, tailles/alpha variables, dérive.

## État
- [ ] squelette (données, schéma, note)
- [ ] passe profondeur + brume + aérien + froid + étalonnage
- [ ] flocons doux
- [ ] vérifs + captures

## Prochaine étape
Implémenter la passe dans `terrain/post.rs` / `post.wesl`.
