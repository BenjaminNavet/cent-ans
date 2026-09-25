# SM1 — smoke : « Message queue out of memory »

## État — corrigé (branche de l'agent, non fusionnée dans main)

- Symptôme : `smoke.gd` sortait en 138 après 4 fins de tour enchaînées (étape campagne),
  « Message queue out of memory » (`CanvasItem::_redraw_callback`).
- Cause : boucle de mise en page dans `DiplomacyPanel._fit_minimap` (connecté à
  `MapHolder.resized`). La carte était ajustée à `taille du support − 16 px` ; depuis UI1
  (6efd3c84, `HudStyle.panel_box` → cadre texturé, marge de contenu ≥ 10 px, soit 20 px de
  cadre), la minicarte dépasse son support de 4 px à chaque ajustement : le support grandit,
  se redimensionne, relance l'ajustement… ~11 000 cycles tri/redimensionnement au premier
  affichage, soit l'essentiel des 32 Mo de la file de messages ; 4 tours sans image = saturation.
- Correctif : la marge soustraite est le cadre réel de la minicarte (taille minimale combinée −
  taille minimale de la vue, au moins 16 px), et la taille n'est réaffectée que si elle change.
  Après correctif : ≤ 9 tris par conteneur par tour.
- « music playlist too short » : le smoke pointe `MapPaths` vers les fixtures, donc
  `AudioDirector` ne trouvait pas `audio/music.json`. `_run_assets` recharge les listes depuis
  `data/audio/music.json` (vraies données, aucune piste ajoutée).
- Smoke complet : exit 0, 28 « smoke OK », 0 « smoke FAIL ».

## Prochaine étape

Aucune ; fusion dans main par l'orchestrateur.
