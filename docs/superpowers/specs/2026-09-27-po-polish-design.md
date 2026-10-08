# PO — Polish : rendre le jeu plus beau et moins « amateur »

Date : 2026-09-27. Statut : design validé par le joueur (brainstorming), en attente de relecture de la spec.
Demande : « comment rendre le jeu plus joli et moins 'amateur' ? » → plan global par rendement (réponse D),
refonte de la disposition de l'UI autorisée (réponse A), approche en tranche verticale.

## 1. Diagnostic (captures du 27/09)

L'art de base est déjà travaillé (bible DA, DA0-DA7, figurines fines FG, portraits DA2). L'effet
« amateur » vient de ce qui relie les assets :

- **Interface** (`docs/img/cv3/ui-postures.png`) : six fenêtres flottantes qui se chevauchent,
  texte trop petit, aucune hiérarchie ; un message technique affiché au joueur (« Relief rapproché
  limité… `uv run`… Copier », `game/scripts/map/relief_cache_status.gd`).
- **Carte de campagne** (`docs/img/cv3/carte-05-relief-regional-paris-orleans.png`) : aspect de SIG,
  vert plat uniforme, forêts en pâtés de boules identiques, brume grise sans lumière, étiquettes en
  pastilles blanches génériques.
- **Bataille** (`docs/img/fg/fg5_melee_fine.png`) : lumière de midi dure sans étalonnage, herbe en
  taches répétées, sol en basse résolution, cadre de sélection jaune vif, rangs rigides, arbres
  « sucette » à l'horizon.

## 2. Périmètre

**Tranche verticale** (les 15 premières minutes) :

1. écran titre ;
2. choix de faction ;
3. début de campagne (Paris, 1337) ;
4. sélection et déplacement d'une armée ;
5. fin de tour ;
6. rencontre, puis déclenchement de bataille ;
7. déploiement, mêlée, déroute ;
8. écran de résultat, puis retour à la carte.

**Phase 2** (après validation de la tranche) : cour, techniques, diplomatie, codex, encyclopédie,
arbre familial, fiche personnage, sièges, naval (rendu automatique seulement). Elle applique le
gabarit du § 4 de façon mécanique.

