# bm-showfix : pourquoi les captures de référence bougent peu

## État (16/10)
- Constat : le « 2/255 » de la REPRISE comparait les captures de 13 h 30 à celles de 15 h 10 ; la plupart des
  lots visuels étaient déjà fusionnés avant 13 h 30. De 08 h 24 (réf.) à 15 h 10, l'écart Bevy est de 40 à 74/255
  par vue : les lots ont bien atteint les scènes. Restent des défauts réels, traités ici un par un.
- Chargements : bataille 300 vs 900 images = 1,1/255 ; campagne 600 vs 1500 = 6/255 (animation) ; dumps identiques
  (végétation prête, colormap, paquets) → l'attente de 600/300 images suffit.
- Rivières invisibles à d=60 : PROUVÉ. Les rubans sont cuits au premier `RebuildWater` (terrain prêt, pages fines
  pas encore résidentes) sur la carte de hauteurs mondiale ; le terrain proche affiche ensuite les pages fines.
  Sonde `CENT_ANS_SEAT_PROBE=1651,2733` (Paris, 81×81 pts) : page − carte = −51 … +54 m. Avec
  `CENT_ANS_RIVER_LIFT_DEBUG=1.0` la Seine réapparaît → elle est enterrée sous le relief.
- Toits des tours de siège plats : cône de hauteur fixe 5 m, rayon ×1,25, tuile ; Godot : hauteur r×1,55, ardoise
  sombre, une tour sur deux (autres crénelées) — défaut de portage dans `battle/siege/works.rs`.

## Prochaine étape
- Corriger le calage des rivières (hauteur au shader depuis la même source que le terrain, ou recuisson quand les
  pages fines arrivent) ; toits coniques ; arbres pâles ; recaptures + écarts par vue.
