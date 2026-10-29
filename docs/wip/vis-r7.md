# VIS-R7 — végétation de plaine

Spec : `docs/audits/2026-10-29-visuel-campagne.md` table (d) R7, constat G2-7. ADR : `docs/decisions/XXXX-vis-r7-vegetation-de-plaine.md`.

## État
- Squelette (note + ADR en brouillon).

## Plan
1. Mesure de base (camp_0600, camp_guyenne, automne) + nb d'arbres isolés (dump `open_forest`).
2. Arbres isolés (rôle Open) et bosquets retirés au-delà de d 150 (`open.*`, `groves.far_share` de `data/art/campaign_forest_look.json`).
3. Masses forestières : plancher de `forest_masses` relevé pour la teinte seulement (nouveau champ `value_lift`).
4. Haies du bocage en lignes sombres ; saisons des arbres par classe.

## Points ouverts
- terrain.wesl touché uniquement dans `dark_masses` (forêt) : à signaler à VIS-R1.
