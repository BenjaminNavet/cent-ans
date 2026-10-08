# Q1 — recette et intégration (25/09, après les fusions de la nuit)

Partie jouée en fenêtre par un pilote qui envoie de vraies entrées (clics souris, touches) et
capture chaque écran : `game/tests/q1_playtest.gd`.

```
godot --resolution 1920x1080 --path game --script res://tests/q1_playtest.gd -- \
  --out=<dossier> [--faction=fac_england] [--turns=16] [--phase=all|battle|siege|actions|panels|turns|save]
```

Parcours : menu → carte de faction → écran de chargement → campagne ; bataille mise en scène
depuis la carte (avant-bataille → déploiement → combat ×4 → retraite générale → écran de fin →
retour carte) ; assaut de Bordeaux en résolution automatique ; zooms sur la capitale ;
recrutement et construction au clic dans le panneau de province ; sélection d'armée au clic,
aperçu et ordre de déplacement libre au clic droit ; tous les panneaux par leur raccourci
(P, C, T, G, K, L, O, F1, N, R, M), fiche de personnage, panneau de faction, chaque entrée du
menu ; 16 fins de tour (Entrée) avec rapports de saison, décisions de chronique et tutoriel ;
F5 / F9 ; menu pause, les 4 onglets de réglages, qualité basse / ultra / haute, taille
d'interface 1,25 / 0,8 / 1. France en 1920×1080 et 1280×720, Angleterre en 1920×1080.

Captures commentées : `docs/audit/captures/q1/`.

## Corrigé (un commit chacun)

