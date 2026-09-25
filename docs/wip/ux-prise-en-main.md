# UX — prise en main (session du 25/09, « rendre l'interface plus intuitive »)

Demande du joueur : « essaie de rendre l'interface utilisateur plus intuitive pour le joueur ».
Point de départ : audit `docs/audit/a3-ui.md` ; U0-U5 et U7-U13 sont faits (UI2, UI3, UB1).
Restent U6 (en partie fait par C7b), U14 (étiquettes) et U15 (tutoriel), plus des défauts
de découvrabilité vus sur `docs/audit/captures/ui3/u5-apres-alerts.png` et `u7-apres-carte-1280.png`.

Constat (joueur qui découvre le jeu) :
1. Les étiquettes d'armée (« 620 », « 429 siège ») couvrent les noms de ville (Paris, Mons, Boulogne…).
2. Petits symboles de carte sans légende : points roses, écus bleus, étoiles, pastilles « E ».
3. La barre du haut : 5 boutons sans libellé ; « Chronique » paraît désactivé ; boutons
   « Politique / Relief » de la minicarte en gris système.
4. Le tutoriel : bulle qui couvre sa cible, flèche rouge, pas de sommaire ni de « Plus tard » (U15).
5. Aucune indication de « que faire maintenant » au premier tour.

Fichiers évités : ceux de B1 (`codex_bubbles.gd`, `rich_tooltip.gd`, `army_strip.gd`,
`battle_hud.gd`, `smoke.gd` : ajouts seulement en fin de fichier) ; interface de bataille (session « épique »).

| Lot | Contenu | État | Branche |
|---|---|---|---|
| UX1 Carte lisible | collision étiquettes armée/ville, légende de la carte (bouton + panneau), boutons de minicarte au thème | lancé (worktree agent) | |
| UX2 Premiers pas | libellés de la barre du haut, style « Chronique », tutoriel U15, conseil « prochaine action » au premier tour | **fusionné** aaabb61c , cartouches corrigés 59113eb2 | worktree-agent-a109e2bd8dafc923f |

Fusion : ff-only dans main après `git merge main` par chaque agent ; commits avec chemins explicites.
Machine très chargée le 25/09 à 6 h 40 (charge 141) : 2 agents seulement.

## Journal
- 2026-09-25 06:45 : état des lieux, lots UX1 et UX2 lancés.
- 2026-09-25 : UX2 fusionné (ff aaabb61c) : barre libellée avec repli, Chronique au style normal, tutoriel U15 (placement auto, surbrillance dorée, sommaire, « Plus tard »), conseil « que faire maintenant » (`data/ui/next_hints.json`, réglage `interface/next_hint`). Suite demandée : cartouches de touche qui mordent sur les libellés. Question ouverte : la touche L ouvre l'encyclopédie, la reprise du guide passe par le conseil, F1 ou le menu.
- 2026-09-25 : suite UX2 fusionnée (59113eb2) : marge des cartouches de touche, moins de libellés simultanés à 1280.
