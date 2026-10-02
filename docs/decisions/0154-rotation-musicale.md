# 0154 — Rotation musicale mélangée, persistante, et liste de guerre élargie

Date : 2026-10-02. Complète l'ADR 0060 (musique d'époque).

## Contexte

Retour du joueur : la campagne « commence toujours avec la même musique », et ce premier morceau
déplaît. Constat :

- La France et l'Angleterre démarrent en guerre : la carte ouvre donc sur le contexte `war`, dont
  la liste `primary` ne comptait que 3 morceaux — et la guerre dure presque toute la partie.
- Le tirage était aléatoire mais sans mémoire : seul le morceau précédent du même contexte était
  exclu, et rien ne survivait à la fermeture du jeu. Sur 3 morceaux, la même ouverture revient
  une fois sur trois ; l'estampie « Retrove » figurait en plus dans cinq listes (menu compris).
- Les trois pièces synthétiques d'origine (`assets/audio/music/{campaign,war,court}.ogg`, numpy)
  étaient restées en `primary`, contre la bible DA § 9 (pas de synthé) déjà appliquée aux MIDI.

## Décision

- **Rotation mélangée par contexte** (`AudioDirector.next_track`) : la liste est battue, chaque
  morceau passe une fois, on ne rebat qu'une fois la liste épuisée. Le dernier morceau tiré n'est
  jamais rejoué aussitôt, y compris en changeant de contexte.
- **Rotation persistante** dans `user://music_rotation.cfg`, écrite à chaque tirage : une nouvelle
  session reprend la rotation là où la précédente s'est arrêtée, donc n'ouvre pas sur le même
  morceau. Pas de persistance en mode sans affichage (tests).
- **En guerre, `war` est complétée par la liste de campagne de la région** de la faction jouée
  (`campaign_<région>`, même niveau `primary`/`fallback`). L'ADR 0060 avait écarté des listes
  `war_<culture>` ; le mélange donne la variété sans nouvelle donnée à maintenir.
- **Pièces synthétiques rétrogradées en `fallback`** dans `data/audio/music.json` (jamais
  supprimées, comme Kevin MacLeod) : elles ne jouent plus que si aucune piste d'époque n'est là.

## Conséquences

- Guerre, France : 8 morceaux en rotation au lieu de 3 ; aucune ouverture identique deux
  lancements de suite.
- La musique de guerre est moins martiale (airs de cour et de campagne mêlés) ; à réévaluer si
  de vraies pièces guerrières libres de droits sont ajoutées.
- La bataille (`BattleMusicDirector`) tire toujours sa piste de base dans `playlist("war")`
  (`primary` + `fallback`) : inchangée.
- Vérifié par `game/tests/smoke.gd` (rotation complète, reprise après rechargement, mélange).
