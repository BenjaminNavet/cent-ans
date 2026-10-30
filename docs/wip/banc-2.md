# BANC-2 — auto-résolution recalibrée et datée

Etat : émetteur + test de hachage faits ; reste la retouche des coefficients. Spec : docs/audits/2026-10-29-bataille.md (BANC-2, C3), ADR 0800.

## Plan
1. `battle_bench --emit-auto-fixture` : rejoue les scénarios de la fixture (définitions conservées) + les 6 armées du banc,
   IA contre IA, 11 graines (impair : pas d'égalité), écrit la fixture avec `data_hash`.
2. `auto_resolve_calibration.rs` vérifie `data_hash` (FNV-1a 64 sur `data/unit_types/*.json` + `data/rules/battle_*.json`,
   fichiers triés par nom).
3. Mesure de l'accord AVANT toute retouche d'`auto_resolve.json`, puis retouche des coefficients si < 80 %.

## Mesures
AVANT retouche d'`auto_resolve.json` (fixture régénérée le 30/10, 26 scénarios = 20 historiques de la fixture + 6 armées du banc,
11 graines 3D, 60 tirages auto) : **accord 73,1 % (19/26)**. Désaccords : Crécy colline (auto 27/60, 3D 8/11), 10 chevaliers
contre 10 arcs (0/60, 3D 6/11), 4 chevaliers contre 20 milices x2 (auto 43/60, 3D 2/11), 10 piquiers contre 8 hommes d'armes
(0/60, 3D 6/11), FR/EN collines (0/60, 3D 6/11), symétrique 7 (25/60, 3D 6/11).

## Prochaine étape
Implémenter l'émetteur.

APRES retouche (données seulement, `data/rules/auto_resolve.json`) : `melee_lethality` 0,12 -> 0,20, `armor_vs_melee` 0,9 -> 0,6.
**Accord 80,8 % (21/26)**. Corrigés : 4 chevaliers contre 20 milices (x2 : auto 43/60 -> victoire milices) et 10 piquiers
contre 8 hommes d'armes. Plateau stable (0,55/0,22 et 0,6/0,25 donnent le même chiffre). Balayage des autres coefficients
(`cavalry_melee_factor`, `charge_lethality`, `armor_vs_charge`) : sans effet sur les désaccords.
Désaccords restants (5) : 10 chevaliers contre 10 arcs (3D 6/11, pile ou face), FR/EN collines (6/11), symétrique 7 (6/11),
Crécy colline (3D 8/11 pour l'attaquant français, contraire à l'histoire : à juger côté 3D, pas côté auto).
Seuil du test abaissé de 95 % à 80 % (cible de l'audit).
