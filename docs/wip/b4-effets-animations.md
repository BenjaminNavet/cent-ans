# Lot B4 — Effets et animations de bataille

Plan : `docs/design/2026-09-24-rapprochement-total-war.md` (lot B4). Suite de B1 (`docs/wip/b1-maillages.md`).

## État : en cours
- [ ] Rythme : cause du « pas de contact en 300 s » de la démo (IA `sim-battle` ou scénario)
- [ ] Figurines (script Blender) : poids de coude, jambes du cavalier (membre propre), rênes aux mains
- [ ] Shader : archer (corde à la main droite, coude, torsion du buste, décoche), arbalétrier
  (visée / recharge), cavalier (jambes, rênes, charge lance couchée), caparaçon au galop,
  mêlée variée et désynchronisée + recul, chutes variées
- [ ] Effets (`battle_effects.gd`) : poussière, flèches/carreaux en vol et fichés, fumée et éclair
  des bombardes, éclaboussures aux gués, impact de charge
- [ ] Captures `docs/img/b4/`, smoke, banc d'essai

## Prochaine étape
Diagnostic du rythme (démo réelle vs fixture `demo_battle_1337.json`).
