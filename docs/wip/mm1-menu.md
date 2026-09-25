# MM1 — première impression : écran titre, menu, choix de faction, chargement, introduction

Agent MM1 (session 7). Sources : `docs/audit/a3-ui.md` (§ 3.1, M1-M6), `docs/wip/au1-audio.md`,
`docs/wip/v3-atmosphere.md`, ADR 0014 (figurines skinnées) et 0015 (Paris).

## État : terminé (à fusionner)

- [x] Données : `data/ui/front_end.json` + schéma `data/schemas/front_end.schema.json` + test
  `tools/tests/test_front_end_schema.py` (schéma, fichiers référencés, toutes les factions jouables
  présentées) ; lecteur `game/scripts/ui/front_end_data.gd` (`FrontEndData`).
- [x] Décor 3D du menu `game/scripts/ui/menu_backdrop_3d.gd` (`MenuBackdrop3D`) : maquette L1
  `paris_siege.glb` (73 000 triangles, shader `landmark` à hauteur nulle), ciel HDRI V3 « dawn »
  teinté crépuscule + LUT, brume, soleil rasant, campagne `menu_ground.gdshader` (bras sud de la
  Seine prolongé, parcelles, berges), 963 arbres (`BattleMeshes.tree`), 36 000 touffes d'herbe
  (`menu_grass.gdshader`), ost de 422 figurines skinnées V2 (7 régiments France, Bourgogne,
  Gênes) en `MultiMesh` animés, 11 bannières et fanions au vent ; 3 plans de caméra lents
  (26 s) enchaînés par un fondu au noir ; nappes vent + campagne sur le bus Ambiance.
- [x] Menu principal (`start_menu.tscn/.gd`, refaits) : titre enluminé dessiné en code
  (`illuminated_title.gd`), colonne de boutons (Nouvelle partie, Continuer + détail de la
  sauvegarde, Charger, Prologue, Codex, Réglages, Crédits, Quitter), dégradés de lisibilité,
  légende du plan en bas à droite, fondus ; style commun `front_end_style.gd` (IM Fell English,
  EB Garamond, or/azur/vélin).
- [x] Choix de faction (`faction_select.gd`) : date de départ (1337 seule dans les données),
  3 cartes opaques (miniature, écu, accroche, souverain + portrait + titre, difficulté), fiche
  (intro, objectifs historiques, forces, faiblesses), « Options avancées » (graine), Retour /
  Commencer — <faction> ; clavier ← →, Échap, double-clic.
- [x] Écran de chargement (`loading_screen.gd`) : miniature encadrée (fonds commun + propre à la
  faction), même image assombrie en fond, citation datée, conseil, étape n/3, barre dorée et %.
- [x] Prologue (`intro_cards.gd`) : 5 cartons 1328-1337, musique « court », Ken Burns, passable
  (Échap, bouton), avance au clic / Espace ; joué une fois au premier lancement
  (`interface/intro_seen` ajouté à `settings.gd`), puis depuis le menu.
- [x] Smoke : contrôles MM1 ajoutés à `_run_start_menu` ; banc/captures `game/tests/mm1_capture.gd`.

## Mesures (M4 Pro, machine partagée, charge 50)
- Menu 3D : 60,0 i/s sur 20 s (Metal, plafond de l'écran ; Vulkan sans vsync : 60,1, également
  plafonné), pire image 23-30 ms (charge de la machine).
- Chargement : décor construit en ≈ 90 ms ; première image du menu 0,3 s après l'instanciation,
  0,8-1,7 s après le démarrage du moteur (< 3 s). Vulkan à froid (cache de shaders vide) : 5 s.

## Captures (`docs/audit/captures/mm1/`)
`01_menu_ost`, `01b_menu_cite`, `01c_menu_seine` (trois plans), `02_choix_faction`,
`03_prologue`, `04_chargement`. Commande :
`godot --path game --resolution 1440x900 --script res://tests/mm1_capture.gd -- --scene=menu
--menu-shot=0 --menu-shot-t=0.4 --out=<png>` (`--menu-stage=faction|intro`, `--fps-seconds=20`).

## Écarts et limites
- Aucune dépense : illustrations existantes (Codex, événements, factions) suffisantes.
- Une seule date de départ (les données n'en ont pas d'autre) ; l'interface affiche la liste.
- Barre de chargement : réelle pour les ressources (ResourceLoader) ; le `_ready` de la carte
  reste un bloc synchrone (≈ 2,4 s) pendant lequel la barre reste à 38 %.
- La maquette `paris_siege` n'a que la rive droite et la Cité : la rive gauche est la campagne
  procédurale (pas de faubourgs Saint-Germain) ; façade de Notre-Dame à une tour (gabarit L1).
- Pas de lumières aux fenêtres ni de fumées de cheminées au crépuscule.
- Headless ou `--no-menu-3d` : ancien fond 2D (`MenuBackground`).
- Premier lancement après un changement de shaders (cache vide) : première image en 3,2 s ; ensuite ≈ 1 s.
- Données de test (fixtures) : `FrontEndData` retombe sur `data/ui/front_end.json` du dépôt.

Fusion de `main` faite (conflit `settings.gd` résolu : clés BV1/BV2 + `interface/intro_seen`) ; import OK, smoke 24 « OK », exit 0.

## Prochaine étape éventuelle
Faubourgs de la rive gauche dans la maquette ; lumières de fenêtres ; autres dates de départ
quand les données existeront.
