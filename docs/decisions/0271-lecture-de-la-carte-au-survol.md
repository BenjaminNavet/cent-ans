# 0271 — Lecture de la carte au survol (lot WH hover)

## Contexte
Dans Total War: Warhammer III on lit la carte d'un coup d'œil : hostilité, posture et état des armées, bulle riche au survol, tours du chemin, zones de contrôle, dernières positions vues. Le jeu n'affichait qu'une ligne de survol et un seul cercle de zone de contrôle. Source : `docs/wip/wh/carte.md` § 3 points 2, 3, 5, 6, 7, 10. Aucune règle de jeu : tout est présentation.

## Décision
- **Jetons du parchemin** : anneau d'hostilité de `StanceCues.army_ring`, glyphe ennemi, pastille de posture (`StanceBadge`), le signe » en marche ; `ParchmentOverlay.token_cues` est pur. L'embuscade ennemie n'est jamais montrée.
- **Bulles riches** (`MapHoverText`, `ObjectHover` qui étend `DecorHover` : mêmes délai et fermeture, réglage `interface/decor_hover_delay`) pour armée, colonie et fantôme. Une colonie hors de vue ne livre que son nom et son propriétaire ; vivres seulement pour les armées du joueur. La ligne de survol immédiate (`army_hover_text`) reste en repli.
- **Numéros de tour** : `Label3D` poolés sur les jalons du chemin (« 1 », « 2 », « Tn » au dernier).
- **ZOC ennemies** : un `Decal` par armée ennemie dont le marqueur est montré (donc filtré par le brouillard), ≤ `zoc.max_rings`, pendant qu'une armée du joueur est sélectionnée ; réglage `map/show_zoc`.
- **Mémoire** (`ArmyMemory`) : dernière situation des armées ennemies vues, fantôme grisé (3D, parchemin, losange creux sur la minicarte) pendant `fog_memory.turns` saisons ; éphémère, hors sauvegarde, ne lit jamais la simulation. Données dans `data/map/stance_cues.json` (blocs `fog_memory`, `zoc`).

## Conséquences
- Un fantôme ne disparaît qu'à l'expiration ou à la réapparition de l'armée (pas de test « la place est vide » : il faudrait la vision par cellule). Une armée détruite hors de vue reste donc affichée jusqu'à expiration.
- Le disque de fond à 12 % de la ZOC n'est pas fait : l'anneau seul (texture d'anneau existante).
- Le délai de la bulle est celui du décor naturel ; le mettre à 0 désactive aussi ces bulles.
