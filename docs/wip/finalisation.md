# WIP orchestrateur — finalisation (session 4)

Plan : `docs/design/v2-finalisation.md`.

| Lot | État | Branche / agent | Notes |
|---|---|---|---|
| F1 Règles inertes | **fusionné** | | reste inerte : recruit_slots, Piety bâtiments/traits, army_armor/army_ranged ; alliés absents des assauts ; pas d’ordre de rançon joueur |
| F2 Icônes et infobulles | **fusionné** (b1405fe) | | 144 ids → 106 SVG, RichTooltip, CREDITS.md ; reste : accesseurs GameDataStore pour retirer GameCatalog |
| F3 Écrans et flux | **fusionné** | | menu illustré, chargement, pause, réglages, emplacements + auto, rapport de saison, alertes, crédits |
| F4 Guerre vivante | **fusionné** (374dfa4) | | FR-EN en guerre 73 % du siècle, 7 phases ; hors cible : trésors > 8 saisons par pointes, banqueroutes 0,92 (Écosse) ; Flandre/Bourgogne jamais pro-anglaises |
| F5 Batailles | **fusionné** (F5a-F5d, 88a8524) | | démo : contact à 75 s ; escalade 3/6 ; renforts au-delà de 20 régiments ; messages de déploiement nommés |
| F6 Rendu carte | **transféré** | session parallèle « visual » | refonte visuelle semi-réaliste (shaders, terrain, marqueurs, modèles, battle_meshes/terrain) : ne pas toucher ces fichiers |
| F7 Contenu | **fusionné** : factions (3a49557) + F7b 40 événements (90 au total, 7 chaînes) | main | 13 factions, 18 personnages, 6 listes de noms, meubles héraldiques |
| F8 Tutoriel / encyclopédie | **fusionné** (touche L ; K = codex d'une autre session) | | manuel à écrire en F9 |
| F9 Recette | en cours (orchestrateur) | main | manuel écrit (6a36ffb), victoire tenue N saisons + sonde playthrough (0a5d9fa) ; reste : parties automatisées 3 factions, parcours des écrans, export .app, docs finales |

Prochaine étape : attendre les rapports de la vague 1, fusionner dans main, tests, puis vague 2.
Clé OpenRouter bloquée jusqu'au 1er octobre (limite propre 100 $/mois) : pas de portraits cette session.

## Coordination avec les sessions parallèles
- **game-project-76 (« visual »)** : refonte semi-réaliste, possède shaders, terrain, côtes, rivières, mer, marqueurs (villes, armées, constructions, chemin), `battle_meshes.gd`, `battle_terrain.gd`, assets models/textures, `tools/geo`, réglages de rendu de `project.godot`. Fusionne lui-même dans main après rebase.
- **game-project-e3 (« ui-tw », audit UI Total War, `docs/design/2026-09-23-audit-ui-total-war.md`)** : nouveaux fichiers `game/scripts/ui/{army_strip,general_seal,end_turn_cluster,news_letters}.gd` ; ordres de chef en bataille dans `sim-battle` + `data/battle_orders`. **F5 attend sa fusion.** Son lot A (défauts 1-5, 10-15 : economy.rs, movement.rs, map_ui.gd, province_panel.gd, thème) après fusion de F1/F2/F3 → **le prévenir quand c'est fusionné.**

## Défauts relevés à la revue (pour F9 / recette)
- (corrigé F4) Portugal : « la lignée s'éteint » au tour 3 (Hiver 1337) alors qu'Afonso IV a un héritier (Pierre Ier) — à diagnostiquer (succession / héritier non lié ?).
- (corrigé) Menu de départ : sous-titre et ligne d’état chevauchent les noms de villes de l'illustration (fond à dégager sous le texte).
- (pris par ui-tw lot A) Panneau de province : ligne de debug « Identifiant prov_… (index N) » à retirer.
- (pris par ui-tw lot A) Menu « Menu principal » de la barre du haut quitte sans confirmation (seul le menu pause confirme).
- Rapport de saison : n'inclut pas les batailles résolues par le dialogue après la fin du tour.
- **Session « historien »** (`docs/wip/historien.md`, conception `docs/design/2026-09-23-histoire-et-savoir.md`) : audit historique de `data/`, **Codex** (`data/codex/`, bulles imbriquées `game/scripts/codex/`, touche K) → **F8 encyclopédie doit s'appuyer sur le Codex** ; régimes alimentaires (`SetDiet`), 3e branche de techs Médecine, puis monnaie et rançons. Touche `tech_panel.gd`, `character_sheet.gd`, `chronicle_window.gd` (hooks minimaux) ; le panneau de province (section « La Table ») sera négocié avec ui-tw en vague 2.

## Portée ajoutée à F5 (demande ui-tw, audit UI § 3.2)
HUD de bataille (`battle_hud.gd`) : cartes d'unités moitié moins larges (icône de classe, effectif, barres fines moral/fatigue/munitions, plus de ligne « Formation »), noms jamais coupés au milieu d'un mot ; cartes groupées par « bataille » (avant-garde / corps / arrière-garde) avec Ctrl+1..9 ; plus de ligne d'aide permanente (F1) ; vitesses en icônes (pause, ×1, ×2, ×3) en bas à droite ; minicarte ; `leader_orders_bar` reste au-dessus des cartes (marge 216 px à ajuster par une constante).

- Autre flux découvert : lots H (session « historien » ? : régimes et médecine H3/H4, codex historique H2/H8 dans `data/codex`, touche K). L'encyclopédie F8 (données de jeu) et le codex (récit historique) coexistent.

## 24/09 matin
- 5 h 40 : branche `visual` (V1-V4 : éclairage, terrain splat/SDF/PBR/eau, végétation, villes, marqueurs, rendu de bataille) fusionnée dans main (d7336b5), smoke 16/16.
- Session « historien » (game-project-41) : lots H (régimes, médecine, codex, H7 : 20 événements). **Règle : commiter uniquement avec `git commit -- <chemins>`** (l'index est partagé ; un `-am` a embarqué sa fusion H7, corrigé en 44c48b0).

## Recette F9 (24/09 midi)
- Sonde `cargo run --release -p ai --example playthrough [graine]` (3 factions, IA aux commandes du joueur).
- Graine 1337 avant correctif : France victorieuse en 1343 (tour 24) — objectifs trop faciles. Après `hold_turns` (France 20, Angleterre/Bourgogne 12) : France 1371 (graine 1337), fin de campagne en 1454 (graines 1, 2) ; Angleterre fin 1454 (22-25 provinces) ; Bourgogne fin 1478 avec 2 provinces (faible, IA prudente en joueur) ; 0-15 ordres refusés.
- 8c81452 : personnages historiques épargnés par la mort naturelle avant leur date réelle − 2 ans (Philippe VI mourait avant 1340 dans ~6 % des parties).
