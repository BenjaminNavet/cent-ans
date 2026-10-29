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
| 1 | Berlin (fichier oublié), arcs longs, cavalerie, économie, provinces qui fondent, damier rose, forêt, mémoire 7-12 Go, mêlée 59 fps | 7 lots fusionnés (DATA, BATDA, SOL, TEXTES, COMBAT 0792, ECO 0793, MEM 0794) + illustration mérinide | mêlée 59 (inchangé), mémoire bataille 6,5 → 3,6 Go, campagne repos 6,2 → 5,4 Go | 59c0047de |

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

### Restes pour l'itération 2
- IA de bataille : charge de cavalerie groupée (les chevaliers ne battent pas les arcs longs, ADR 0792), approche encore longue, fixture auto_resolve à régénérer, chevaliers contre hommes d'armes en marche (4 contre 231).
- fps mêlée 59 < 60 : voile volumétrique `step_count` 64 → 40-48, préchauffage des pipelines pendant le voile de chargement (pics de 350-1100 ms).
- Mémoire campagne : 3 Go non comptés par Metal (ADR 0794).
- Prestige de faction non lu (le brancher change l'IA) ; réserve de recrutement à afficher (`game_api::hud::recruit_reserve`) ; seuil de victoire 50 à valider.
- Tutoriel : recrutement, siège, bataille, ravitaillement.
- Bataille : plaques de régiments ennemis en grappe sur l'horizon ; barre du haut tronquée à 1280 px.
- « Edimbourg » cuit dans menu_map.jpg ; Hainaut 1337 ; date de départ.

## Itération 2 (29/10 soir) — virage vers les refontes
Demande du joueur : changements plus importants, analyses plus profondes, parties du jeu révisables.
En cours :
- 4 audits de fond Opus (lecture seule) → `docs/audits/2026-10-29-{campagne,bataille,economie-politique,experience}.md` : diagnostic, 3-6 refontes, specs de lots ;
- juges histoire/UX et technique (iter-02) ; juges DA et design arrêtés (couverts par les audits) ;
- lots IA-CAV, FPS, TUTO (worktrees) ; MEM5 Opus (3 Go de mémoire de campagne).
Suite : choisir les refontes prioritaires dans les audits et les lancer en lots larges.

### Décisions du joueur (29/10)
- Priorité : le visuel, surtout la carte de campagne → audit dédié `docs/audits/2026-10-29-visuel-campagne.md`.
- R6 acceptée : recentrer sur 6-8 maisons jouables approfondies ; les autres factions restent dans le monde, pilotées par l'IA.
- Audit campagne rendu (02292a92f) ; refonte R1 (renommée, actes à dates fixes, ambitions) lancée.
- Audit expérience rendu (df3e688da) : C1 neuf canaux d'attention, C2 guide « boutons », C3 139 factions jouables, C4 bâtir = 130 clics ; refontes XP R1-R6, ordre R4+R1 (XP-POURQUOI, XP-AUDIENCE) puis XP-LISIBLE. À lancer après fusion de TUTO et R1 (fichiers communs) ; machine saturée (attente heavy.sh ≈ 1 h), pas plus de lots d'ici là.
- Audit économie/politique rendu (741e6022a) : la guerre ne coûte presque rien (armée 12-16 % des dépenses), l'ost ne donne pas de soldats, leviers sans interlocuteur, légitimité absente ; 4 défauts de diplomatie. Ordre R0 → R1 (impôt consenti) → R2 (ost et retenue) → R5 → R4 → R3. Lot ECO-R0 (Sonnet) lancé.
