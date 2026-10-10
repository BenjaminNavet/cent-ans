# UX5 — diplomatie, savoirs, recrutement, Colonies, Unités

Demande joueur (2026-10-10) : améliorer ces cinq écrans en s'inspirant des meilleures pratiques
du genre (TW Warhammer III, CK3, EU4/5, Victoria 3, Civ VI/VII, Old World, Bannerlord…),
sans quitter l'esprit médiéval ni la charte parchemin (`hud_style.gd`, ADR 0135).

Méthode : 5 agents Sonnet de recherche (un par écran), puis lots d'implémentation.
Réserve des agents : la recherche web a peu rendu ; les inspirations reposent surtout sur
la connaissance générale des jeux cités.

## Charte commune (tous lots)
- Encre, cire, rubrique : `HudStyle.INK/INK_SOFT/INK_FADED`, `RUBRIC` (négatif), `WAX`,
  `WAX_GREEN`, `GOLD`, `GOOD/FAIR/POOR` ; pas de pastilles saturées, pas de toasts, pas de
  « chance % ».
- Pas de grille de tableur : filets `INK_SOFT` entre colonnes, en-têtes en petites capitales.
- Couleur jamais seule : un glyphe ou un sceau double toujours l'état.

## Synthèse retenue (lots Godot seuls, sans Rust)

### D — Diplomatie (`game/scripts/ui/diplomacy*`)
- ÉTAT (branche ux5/d) : D1-D5 FAIT (voisinage non exposé par `get_diplomacy` : tri attitude/puissance seulement). Test `ux5_d_test.gd`.
- D1 Liste : tri (attitude, puissance, voisinage) + recherche (`diplomacy_faction_list.gd:47-55`).
- D2 Bloc « Opinion » trié par poids avec total, gloses rubriquées (négatif) / encre (positif),
  remplace la ligne de légende (`diplomacy_head_section.gd:65-73`).
- D3 Jauge d'accord : repère du seuil à la plume, distance au seuil ; retirer « chance % »
  (`diplomacy_negotiation_tab.gd:239-262`).
- D4 Bandeau « Affaires de la Cour » à l'ouverture : offres en attente, trêves/pactes
  qui expirent (`turns_left`), ligue active.
- D5 Tendance ↑↓ de l'attitude par cache UI par tour (sans Rust ; vide après chargement).

### T — Savoirs (`tech_panel.gd`, `tech_tree_view.gd`)
- T1 Pastilles de déblocage (unités/bâtiments) + effet clé sur le nœud.
- T2 Chemin surligné au survol (ancêtres non acquis + descendants en or, reste estompé),
  « Verrouillée : requiert X ».
- T3 Recherche + filtres (« À portée », « Débloque une unité », « Acquises estompées »).
- T4 Ruban de file : 3 sceaux, tours restants, fin de file au tour N (affichage seul).
- T5 Ouverture centrée sur la recherche en cours / premier savoir disponible.
- T7 Glyphe d'état doublant la couleur ; `STATE_COLORS` → jetons `HudStyle`.

### C — Colonies (`game/scripts/map/holdings_controller.gd`)
- C1 Lignes en colonnes alignées (nom, revenu, ouvrage, garde, alertes) — défaut l.285-287.
- C2 Sceaux d'alerte avec infobulle (chantier libre, promotion, siège, révolte, file vide ;
  `recruit_queue_len`, `garrison_free` déjà exposés).
- C3 Tri par en-tête de colonne, sens inversable, appliqué aux colonies.
- C4 Filtres cumulables + recherche par nom (l.16, l.337).
- C9 Ligne de totaux en pied (`settlement_income_total`).

### R — Recrutement (`game/scripts/map/settlement_panel.gd`, `panel_widgets.gd`)
- R1 Pool régional visible : gouttes de cire pleines/vides + « +1 dans K saisons » (`pool_*` exposés).
- R2 Panier : clic = 1, Maj+clic = 5, clic droit = retirer, plafonné par `recruit_slots_free` ;
  compteur sur la carte.
- R3 Pied « Montre » : coût total, solde ajoutée par saison, trésor après ; bouton « Sceller la levée »
  qui émet les `recruit_requested`.
- R4 Cartes d'unités en grille (icône, nom, « hommes · délai », prix, raison si grisée) ;
  Rust : exposer `soldiers`, `recruit_time_turns`, `category` dans `get_recruitable`.
- R5 Onglets de catégorie (Tous / Pied / Trait / Cheval / Engins) + tri par prix.

### U — Unités (`game/scripts/map/unit_roster_controller.gd`)
- U1 Sceaux d'alerte par ligne : sans chef (rubrique), vivres bas (`supply`), sous-effectif
  (strength/max_strength), hors mouvement.
- U2 Ligne sur 2 niveaux : mini-sceau du chef (`general_seal.gd`), effectif « 840/1200 », moral moyen,
  état en glyphe (marche, siège, campement, embarqué).
- U3 Tri (effectif, mouvement restant, lieu, alerte) + filtres (sans chef, en siège, à bout de mouvement) ;
  ne plus tout reconstruire (garder le défilement).
- U4 Pied « Montre du royaume » : hommes, osts, entretien total (`army_upkeep`).
- U8 Tab / Maj+Tab parcourent les osts (vérifier les actions existantes de `project.godot`).

## Reporté (exige du Rust ou une refonte)
- Diplomatie : aperçu des alliés qui suivront une déclaration (`war_verdict`), contre-offres
  multiples, journal des parjures (`TreatyRecord.kind`), négociation à tours.
- Savoirs : eurêkas médiévaux, cartes à tirer, réordonner la file (`reorder_research_queue`).
- Recrutement : contres d'unités (champ `counters` en données), glisser sur l'armée, entretien projeté
  avec revenu net, arrière-ban (ordre `Muster`).
- Unités : prévision d'usure, actions rapides par ligne, fusion d'osts, onglet garnisons.
- Colonies : ordre public par colonie + durée totale du chantier (champs de lecture),
  regroupement par seigneur, actions en masse.

## État
- [x] 5 rapports (2026-10-10)
- [ ] Lots D, T, R, C, U (worktrees `../gp-ux5-<lot>`, branches `ux5/<lot>`), consignes `docs/wip/ux5/brief-lot.md`
