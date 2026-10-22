# Synthèse des 12 critiques (2026-10-22)

Rapports : un fichier par rôle dans ce dossier. Priorisation Claude ; modèles : haiku par défaut, sonnet pour les lots difficiles.

## Vague A (lots indépendants, worktrees)
| Lot | Contenu | Source | Modèle |
|---|---|---|---|
| A1 sauvegardes | écriture atomique, vérif `version`, fixture v9, signal d'échec d'autosave | qa, outils | sonnet |
| A2 audio | anachronismes `music.json`, écran de crédits CC BY | audio | haiku |
| A3 docs | manuel §15 vs keymap, status/roadmap, animation.md, ADR en double | ui-ux, production, anim, outils | haiku |
| A4 UI | légendes barre du haut, guide tutoriel, tailles de texte, alertes | ui-ux | sonnet |
| A5 narration | événements 1370-1440, Angleterre/Bourgogne, filtres, Peste noire | narration | sonnet |
| A6 hygiène | .gitignore résidus Godot, CI (fmt/clippy/pytest/schémas) | outils | haiku |

## Vague B (après A)
IA : ordre de jeu tournant, rejets d'ordres tracés, constantes IA vers `data/` ; mêlée orientée + coups synchronisés ; impacts VFX ; Guyenne en 3 provinces + terrains de bataille ; économie/révoltes (jauge de tension, armée rebelle, ADR) avec garde-fou part de guerre FR-EN ; perf (ombres impostors).

## Hors périmètre cette nuit (décision du joueur)
Push des 994 commits, plafond budget (117 $ vs 50 $), 1,7 Go `models/dn` non suivi, tri des 32 branches.
