# Relecture historique — Londres vers 1340 (VH6)

Branche : `worktree-agent-a50490ef8dc8e2554`. Rapport : `docs/histoire/relecture-vh-londres.md`.

## État
- [x] Lecture des données (`data/landmarks_v2/london.json`, `docs/wip/vh6-londres.md`)
- [x] Recherche des sources (NHLE/Historic England via ArcGIS ouvert, VCH, Survey of London,
      Gerhold, MOLA, Britannica 1911, OSM)
- [x] Corrections du JSON (mur, portes, pont, St Paul's, couvents, hôtels du Strand, rues exclues)
- [x] Rapport
- [x] Tests : pytest `test_landmarks_v2.py` (32 OK), `vh4_landmarks_test.gd` OK (7 042 maisons)

## Prochaine étape
Fusion par l'orchestrateur. Points ouverts listés à la fin du rapport (tablier et avant-becs du
pont, angle exact d'Old St Paul's, Moorgate avant 1415, rues modernes sur Old St Paul's).
