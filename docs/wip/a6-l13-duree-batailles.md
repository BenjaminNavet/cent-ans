# A6-L13 : durée des batailles

## État
- Sonde `sim-battle/tests/l13_duration.rs` et `l13_trace.rs` (ignorées) ; surcharges par variables d'environnement.
- Cadence du combat en données : `data/rules/battle_pace.json` (`PaceRules`), valeurs inchangées.
- Mesures et analyse : `docs/decisions/0180-duree-des-batailles.md`. Cible ×2-×3 non atteignable par les seules
  données de combat/moral sans casser les vainqueurs (plafond ×1,3-1,4) ; décision de conception à prendre.

## Prochaine étape
- Arbitrer parmi les trois pistes de l'ADR (marche d'approche, effectifs, nouvel équilibre historique).
