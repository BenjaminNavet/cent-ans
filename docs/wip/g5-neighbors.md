# Lot G5 « Voisinage réel » — état

Branche : `worktree-agent-a8e6ab2920478f32d` (à partir de `main` f52fd92).
Objet : `CampaignState::are_neighbors` sur l'adjacence de la carte (`movement::land_neighbors`, graphe de
`data/map/provinces.geojson`) au lieu des `neighbors` des fichiers de province (6/132 renseignés) ;
suppression du doublon `ai::alignment::borders` ; rééquilibrage pour garder les indicateurs G2/G4.

## Fait
- [x] `are_neighbors` → `movement::land_neighbors` ; `alignment::borders` supprimé (une seule source).
- [x] Propagation de l'hérésie (`religion.rs`) et `get_province().neighbors` du pont → même graphe.
- [x] `century_probe` : synthèse G5 (moyennes, bandes cibles, chutes des majeures avec l'année).
- [ ] Mesure avant/après (5 puis 40 graines), `playthrough`.
- [ ] Rééquilibrage (réglages dans `data/`).
- [ ] Tests, docs (`docs/status.md` section G5, `docs/design/m9-ai.md`).

## Mesures
Avant (main f52fd92, 40 graines) : guerre FR-EN moy. ≈ 66 %, survie en 1400 : Écosse 37/40 (graines 26, 27, 35).

## Prochaine étape
Mesurer l'effet brut du correctif (40 graines), puis régler.