**Hors périmètre** : nouveaux modèles 3D payants, changement de règle (tout reste dans `game/`,
`core/` n'est pas touché), écrans hors tranche pendant la phase 1.

## 3. Critères de réussite

| # | Critère | Vérification |
|---|---|---|
| C1 | Aucun texte technique visible (commande, chemin, identifiant brut) hors `dev_mode` | test headless qui parcourt les `Label`/`RichTextLabel` visibles de la tranche et refuse `uv run`, `res://`, `user://`, `--`, les chemins de fichier et les mots en snake_case (identifiants bruts) |
| C2 | Disposition fixe : au plus un panneau `side_panel` ouvert ; aucun panneau ne chevauche `top_bar`, `bottom_selection`, `minimap` | test headless de rectangles sur la tranche, à 1280×720 et 1920×1080 |
| C3 | Échelle typographique : 4 tailles, 14 px minimum à 1080p | test du thème et des surcharges de taille dans les scripts de la tranche |
| C4 | Étalonnage : une heure du jour et un étalonnage par contexte, réglés dans `data/fx/atmosphere.json`, sans valeur de soleil codée en dur | test de schéma + test que chaque contexte de la tranche (carte × saison, bataille × météo × heure) résout un préréglage complet |
| C5 | Jugement visuel | planche avant/après (8-10 vues) jugée une seule fois par le joueur à la fin de la vague 1 |

Les performances ne régressent pas : le banc PB1 (`docs/archive/chantiers.md`) reste dans
±5 % sur la carte et en bataille.

## 4. Fondations (PO0, orchestrateur, 0 $)

1. **Planche « avant »** : captures headless de la tranche dans `docs/img/po/avant/` (script
   `game/tests/po_shot.gd`, 1280×720) et inventaire des défauts par écran dans `docs/wip/po.md`.
2. **Gabarit de style** : nouvelle section « § 12 Gabarit d'interface et d'étalonnage » de la
   bible DA (`docs/design/2026-09-25-bible-da.md`), figée par l'ADR 0097 (0096 est pris par AN1) :
   - **Zones d'écran** : `top_bar`, `bottom_selection`, `minimap`, `side_panel` (une seule
     occupante, l'ouverture d'une autre ferme la précédente), `toasts` (conseils, annonces,
     notices : pile en haut à droite, effacement automatique), `modal` (choix bloquants seulement,
     fond assombri).
   - **Échelle typographique** : `title`, `heading`, `body`, `caption` (tailles exactes fixées dans
     l'ADR, `caption` ≥ 14 px à 1080p), exposées comme variations de type du thème
     `game/scenes/ui/parchment_theme.tres`.
   - **Espacements** : grille de 8 px (4 / 8 / 16 / 24), constantes de thème.
   - **Étalonnage** : on réutilise la LUT 3D procédurale existante (`AtmosphereLibrary.grade_lut`,
     blocs `grades` de `data/fx/atmosphere.json`). On ajoute la clé `time_of_day` (préréglages
     `morning`, `midday`, `evening` : élévation, azimut et couleur du soleil, étalonnage) dans
     `atmosphere.json` et son schéma. Les préréglages de soleil codés en dur dans
     `battle_atmosphere.gd` (l. 19-40) y sont déplacés.
3. **Squelette** commité avant la vague : autoload `UiLayout` (API publique, corps vides),
   drapeau `dev_mode` (réglage + argument `--dev`), tests PO désactivés, `docs/wip/po.md`.

## 5. Lots

### Vague 1 (5 agents en worktree, fusion par l'orchestrateur)

| Lot | Objet | Contenu | Dépend de |
|---|---|---|---|
| **PO1** | Disposition fixe | `UiLayout` : `claim(zone, control)`, `release(control)`, signal `side_panel_changed`. Migration des panneaux de la tranche : barre du haut, bande d'armée (`army_strip`), province (`province_panel`), fin de tour (`end_turn_cluster`), conseiller et journal passés en toasts, lettres (`news_letters`), minicarte. `relief_cache_status` et tout message d'outil ne s'affichent qu'en `dev_mode`. Tests C1 et C2. | PO0 |
| **PO2** | Typographie et finition UI | Variations de type et constantes d'espacement appliquées au thème ; suppression des tailles en dur dans les scripts de la tranche ; états survol / pressé / désactivé / focus ; fondu d'ouverture et de fermeture ≈ 120 ms (`Tween`) ; sons de clic et d'ouverture (CC0, `game/assets/third_party/`). Test C3. | PO0 ; fusion avant PO1 si conflit de thème |
| **PO3** | Carte de campagne | Soleil rasant doré par saison (`campaign_atmosphere.gd`) ; LUT campagne ; perspective aérienne teintée (bleu-or, pas gris) ; forêts : variation de teinte, d'échelle et de densité, lisières plus basses ; étiquettes de villes dans le registre manuscrit (encre sur fond clair sans pastille, graisse selon le rang, bible § 4). Respecte la saturation ≤ 35 % (DA7b, `tools` `scene_saturation.py`). | PO0 |
| **PO4** | Bataille | Préréglages d'heure (`matin`, `soir`, `couvert`) : soleil bas, ombres longues, LUT ; texture de détail proche du sol (CC0 Poly Haven) ; herbe en touffes avec variation plutôt qu'en taches ; léger désordre visuel des rangs (décalage de position au rendu seul, déterministe par graine d'unité, core inchangé ; les animations et tissus relèvent d'AN1) ; arbres lointains : vérifier que DA6 couvre l'horizon, sinon imposteurs à feuillage. **Le style du marqueur de sélection attend CB-M1** (voir § 6). | PO0 |
| **PO5** | Mouvement et transitions | Caméra carte et bataille : inertie, amorti, zoom lissé ; transitions fondu/volet carte ↔ chargement ↔ bataille ↔ résultat ; transition de fin de tour (fondu du sceau, changement de saison) ; retour visuel de clic (onde d'encre à l'ordre de déplacement sur la carte). Marqueurs d'ordre en bataille : style seulement, après CB-M1. | PO0 ; coordonné avec CB |

Tous les lots : `smoke.gd` passe, captures produites par script (`*_shot.gd`), pas d'outil de
capture ni d'éditeur pour les agents (règle CLAUDE.md), commits `wip:` toutes les 15 min, fichier
`docs/wip/po<N>-<objet>.md`.

### Vague 2

- **PO6** (orchestrateur) : intégration dans `feat/po-polish`, puis ff dans main ; planche
  « après » `docs/img/po/apres/` ; jugement du joueur (C5) ; corrections regroupées en un lot.
- **Phase 2** : lots mécaniques (Sonnet), un par groupe d'écrans hors tranche. Chacun migre ses
  panneaux vers `UiLayout` et le thème, sans nouvelle règle de style.

## 6. Coordination

- **AN1/PR1** (`docs/wip/an1-animation-vivante.md`) possèdent le shader des soldats (sommets), les
  clips, les étendards et la retexture des accessoires. PO4 n'y touche pas.

- **CB-M1** (contrôles de bataille, `docs/archive/chantiers.md`) possède la sélection et les ordres en
  bataille. Le cadre de sélection jaune et les marqueurs d'ordre sont restylés par PO4/PO5
  **après** la fusion de CB-M1, en ne touchant qu'au matériau et à la couleur.
- **HL1/HL2** (liste des colonies, `docs/archive/chantiers.md`) et **CV3** (`docs/archive/chantiers.md`) :
  leurs nouveaux panneaux rejoignent `side_panel` ou `modal` de `UiLayout` ; note ajoutée dans
  leurs wip au lancement de PO1.
- Commits avec chemins explicites (`git commit -- <chemins>`), pas de `git stash`, cible cargo
  privée par worktree (mémoire du projet).

## 7. Budget

0 $ prévu : LUT procédurales existantes, textures CC0, sons CC0. Enveloppe de
sécurité ≤ 3 $ (section « Polish » de `docs/budget.md`) si des cadres d'interface doivent être
régénérés dans le registre enluminure.

## 8. Risques

- **Boucles de mise en page** (déjà vues avec UI1, marges 9-slice) : `UiLayout` fixe les
  rectangles des zones et ne dépend pas de la taille minimale des enfants.
- **Conflits de fusion** sur `campaign_map.gd` et le thème : PO2 fusionne en premier, puis PO1 ;
  PO3 et PO4 ne touchent pas l'UI.
- **Perf** : les LUT et le bruit de détail coûtent peu ; toute hausse de plus de 5 % au banc PB1
  bloque la fusion du lot.
