# ZG8 — relief exagéré façon Total War (visuel seulement)

État : EN COURS (branche worktree-agent-a324124257f1dc359).

## Avancement
- [x] Squelette : `ReliefExaggerationProfile` (+ `resources/relief_exaggeration.tres`), `ReliefFloor`
  (fond min+flou, WorkerThreadPool), `MapData.display_height` / `height_from_display` / `set_relief_floor`,
  `shaders/campaign_relief.gdshaderinc` (`campaign_display_height`), globaux `campaign_relief_*`.
- [x] Brancher tous les consommateurs (terrain/quadtree, E0, fleuves, routes, villes, monuments, ponts).
- [x] Roche selon la pente affichée, soleil plus rasant (34°).
- [x] Test `tests/zg8_relief_test.gd` OK ; ZG2/ZG4/ZG5b/ZG6 OK (ZG2/ZG4 adaptés à la formule) ; smoke :
  mêmes 15 échecs avec et sans ZG8 (dylib périmée vs `data/unit_types` : `required_building`, le core de
  main ne compile pas à cette heure — hors lot).
- [x] Banc `--bench-map` lancé avec/sans (`--no-relief-exaggeration`) : machine chargée (21 Godot),
  mesures non concluantes, pas de régression visible ; fond calculé en 90 ms au chargement.
- [ ] Captures définitives `docs/img/zg8/` (script `tests/zg8_relief_shots.gd`), docs (godot-map.md, ADR 0036),
  fusion de main (ZG4b).

Prochaine étape : voir la première case non cochée.

## Constat (capture Total War Warhammer III fournie par le joueur, 25/09)
- Exagération non uniforme : plaines plates, montagnes en falaises.
- Roche posée selon la pente, pas l'altitude.
- Lumière rasante, ombres longues : le relief se lit par l'ombrage.
- Exagération forte même de près. Chez nous ZG4 descend à ×1,5 près du sol, d'où une vue rapprochée plate.

## Objectif
Accentuer le relief **local** : hauteur affichée = s(d)·h + k(d)·(h − fond(x, z)).
- `fond` = fond de vallée lissé (flou ou min-flou ~5-10 km), grille basse résolution calculée une fois au chargement.
- `s(d)` : échelle ZG4 existante, avec le plancher de près relevé à ~×2,5-3 (réglable).
- `k(d)` : gain de relief local (~1-2), réglable, 0 = comportement actuel.
- Plaines inchangées, rivières au fond (h ≈ fond) donc ponts et berges stables.

## Contraintes
- Purement visuel : rien dans `core/`, aucune règle ne change (déplacement, vision, batailles en mètres réels).
- **Une seule fonction** : `campaign_display_height` dans un `.gdshaderinc` commun, et son double GDScript (`MapData` / `TerrainBuilder.surface_height_at`).
  Tous les consommateurs de `campaign_vertical_scale` passent par elle : terrain, fleuves, routes, villes, monuments, ancrages, armées, caméra (plancher), sondes de survol.
- Réglages dans `close_camera.tres` (ou une ressource voisine), avec un interrupteur pour revenir à l'état actuel.
- Roche selon la pente dans `terrain.gdshader` (triplanaire ou étirement corrigé sur falaises).
- Ombrage plus marqué (lumière plus rasante), sans casser le parchemin ni les filtres de carte.

## Recette
- Captures avant/après : Pyrénées, Alpes, Massif central, Vosges, pays de Galles, falaises normandes, coteaux de Seine et de Loire, Paris (plaine, doit rester plate).
- Tests ZG2/ZG4/ZG5b/ZG6 + smoke. Test dédié : plaine inchangée, sommet rehaussé, objet posé = hauteur affichée du sol.
- Banc `--bench-map` : coût ≤ quelques %.
