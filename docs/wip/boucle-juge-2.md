# Boucle juge → corrections, 2e campagne (29/10)

Mandat 29/10 : nouveau départ (juges neufs, sans les rapports de la 1re boucle `docs/wip/boucle-juge.md`),
itérations illimitées, arrêt par le joueur ou quand le juge ne trouve plus rien de majeur.
Refontes et révisions d'ADR permises si un juge les justifie. Rien de poussé.
Budget : tout le Meshy restant (145 crédits au 29/10), 2D via Z-Image. Cible 60 fps en Ultra.

Axes : jouabilité, gameplay, visuels, assets, couleur, textures, fps, cohérence, véracité historique et géographique ;
UI/UX, prise en main, équilibrage, accessibilité ; stabilité, chargement/mémoire, aberrations visuelles ;
audio (logs/fichiers), textes/localisation, échelle, cohérence artistique, données ;
authenticité matérielle, plausibilité militaire et politique.

Organisation : 4 juges Sonnet libres par itération (DA, game design, histoire/UX/textes, technique),
rapports `out/juge2/iter-NN-<angle>.md` ; triage par la session principale ; lots de correction en worktrees,
chaque lot vérifie d'abord le constat ; fusion ff-only avec contrôle complet (clippy, tests, smoke), seed bots en fin d'itération.
Ne pas modifier main pendant que les juges tournent (run_bg.sh recompile depuis les sources).

## Itérations
| # | Majeurs | Corrigés | fps Ultra (carte / bataille) | Commit |
|---|---------|----------|------------------------------|--------|

## Meshy
| Date | Asset | Crédits | Reste |
|------|-------|---------|-------|

## État / prochaine étape
29/10 : intégration des modifs non commitées de main (sélection des armées plus tolérante), puis itération 1.
