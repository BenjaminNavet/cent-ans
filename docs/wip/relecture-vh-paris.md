# Relecture historique de Paris vers 1340 (VH5)

Branche : worktree d'agent `worktree-agent-a8c2568b3bd7b7929` (depuis `main`). Rapport :
`docs/histoire/relecture-vh-paris.md`.

## Méthode
- Corrections dans `tools/geo/paris_v2_author.py`, puis
  `uv run --project tools python tools/geo/paris_v2_author.py` et
  `uv run --project tools cent-ans geo landmarks --city paris` (reproductible : diff nul avant
  corrections).
- Liens symboliques non versionnés `data/map/pyramid`, `tools/geo/raw` ; dylibs copiées dans
  `game/bin/`.

## État : terminé (à fusionner par l'orchestrateur)
- [x] Recherches (ponts, Notre-Dame, enceintes, portes, Louvre, Temple, Saint-Victor, Bernardins)
- [x] Corrections script + JSON (régénéré), `test_alpage.py` mis à jour (noms de portes de 1340,
      Filles-Dieu 1360, Saint-Eustache)
- [x] Rapport `docs/histoire/relecture-vh-paris.md`
- [x] pytest `test_landmarks_v2` + `test_alpage` (36 OK) ; Godot `vh4_landmarks_test` OK
      (Paris : 109 monuments en 1340 au lieu de 110, Filles-Dieu datées de 1360)

## Points ouverts
Voir la fin du rapport (Bernardins en chantier, porte Saint-Antoine démolie en 1382, premier site
des Filles-Dieu, largeur du pont aux Changeurs, Petit Châtelet 1296-1369).
