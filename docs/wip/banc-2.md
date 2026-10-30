# BANC-2 — auto-résolution recalibrée et datée

Etat : squelette. Spec : docs/audits/2026-10-29-bataille.md (BANC-2, C3), ADR 0800.

## Plan
1. `battle_bench --emit-auto-fixture` : rejoue les scénarios de la fixture (définitions conservées) + les 6 armées du banc,
   IA contre IA, 11 graines (impair : pas d'égalité), écrit la fixture avec `data_hash`.
2. `auto_resolve_calibration.rs` vérifie `data_hash` (FNV-1a 64 sur `data/unit_types/*.json` + `data/rules/battle_*.json`,
   fichiers triés par nom).
3. Mesure de l'accord AVANT toute retouche d'`auto_resolve.json`, puis retouche des coefficients si < 80 %.

## Mesures
(à remplir)

## Prochaine étape
Implémenter l'émetteur.
