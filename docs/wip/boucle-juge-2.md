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

## Itération 1 — triage en cours
- « Départ sur Berlin » (DA + histux) : cause = `out/camera.txt` oublié le 28/10 (13.405,52.52), renommé en `out/camera.txt.berlin-2810.bak`. À corriger : ce fichier ne doit agir qu'en build `dev` (le joueur lance depuis le dépôt).
- Coupure internet pendant l'itération 1 : juges design et technique relancés par message.

### Lots de l'itération 1 (lancés 29/10, worktrees)
| Lot | Modèle | Constats | Rapports source |
|-----|--------|----------|-----------------|
| COMBAT | sonnet | arcs longs intouchables, cavalerie écrasante, approche de bataille 123 s | design |
| ECO | sonnet | économie trop riche, provinces qui fondent, banqueroutes, score de règne, attrition hors territoire, bandes seed bots, invariant army_refs | design |
| TEXTES | sonnet | camera.txt hors dev, HUD abrégé, Désordre/Mécontentement, « colonie », Codex futur, Jean le Bel, contrastes, typo, objectifs/prestige/domaine affichés, réserve de recrutement visible | histux, design |
| SOL | sonnet (captures ≤ 5) | damier rose du sol (Angleterre), forêt saturée | art |
| MEM | sonnet | mémoire campagne 7-12 Go, perf_guard.sh zsh, mêlée 59 fps, pics de descente | tech |
| BATDA | sonnet | héraldique cavalerie, bannières minuscules, terrain de bataille plat/horizon, mesh sans normales | art |
| DATA | haiku | valley_courses schéma, ids names_*, couverture des schémas, war_horn, filtre bus lointain | tech |
Reportés à l'itération 2 : tutoriel (recrutement/siège/bataille), illustration d'Abu al-Hasan (Z-Image), Hainaut 1337, date de départ.
ADR : les lots nomment `docs/decisions/XXXX-<lot>-titre.md`, numérotés à la fusion (dernier : 0790).
