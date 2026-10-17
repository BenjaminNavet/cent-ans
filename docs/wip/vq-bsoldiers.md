# VQ bsoldiers — soldats de bataille réalistes (17/10)

## État
Squelette. Lot : matières (acier/mailles, étoffes moins saturées, peau), distance de bascule imposteur (`battle_lod`), variation par soldat.

## Plan
1. `data/fx/battle_soldier_look.json` (nouveau) : désaturation des livrées, lueur lointaine, variation, teinte peau ; lu par `SkinParams` (créneaux libres de `hitn.zw`, `fine.w`).
2. `battle_lod` du préréglage -> `RenderQuality.battle_lod` -> bord imposteur de `Lod::for_distance`.
3. Mesures A/B mêlée (`tools/bevy_bench/ab.sh battle`).

## Décisions (pas d'ADR numéroté disponible)
- (à compléter)

## Mesures / Reste
- (à compléter)
