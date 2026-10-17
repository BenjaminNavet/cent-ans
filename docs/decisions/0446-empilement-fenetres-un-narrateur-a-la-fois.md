# ADR 0446 — Empilement des fenêtres : un narrateur à la fois, avis sous les fenêtres

## Contexte
La campagne de test du 17/10 a montré des avis et sous-titres du conseiller (étage `Toast`, 90) par-dessus les modales, les panneaux et leurs titres, et au premier écran briefing, tutoriel, conseiller, guide féodal et offre de mission affichés ensemble. F3/F4 ouvraient des panneaux sous le menu Pause ; Échap fermant l'aide ouvrait aussi la Pause.

## Décision
- Étages : HUD (0) < avis (5) < panneaux (10) < bandeaux < modales < tutoriel < infobulles. Un avis ou sous-titre ne couvre jamais une fenêtre : il attend derrière.
- Un seul narrateur à la fois : le tutoriel, le guide féodal et l'offre de mission attendent que la file de modales soit vide ; le guide féodal attend aussi la fin du tutoriel ; le conseiller se tait (ses répliques restent en file) tant qu'une modale, la Pause ou la carte du tutoriel est affichée. Ordre au premier écran : briefing, tutoriel, conseiller.
- `KeyMap::set_suspended` : pendant la Pause, toute action sauf `campaign_pause` / `battle_pause` se lit comme non pressée (aucun raccourci de panneau).
- `modal_escape` consomme l'arête Échap (`KeyEdges::escape`) quand la modale retient Échap, et `pause_escape` passe après lui : fermer l'aide n'ouvre plus la Pause.
- Les touches ne sont plus écrites dans les données : `{key:ACTION}` est remplacé par la touche réellement liée (`KeyHints::expand`).

## Conséquences
Un avis émis pendant qu'une fenêtre est ouverte n'est visible qu'après sa fermeture s'il dure encore. La règle de réserve de recherche (points accumulés jusqu'à un plafond, puis perdus) est dite telle quelle dans l'alerte.
