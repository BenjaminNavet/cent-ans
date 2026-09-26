# SZ5 — pluie crédible à tous les paliers de zoom (défaut S7)

Worktree `agent-adeddc2012349e482` (depuis `main`). Lien : `docs/wip/sz-suites-zoom.md`,
`docs/wip/zg7c-recette.md` (défaut S7 : « pluie en bâtonnets blancs géants au palier site »).

## Diagnostic
`CampaignWeatherView._update_particles` (`game/scripts/map/campaign_weather_view.gd`) dimensionne
les gouttes avec `quad.size = base * maxf(k, minf(0.3, k * 1.36))`, `k = distance / 100`.
1 unité monde = `MapData.meters_per_px` (≈ 719 m) : à `distance = 1,8` (palier site,
`ZoomTiers.site_threshold`), cette formule donne une strie longue de 0,027 unité, soit
**≈ 19 m réels** (large de 0,6 m) — d'où les « bâtonnets géants ». La formule reste globalement
proportionnelle à la distance (cohérente en angle vu depuis la caméra), donc correcte en vue
lointaine (validée par ZG7c), mais elle ignore l'échelle physique réelle : au palier site, une
goutte doit rester de l'ordre de quelques centimètres à quelques dizaines de centimètres, pas de
plusieurs mètres.

## Plan
1. Squelette : ressource `PrecipitationProfile` (`game/resources/precipitation.tres`), branchée
   dans `campaign_weather_view.gd` à la place des constantes codées en dur ; test dédié désactivé.
2. Courbe continue taille/traînée/vitesse/boîte d'émission : ancrée sur une taille réaliste (m) au
   palier site (`near_distance`), raccord lisse (log-distance) avec la formule d'origine à partir
   d'une distance charnière (`far_distance`, ancienne valeur reprise telle quelle au-delà → vue
   vallée/lointaine inchangée). Neige : même mécanique, tailles propres.
3. Captures avant/après (site, vallée, lointain) `docs/img/sz5/`, itération visuelle.
4. Test headless dédié (continuité, tailles réalistes, coût), `smoke`.

## État
- [ ] 1. Squelette
- [ ] 2. Courbe
- [ ] 3. Captures
- [ ] 4. Tests

## Prochaine étape
Écrire `precipitation_profile.gd` + `.tres`, brancher dans `campaign_weather_view.gd`.
