# EP2 — Horizon des batailles

Branche : `worktree-agent-a5e4208be5556aa24`. ADR : `docs/decisions/0032-horizon-de-bataille.md`.
Plan d'ensemble : `docs/wip/epic.md`.

## Conception retenue
- **Relief réel** : `cent-ans geo horizon` cuit, par province (point représentatif : centroïde, ou
  point à ~5 km de la côte pour une province côtière), une tuile 25 × 25 km à 100 m (Copernicus
  GLO-90 moyenné, repli tuiles fines 360 m hors emprise), altitude int16 (quart de mètre) + forêt
  (splat.png canal B) dégonflées dans `game/assets/horizon/relief/<prov>.bin`, et un index JSON
  (lon/lat, altitude de référence, profil de ligne d'horizon réel 12-150 km sur 256 azimuts).
- **Raccord** : `BattleTerrain.world_height` mêle le relief généré et le relief réel (recalé sur la
  hauteur moyenne du champ) entre ~0,7 et 3 km du bord du champ ; rivière et côte B5 gardent la main.
  Anneau d'horizon (7 → 13 km, pas 400 m, shader propre, brume selon l'altitude) + mer lointaine.
- **Panoramas peints** : cylindre à ~14,8 km qui suit la caméra, image peinte dont la ligne de crête
  est détourée (alpha) et recalée colonne par colonne sur la ligne d'horizon réelle ; secteurs de
  mer = horizon marin. Choix par région (`data/fx/horizon.json`, règles).
- `--no-horizon` coupe tout (A/B).

## État
- [ ] squelette
- [ ] cuisson relief + index
- [ ] panoramas (sonde puis lot)
- [ ] intégration Godot (anneau, mer, cylindre, silhouettes)
- [ ] brume / fog
- [ ] mesures A/B, captures, ADR

## Prochaine étape
Squelette.
