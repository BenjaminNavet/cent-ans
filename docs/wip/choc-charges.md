# Choc des charges — lots 1-4 (ADR 0711)

Branche `feat/choc`, worktree `../gp-choc`. Mandat du joueur (20/10) : enchaîner les 4 lots.

## Lots
1. [fait] Constantes du choc → `data/rules/battle_impact.json` (`ImpactRules`), sans changer les résultats.
2. [code écrit] Élan réel au contact (mètres courus avant l'impact) × rapport de poids chargeur / cible.
3. [code écrit] Arc-boutement graduel : à l'arrêt, face à la charge, profondeur (rangs) ; schiltron = complet.
4. [code écrit] Paliers (reculé / renversé selon l'intensité), immunité courte après un choc, bonus de charge décroissant.
Puis : recalage (Crécy/Poitiers, duel symétrique, seed bots rapides), ADR 0711, fusion dans main.

## État
- Lot 1 : fait, 509 tests sim-battle verts.

## Constat lot 2
Un régiment au contact relançait un impact de charge à chaque retour en mêlée (≈ 7 000 impacts
de cavalerie d'un camp sur 10 duels symétriques) : bonus de charge quasi permanent. Désormais
`elan.min_impact_run_up_m` (5 m courus) est exigé : plus d'impact sans élan.

## Prochaine étape
Recalage : 9 tests d'équilibre rouges (duel symétrique, IA contre passive, fins de bataille,
b6 empreinte, bv2). Leviers : `battle_pace.melee_rate`, `charge_bonus_s`, cohésion, renversés.
