# Lot G5 « Voisinage réel » — état

Branche : `worktree-agent-a8e6ab2920478f32d` (à partir de `main` f52fd92).
Objet : `CampaignState::are_neighbors` sur l'adjacence de la carte (`movement::land_neighbors`, graphe de
`data/map/provinces.geojson`) au lieu des `neighbors` des fichiers de province (6/132 renseignés) ;
suppression du doublon `ai::alignment::borders` ; rééquilibrage pour garder les indicateurs G2/G4.

## Fait
- [x] `are_neighbors` → `movement::land_neighbors` ; `alignment::borders` supprimé (une seule source).
- [x] Propagation de l'hérésie (`religion.rs`) et `get_province().neighbors` du pont → même graphe
  (`core/build.sh` + smoke Godot OK).
- [x] `century_probe` : synthèse G5 (moyennes, bandes cibles, chutes des majeures avec l'année).
- [x] Réglages `data/ai/diplomacy.json` (schéma, struct `AiDiplomacy`, défauts = F4, test Python).
- [x] Règles : cobelligérance (`min_ally_power_ratio`, `border_only_claim_wars`), paix (`keep_capital`,
  `cornered_provinces`, solde de guerre si la dernière terre est gardée).
- [x] Tests `sim-campaign/tests/g5_neighbors.rs` (5) ; fmt, clippy, cargo test verts.
- [x] Mesures 40 et 5 graines, `playthrough` 1-5 ; docs (`docs/status.md` G5, `m9-ai.md` § 7).

## Mesures (40 graines, century_probe 464 tours)
| | Guerre FR-EN | Auld | Brabant | Bourg.-Angl. XVe | Banq. | Appels | 4 maj. 1400 |
|---|---|---|---|---|---|---|---|
| avant (main f52fd92) | 66 % [41-83], 22/40 en bande | 81 % | 35 % | 22/40 | 0,47 | 170 | 37/40 |
| correctif brut | 69 % [53-88], 24/40 | 81 % | 31 % | 20/40 | 0,52 | 736 | 39/40 |
| t1 (min_ally 1, keep_capital) | 66 % [46-80], 30/40 | 85 % | 37 % | 17/40 | 0,51 | 527 | 38/40 |
| t2 (+ border_only_claim_wars) | 66 % [50-83], 29/40 | 86 % | 26 % | 23/40 | 0,52 | 308 | 39/40 |
| t3 (+ solde de guerre) | 68 % [49-82], 31/40 | 81 % | 24 % | 30/40 | 0,47 | 337 | 38/40 |
| **final** (+ cornered_provinces 1) | 65 % [42-85], 29/40 | 90 % | 25 % | 21/40 | 0,43 | 350 | **40/40** |

## Prochaine étape
Lot livré sur la branche ; fusion par l'orchestrateur.
