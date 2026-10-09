# 0254 — Lissage lointain du sol de bataille et cadrage de gros plan hors bâtiments

Statut : accepté

## Contexte
La revue d'experts RX (`docs/wip/rx/bataille-v2.md`) relève, à moyenne distance, un motif de « feuilles » répétitif
sur l'herbe (contraste supérieur à celui des soldats) et, sous la neige, un sol en « lattes ». Les couches du sol
(`battle_ground.gdshader`) sont des tableaux de textures PBR à pas de 2 à 8 m : à 100-250 m le tuilage reste lisible
car seul le mip hardware les atténue. Par ailleurs `--closeup` cadre le foyer de mêlée à 26 m, souvent dans l'emprise
d'un hameau (caméra dans un toit).

## Décision
- Le shader du sol mélange, entre `far_flatten_start` (45 m) et `far_flatten_end` (260 m), l'albédo de chaque couche
  vers sa moyenne (mip 9) et sa normale vers le plan, de `far_flatten` (0,7) au plus. Aucune donnée ni règle : réglage
  de rendu, uniformes du shader.
- `BattleDecor` garde les emprises des bâtiments posés ; le cadrage `--closeup` cherche le lacet le plus proche de
  celui du foyer dont la caméra et la ligne de visée ne traversent aucune emprise.
- Le déploiement cadre la zone du joueur (`DeploymentController.frame_zone`) et affiche ses refus au-dessus des cartes.

## Conséquences
- Le sol perd du détail de texture au loin (assumé : c'est le bruit macro et les taches qui portent la lecture).
- Une capture `--closeup` peut changer de lacet par rapport à avant : voulu.
- Les refontes lourdes (UV des toits ardoise étirés, atlas régional sur les maisons de siège) restent à planifier.