| # | Défaut | Lot | Correctif |
|---|---|---|---|
| C1 | **Plantage (panique Rust) de la « Retraite générale »** : l'ordre part avec un `Array[int]` typé, que le pont lisait comme `VarArray` → `FromGodot::from_variant() failed`, « Invalid call error code 1337 », la retraite n'est jamais donnée. Touche aussi Halte / Retraite / Tir à volonté sur une sélection (`_available_selection()` est typé). | UB1 / pont | `convert.rs` : `variant_to_json` lit `AnyArray` / `AnyDictionary` (c771ab84) |
| C2 | Plaques d'effectifs des armées (« 540 », « 620 ») et jetons d'agents de la **carte de campagne affichés par-dessus la bataille 3D** (calques 2D non masqués avec la carte) | CV2 / C6 | `campaign_map._set_campaign_active` masque puis restaure les `CanvasLayer` (3d6a4d3d, capture 04) |
| C3 | **Bandeau de notification invisible** sous les panneaux ancrés et sous le rapport de saison (« Construction lancée », « Recrutement… », refus d'ordre) | U1 / UI2 | `MapUI.show_toast` le remet au premier plan (c7fb18fb, captures 01-02) |
| C4 | Bandeau de fin de tour « Bataille de Toscane : Florence attaque Vérone » : **première bataille venue, même étrangère** | M7 / F3 | seulement les batailles qui concernent le joueur (a499485c) |
| C5 | **Menu → « Son… » ouvrait aussi les Objectifs** (même id 900 dans le menu que « Objectifs (O) ») | AU1 / M10 | id 930 (53c3d970) |
| C6 | Panneau **Objectifs étiré hors de l'écran** en 1080p (libellés à retour à la ligne, hauteur minimale calculée à largeur nulle), moitié basse vide | U1 | largeur fixe des libellés, recentrage différé (a2fe2dc2, capture 17) |
| C7 | Aide F1 : « L : tutoriel » (L ouvre l'encyclopédie) ; volumes indiqués dans « Menu → Son… » alors qu'ils sont dans Réglages → Son (6 bus) | F8 / AU1 | texte d'aide (f1c7da4b) |
| C8 | **Rapport de saison sans date** (« Rapport de saison — ») quand une bataille est livrée avant la première fin de tour | UB1 / F3 | la date est passée avec les événements tardifs (5c3089ba) |

Après la fusion de main (0c082a82 : UI3, MM1, SG1) : C4 et C7 recouvrent des changements d'UI3
(filtre `keeps_news`, fiche des raccourcis générée depuis l'InputMap) ; la version d'UI3 a été
gardée. C1, C2, C3, C5, C6 et C8 restent nécessaires et ont été revérifiés en jeu.

## Restant, par priorité

### P1 — à traiter vite

1. **C4 (édits) et C5 (commerce) ne sont pas dans main.** Ils sont fusionnés dans
   `integration/tw` (6ebe9658, 7d754085) mais `integration/tw` n'a pas été avancé dans main
   (seul B8b l'a été, 95057829). Aucun édit ni route commerciale en jeu : impossible à recetter.
   Reproduire : `git merge-base --is-ancestor integration/tw main` → faux. Lot suspect :
   orchestration TW (`docs/archive/chantiers.md`). Fusion à faire par l'orchestrateur (conflits attendus
   `economy.rs` / `movement.rs` après M2-M4).
2. **Figurines d'armée CV2 géantes au zoom rapproché** : à distance 10 sur Paris, l'ost
   anglais (bannière, cavaliers, piquiers) fait la taille de la ville entière et la recouvre
   (capture 05). À distance 25 déjà, la bannière dépasse Saint-Denis. Reproduire : caméra sur une
   ville où stationne une armée, molette jusqu'au plus près. Lot : CV2
   (`game/scripts/map/army_figures.gd`, `army_markers.update_scale`) — l'échelle devrait être
   plafonnée par rapport aux bâtiments de L1/BR1 au palier « près ».
3. **Clic sur une ville où stationne une armée : impossible d'ouvrir la province.** Le clic
   gauche sur Paris sélectionne toujours l'ost royal ; il faut cliquer dans la campagne pour
   ouvrir le panneau de province (recrutement, construction). Un joueur ne trouve pas le
   recrutement de sa capitale au premier tour. Lot : M4 / C5 (`campaign_map._try_select_army`,
   `settlement_controller`) — un second clic ou un double clic sur la ville devrait ouvrir la
   ville ; ou un bouton « Ville » dans le bandeau d'ost.
4. **Écran de fin après « Retraite générale » : « tient le champ » pour tous les régiments**,
   pertes 0 / 0, alors que toute l'armée s'est retirée ; la bataille se termine à l'instant du
   clic (captures 06, 11). Le sort attendu est « s'est retiré ». Lot : UB1
   (`battle_result_screen.gd`) ou cœur (`sim-battle`, statut des unités en retraite).

### P2 — gênant

5. **Performances en qualité « Ultra » sur la carte** : 26 FPS en 1080p (41 avec l'Angleterre),
   contre 40 en « Haute » et 46 en « Basse », sur une vue proche de Paris (12 M de primitives,
   900 appels de dessin). À 720p : 53 / 61 / 61. Le préréglage Basse ne réduit ni les primitives
   ni les appels de dessin (même 12,3 M, 903) : il ne touche que les effets d'écran. Les
   captures Basse et Ultra sont presque identiques (09, 10). Lot : V3 (`RenderQuality`) / T2
   (en cours : ne pas y toucher ; à transmettre) — la qualité pourrait aussi piloter la densité
   de végétation et les distances de LOD.
6. **FPS au démarrage de la campagne irréguliers** : 43-60 selon les parties (vsync 60), carte
   entière visible (2,3 M primitives). À surveiller par T2.
7. **Tutoriel à l'étape 2 pendant 6 tours** alors que le joueur a déjà sélectionné l'ost royal,
   recruté et construit ; il s'affiche par-dessus la fenêtre de décision de la chronique et le
   menu pause (capture 07). Lot : F8 (`tutorial_controller.gd`) / UI3 (en cours).
8. **Deux entrées de volume** : Menu → « Son… » (fenêtre `AcceptDialog` native, style système)
   et Réglages → Son. Mêmes réglages, deux fenêtres différentes. Supprimer « Son… » du menu
   (AU1) ou le rediriger vers l'onglet Son des réglages. Lot : AU1 / UI3.
9. **Curseurs du réglage Son inversés visuellement** : partie gauche claire, partie droite
   foncée — la zone « remplie » semble être à droite du curseur (capture 08). Lot : UI2
   (`parchment_theme.tres`, style `grabber_area` des HSlider).
10. **Angleterre : solde négatif dès le tour 1** (−1 238 ₶ / saison, capture 13). Choix
    d'équilibre possible, mais aucune alerte ni conseil au joueur. Lot : G1 (en cours).
11. Ordre refusé (« cette armée n'a plus de points de mouvement ce tour ») : le bandeau reste
    3,5 s et se superpose au titre du panneau ouvert ensuite (capture 12). Mineur. Lot : UI3.

### P3 — finitions

12. Rapport de saison et fenêtre de chronique : la chronique s'ouvre en fin de tour par-dessus
    le rapport de saison ; deux fenêtres modales à fermer à chaque tour chargé. Lot : UI3.
13. L'écran de fin de bataille en 720p coupe « Faits notables » derrière le bouton « Retour à la
    campagne » (défilement nécessaire, capture 11). Lot : UB1.
14. Réglages : aucun réglage « taille des unités » (BV1 non fusionné) ; le réglage « Sang »
    (BV2) existe. À vérifier lors de la réconciliation BV1/BV2 (clé `battle/blood` commune).
15. Fuites à la sortie : « 44 ObjectDB instances were leaked at exit » à chaque partie (285 et
    des RID de textures quand un script échoue). Sans effet en jeu. Lot : T2.

## Mesures

| Mesure | 1920×1080 | 1280×720 |
|---|---|---|
| Menu → carte prête | 6,0 s | 5,8 s |
| Fin de tour (16 tours, France) | 250-430 ms | 250-570 ms |
| Chargement d'une bataille | 3,2-3,9 s | 3,2 s |
| FPS bataille (8 contre 6 régiments, ×4) | 53-62 | 60-61 |
| FPS carte, vue Paris, Basse / Haute / Ultra | 46 / 40 / 26 | 61 / 61 / 53 |

Aucune erreur de script dans la console sur les parties complètes après correctifs (hors
panique C1, corrigée). Aucune fin de tour anormalement lente.

## Non couvert

- Édits et commerce : absents de main (point 1).
- Agents : panneau (G) ouvert, aucun agent recruté ni mission jouée.
- Diplomatie : panneau parcouru, aucune proposition envoyée.
- Sièges joués en 3D (« Donner l'assaut ») : SG1 en cours, seulement la résolution automatique.
- Bourgogne.
