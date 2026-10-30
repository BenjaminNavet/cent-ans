# ADR 0807 — BANC-2 : auto-résolution recalibrée et datée

## Contexte

La fixture `auto_resolve_scenarios.json` (ADR 0013) était périmée (ADR 0792) et son outil `balance_probe` avait disparu : le
test d'accord auto-résolution / bataille 3D passait contre des références fausses.

## Décision

- `battle_bench --emit-auto-fixture` régénère la fixture : les scénarios déjà présents (définitions conservées) plus les
  six armées de la matrice du banc, IA contre IA, 11 graines (impair, pas d'égalité).
- La fixture porte `data_hash` : FNV-1a 64 de `data/unit_types/*.json` et `data/rules/battle_*.json` (chemins relatifs triés,
  nom, 0x00, longueur, octets). Le test `auto_resolve_calibration` recalcule le hachage et échoue, avec la commande à lancer,
  si les données ont changé. `auto_resolve.json` n'y figure pas : ses retouches ne périment pas les références 3D.
- Seuil d'accord : 80 % (95 % n'était tenable que sur des références figées). Mesure avant : 73,1 %, après : 80,8 %
  (`melee_lethality` 0,12 -> 0,20 ; `armor_vs_melee` 0,9 -> 0,6), détail dans `docs/wip/banc-2.md`.

## Conséquences

- Toute retouche d'unité ou de règle de bataille impose de relancer l'émetteur, puis de juger l'accord.
- Les scénarios à 3D proche de 50 % (6/11) sont du bruit : ils ne doivent pas guider les coefficients.
