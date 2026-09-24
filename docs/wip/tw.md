# WIP orchestrateur — rapprochement Total War (session 6)

Mandat (24/09) : autonomie complète, plusieurs heures. Rapprocher le jeu de Total War, visuellement et en mécaniques ; garder les mécaniques propres à Cent Ans.
Hors périmètre : refonte colonies (C2c-C7, autre session, `docs/wip/colonies.md`), portraits.

Choix du joueur au lancement :
- Référence : **mélange moderne** (mécaniques de Medieval II + ergonomie et visuel des TW récents : Three Kingdoms, Pharaoh, Warhammer III).
- Priorité : **équilibrée**, alterner vagues bataille / campagne.
- Autorisé : dépenses cloud dans le plafond de 50 $, assets libres CC0/CC crédités, Blender local (MCP), push auto de main après chaque vague.

## Vagues

| Vague | Lots | État |
|---|---|---|
| 0 | P1 petits points cœur (no_quarter, étain, IA Normandie ouest) ; P2 rapport de saison + smoke ; R exploration TW | **fusionné** (P2 f48b557, P1 ae4ec92) |
| 1 | B1 maillages soldats/chevaux (Blender) ; B2 bannières d'unité + vignettes + écran de fin ; C1 minicarte + brouillard | **fusionné** : B2 c0b9635, B1 ff592ae, C1 ee4f2b6 |
| 2 | B3 musique dynamique + caméra de suivi ; B4 effets + animations (suites B1) + rythme d'engagement ; C2 zone de contrôle **après C4 colonies** (refonte du déplacement) | B3 fusionné (953a758) ; B4 en cours ; C2 en attente |
| 3 | C3 arbre familial + fiche de général (arbre de compétences) ; B5 champs de bataille tirés de la campagne (biome, village, saison) | en cours |

## Décisions
- C1 : règle de vue `vision.rs` + `data/rules/vision.json` ; brouillard désactivable (Réglages > Carte).
- B3 : musique de bataille = `war.ogg` transformé (filtre, couches sfx), pas de nouvelle piste.
- Piège : le smoke Godot sort en 0 même sur erreur d'analyse ; compter les lignes « smoke OK » (21 attendues) et `--import` après un nouveau `class_name`.
- B1 : figures modélisées en Blender scripté (`tools/blender/battle_figures.py`), ADR 0006 ; `--legacy-figures` pour comparer.
- B2 : illustrations d'unités générées par une autre session (non suivies) ; les cartes s'en servent dès qu'elles existent, sinon composition de repli.
- C2 reporté : C4 colonies refond le déplacement.
- P1 : pas de quartier = chef pris tué, −5 piété au vainqueur ; étain déjà utilisé (maison de fonte), schéma inchangé ; `frontier.rs` source unique de la classification frontière.
- Plan : `docs/design/2026-09-24-rapprochement-total-war.md` (lots B1-B4, C1-C6). Thème parchemin conservé, densité TW visée.

## Prochaine étape
Fusions via worktree `../gp-tw-merge` (branche `integration/tw`) puis ff-only dans main + push. Attendre la vague 1 ; vague 2 : C2 (zone de contrôle + aire de déplacement), B3 (musique dynamique + caméra de suivi), B4 (effets).
