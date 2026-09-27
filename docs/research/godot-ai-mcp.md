# godot-ai : MCP d'éditeur Godot en direct — essai du 2026-09-27

**Décision : non adopté en permanence.** Règles de consommation de contexte versées dans
`CLAUDE.md` (section « Vérification visuelle et MCP d'éditeur »). À ressortir pour un gros
chantier d'UI, installé localement le temps de la session, jamais versionné.

## Pourquoi pas en permanence
- Le cœur du travail (règles en Rust dans `core/`, vérification headless) n'en profite pas.
- Coût : ≈ 14 000 tokens de schémas par chargement (domaines utiles), un éditeur ouvert par
  session/worktree, ports 8000/9500 partagés — incompatible avec les vagues d'agents.
- Intrusif : réécrit `project.godot` et injecte un autoload `_mcp_game_helper` dans le **jeu**
  (casserait l'export si le plugin en est exclu) ; 313 fichiers dans `game/addons/`.
- Projet très mouvant (3 versions en 3 jours, migration v3 → v4 récente).

## Réinstaller pour une session d'UI
1. Télécharger `godot-ai-v4-plugin.zip` depuis https://github.com/hi-godot/godot-ai/releases,
   vérifier le sha256 contre `godot-ai-v4-plugin.manifest.json` (champ `asset.sha256`).
2. Dans un worktree dédié : extraire dans `game/`, activer le plugin (`[editor_plugins]`),
   ajouter `addons/godot_ai/*` aux `exclude_filter` d'export. Ne rien commiter dans main.
3. Précharger (sinon le 1er démarrage échoue sur un délai uvx de 30 s) :
   `UV_HTTP_TIMEOUT=300 uvx --isolated --no-config --from godot-ai==<version> godot-ai --help`
4. Lancer l'éditeur : `GODOT_AI_DISABLE_TELEMETRY=true godot --editor --path game`.
   Exclure les domaines inutiles dans les réglages du plugin (côté serveur : un client qui
   demande d'autres domaines est refusé).
5. Client : entrée MCP locale (pas `.mcp.json` partagé)
   `uvx --isolated --no-config --from godot-ai==<version> godot-ai attach --port 8000 --ws-port 9500 --disable-telemetry`,
   ou `tools/experiments/probe_godot_ai.py` (client stdio de mesure : `tools`, `schema`, `call`).
6. En fin de session : fermer l'éditeur, tuer le serveur orphelin (`pgrep -fl godot-ai`),
   supprimer `godot_ai_server.pid` dans le dossier user de « Cent Ans », supprimer le worktree.

## Mesures
- Schémas des outils : **47 outils ≈ 27 400 tokens** (109 557 caractères). Plus gros :
  material_manage 1 712, resource_manage 1 682, logs_read 1 060, game_manage 1 031.
- Avec seulement les domaines utiles (scene, node, script, project, game, logs, editor,
  test, ui, filesystem, batch) : **≈ 13 900 tokens**. L'exclusion se règle côté serveur
  (réglage éditeur), pas dans `godot-ai attach` : un client qui demande d'autres domaines
  est refusé.
- Claude Code diffère les outils MCP (ToolSearch) : le coût réel n'est payé que quand la
  session charge les outils ; les sous-agents qui les chargent paient à nouveau.
- `editor_screenshot` : `max_resolution` par défaut **640 px** (≈ 300 tokens en 16:9),
  0 = pleine résolution (Retina : plusieurs milliers). Sources viewport/viewport_2d/
  cinematic/game.
- `game_manage(op="get_ui_elements")` : inspection texte de l'UI du jeu en cours
  (chemin, type, texte, rect) — alternative à la capture pour vérifier une UI.

## Effets de bord constatés
- L'éditeur (plugin actif) réécrit `project.godot` : réordonnancement, format des
  événements d'entrée, et **ajout d'un autoload `_mcp_game_helper`**
  (`res://addons/godot_ai/runtime/game_helper.gd`) — le plugin injecte du code dans le
  **jeu**, pas seulement dans l'éditeur. Avec `addons/godot_ai/*` exclu de l'export,
  l'autoload pointerait vers un script absent dans le jeu exporté : il faudrait le retirer
  avant tout export.

## Essai en jeu
- `project_run` main : jeu « live » en ≈ 20 s (helper prêt), sondage par `editor_state`.
- Capture `source=game` 640×400 : **≈ 340 tokens**, lisible pour la mise en page, petits
  textes (sous-titres 10 px) à peine lisibles. `max_resolution` 1280 ou 0 : **échec de
  transport** (PNG ≈ 1 Mo trop gros pour le pont) → rester à 640.
- `get_ui_elements` : menu principal 23 éléments ≈ 1 700 tokens ; écran de choix de
  faction 144 éléments **≈ 16 000 tokens** → toujours filtrer (`root_path`, `max_depth`).
- Entrées : clic souris et touche (`F`) fonctionnent. Scénario `mf1_shot` rejoué : menu
  principal → Nouvelle partie → Commencer (France) → carte chargée (~2 min) → `F` → menu
  des filtres capturé → clic filtre. Le clic « Richesse » estimé depuis la capture 640 a
  touché « Loyauté » (une ligne d'écart) : prendre les rects exacts dans la liste d'UI.
- Coût total de l'essai MF1 via MCP : 3 captures ≈ 1 000 tokens + listes UI ≈ 18 000.
  Le script `mf1_shot.gd` (44 lignes, lancé en headless) reste moins cher pour une preuve
  reproductible ; le MCP gagne pour l'exploration interactive (naviguer sans écrire de script).

## Bilan
- Utile en session principale pour explorer une UI sans écrire de script.
- Pour une preuve reproductible, un `game/tests/*_shot.gd` headless reste moins cher.
