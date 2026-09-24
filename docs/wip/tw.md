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
| 1 | B1 maillages soldats/chevaux (Blender) ; B2 bannières d'unité + vignettes + écran de fin ; C1 minicarte + brouillard | B1, B2, C1 en cours |

## Décisions
- P1 : pas de quartier = chef pris tué, −5 piété au vainqueur ; étain déjà utilisé (maison de fonte), schéma inchangé ; `frontier.rs` source unique de la classification frontière.
- Plan : `docs/design/2026-09-24-rapprochement-total-war.md` (lots B1-B4, C1-C6). Thème parchemin conservé, densité TW visée.

## Prochaine étape
Fusions via worktree `../gp-tw-merge` (branche `integration/tw`) puis ff-only dans main + push. Attendre la vague 1 ; vague 2 : C2 (zone de contrôle + aire de déplacement), B3 (musique dynamique + caméra de suivi), B4 (effets).
