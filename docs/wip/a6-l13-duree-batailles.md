# A6-L13 : durée des batailles

## État
- Sonde `sim-battle/tests/l13_duration.rs` (ignorée) : durée et vainqueur, 20 scénarios de la fixture + campagne 600/550 + Crécy/Azincourt/Poitiers, 6 graines.
- Cadence en données : `data/rules/battle_pace.json` (`PaceRules`), `BattleSim::set_pace` pour le réglage.
- Avant : médiane des médianes 302 s (5 min) ; historiques 14-15 min.

## Prochaine étape
- Régler melee_rate / ranged_rate / loss_morale_factor, comparer les vainqueurs, munitions, ADR 0180.
