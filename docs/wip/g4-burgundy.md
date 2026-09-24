# Lot G4 « Bourgogne et Brabant » — état

Branche : `worktree-agent-a734b058e294e9537` (à partir de `main` ac6b7c2). Périmètre : `core/crates/ai`
(alignement), `data-model` (struct `AiAlignment`, chargée depuis `data/ai/alignment.json`), `sim-campaign`
(constantes de raisons exposées : `GIFT_REASON`, `AT_WAR_REASON`, `PERJURY_REASON`, `AGGRESSION_REASON`),
schéma `data/schemas/ai_alignment.schema.json`, test Python `tools/tests/test_ai_alignment_schema.py`.
Doc : `docs/design/m9-ai.md` § 6 ; tableau avant/après : `docs/status.md` (« Bourgogne et Brabant G4 »).

## Résultat (century_probe, 464 tours)
| Mesure | Avant 1-5 | Après 1-5 | Avant 40 gr. | Après 40 gr. |
|---|---|---|---|---|
| Bourg.-Angl. (graines) | 0/5 | 5/5 (1419-1422) | 4/40 | 23/40 (1419-1442) |
| Angl.-Brabant (tours de guerre FR-EN) | 0 % | moy. 30 % | 10 % | 39 % |
| Guerre FR-EN | moy. 53 % | moy. 54 % | 57 % | 62 % |
| Majeures en vie en 1400 | 5/5 | 4/5 (Écosse g. 3) | 40/40 | 38/40 |
| Banqueroutes / fac. / déc. | 0,44 | 0,49 | 0,47 | 0,49 |
| Auld Alliance (tours de guerre) | 81-97 % | moy. 78 % | 93 % | 83 % |

## Points
1. [x] Grief (`grievance_change`) : modificateurs négatifs personnels ≤ −40 → prétendant/ennemi du patron.
2. [x] Domination relative (15 % du royaume).
3. [x] Réglages dans `data/ai/alignment.json` + schéma + test Python (tous les seuils G2 y ont migré).
4. [x] Fiefs-rentes, toile des Pays-Bas (princes moindres voisins d'un allié moindre en guerre), guerres
   de succession seulement, pas de courtisan pour un vassal, `max_allies`.
5. [x] Rupture avec les alliés rivaux du nouveau patron avant l'offre d'alliance.
6. [x] Tests Rust `ai/tests/g4.rs` (7), mesures, docs.

## Points ouverts
- `CampaignState::are_neighbors` ne lit que `Province::neighbors` (6 provinces sur 132) : bogue global,
  correctif essayé puis écarté (appels aux armes ×5, guerre FR-EN 66-86 %, Auld Alliance cassée) ;
  l'alignement utilise `alignment::borders`.
- 2 majeures disparues avant 1400 sur 40 graines (0 avant) ; Auld Alliance et Bourg.-France en recul.

## Prochaine étape
Lot livré sur la branche ; fusion par l'orchestrateur.
