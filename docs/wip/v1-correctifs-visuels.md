# V1 — correctifs visuels rapides (audit A1)

Source : `docs/audit/a1-visuel.md`. Captures avant/après : `docs/audit/captures/v1/`.

## État

- [x] A1-02 zone de déploiement discrète : `game/shaders/deployment_zone.gdshader` (liseré à largeur écran minimale, halo, hachures en bande de 22 m, fondu 140→700 m de caméra) ; captures `02_*`
- [x] A1-03 brouillard non noir : `terrain.gdshader` (désaturation, brume parchemin, nuages bas fbm animés par TIME, bord fondu 3 px sur les frontières vu/voilé), `campaign_minimap.gdshader` (voile clair) ; captures `03_*`
- [ ] A1-04 pas d'écume autour des lacs
- [ ] A1-01 régiments lisibles à distance
- [ ] A1-06 caméra de bataille TW + gros plan de mêlée
- [ ] bandeau « Temps clair » en `--weather=rain`

## Prochaine étape

A1-04 : écume réservée à la mer (water.gdshader, sea.gd, coast_renderer.gd, lacs du terrain).
