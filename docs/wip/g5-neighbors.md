# Lot G5 « Voisinage réel » — état

Branche : `worktree-agent-a8e6ab2920478f32d` (à partir de `main` f52fd92).
Objet : `CampaignState::are_neighbors` sur l'adjacence de la carte (`movement::land_neighbors`, graphe de
`data/map/provinces.geojson`) au lieu des `neighbors` des fichiers de province (6/132 renseignés) ;
suppression du doublon `ai::alignment::borders` ; rééquilibrage pour garder les indicateurs G2/G4.

## Fait
- [x] `are_neighbors` → `movement::land_neighbors` ; `alignment::borders` supprimé (une seule source).
- [x] Propagation de l'hérésie (`religion.rs`) et `get_province().neighbors` du pont → même graphe.
- [x] `century_probe` : synthèse G5 (moyennes, bandes cibles, chutes des majeures avec l'année).
- [x] Réglages `data/ai/diplomacy.json` (schéma `ai_diplomacy.schema.json`, struct `data_model::AiDiplomacy`,
  défauts = constantes F4) : voisin menaçant, front, prétendants, cobelligérance (`min_ally_power_ratio`,
  `border_only_claim_wars`), paix (`keep_capital` : ni capitale ni dernière province cédée).
- [ ] Mesure finale (5 puis 40 graines), `playthrough`.
- [ ] Rééquilibrage à finir.
- [ ] Tests, docs (`docs/status.md` section G5, `docs/design/m9-ai.md`).

## Mesures (40 graines, century_probe 464 tours)
| | Guerre FR-EN | Auld | Brabant | Bourg.-Angl. | Banq. | Appels | 4 maj. 1400 |
|---|---|---|---|---|---|---|---|
| avant (main) | 66 % [41-83], 22/40 en bande | 81 % | 35 % | 22/40 | 0,47 | 170 | 37/40 |
| correctif brut | 69 % [53-88], 24/40 | 81 % | 31 % | 23/40 | 0,52 | 736 | 39/40 (Écosse g.15 1340) |
| t1 (min_ally 1.0, keep_capital) | 66 % [46-80], 30/40 | 85 % | 37 % | 17/40 | 0,51 | 527 | 38/40 (Écosse g.15, Bourgogne g.23) |

Binaires et sorties dans le scratchpad de session (`g5/`) ; `run.sh <binaire> <nom>`.

## Prochaine étape
Mesurer t2 (dernière province gardée + `border_only_claim_wars`), puis régler.
