# Serveurs MCP : Godot et Blender

Accès MCP (Model Context Protocol) de Claude Code à Godot 4.7 et Blender 5.2, configuré au niveau du dépôt via `.mcp.json` (racine). Aucun secret n'y figure ; Claude Code demande une confirmation à la première utilisation d'un serveur de projet.

Installé et vérifié le 2026-09-23 (macOS arm64, Godot 4.7.2, Blender 5.2.2 LTS, Node 22, uv 0.7).

## Vue d'ensemble

| Serveur | Paquet | Lancement | Prérequis à l'exécution |
|---|---|---|---|
| `godot` | `@coding-solo/godot-mcp` 0.1.1 (npm) | `npx -y @coding-solo/godot-mcp` avec `GODOT_PATH=/opt/homebrew/bin/godot` | Aucun : pilote Godot en ligne de commande, l'éditeur n'a pas besoin d'être ouvert |
| `blender` | `mcp-for-blender` 2.0.3 (PyPI, ex-`blender-mcp` d'ahujasid) | `uvx mcp-for-blender` | **Blender doit tourner en mode graphique** avec l'add-on activé (socket TCP `localhost:9876`) |

Les deux serveurs sont lancés automatiquement par Claude Code (transport stdio) à l'ouverture du projet ; il n'y a rien à démarrer à la main pour Godot. Pour Blender, voir ci-dessous.

## Ce qui a été installé, et où

- `~/dev/tools/godot-mcp/` : clone du dépôt `Coding-Solo/godot-mcp` (référence, non utilisé à l'exécution : `.mcp.json` passe par `npx`, qui met le paquet en cache dans `~/.npm/_npx/`).
- `~/dev/tools/mcp-for-blender/` : clone du dépôt `ahujasid/blender-mcp` (commit du 2026-09-21), source de l'add-on.
- `~/Library/Application Support/Blender/5.2/scripts/addons/blender_mcp.py` : add-on Blender (version 1.7, protocole 9), copie de `addon.py` du dépôt. Activé et sauvegardé dans les préférences utilisateur (`userpref.blend`) via `blender --background --python-expr`.
- Le paquet `mcp-for-blender` est résolu à la volée par `uvx` (cache `~/.cache/uv/`).

Note : `uvx mcp-for-blender install-addon` existe mais a écrit dans le dossier de Blender 5.0 (un ancien profil présent sur la machine), d'où la copie manuelle vers 5.2. Si Blender est mis à jour (5.3…), recopier le fichier dans le nouveau dossier de version et réactiver l'add-on.

## Compatibilité

- **Godot 4.7** : `godot-mcp` n'a pas de plugin éditeur ; il exécute `godot --headless` avec ses propres scripts GDScript. Les outils UID nécessitent Godot 4.4+. Vérifié : `initialize` + `tools/list` répondent, Godot détecté à `/opt/homebrew/bin/godot`.
- **Blender 5.2** : les rapports d'erreurs sous Blender 5.0 (issue #185 du dépôt, février 2026) ne se reproduisent pas avec la version actuelle de l'add-on. Vérifié : activation sans erreur, socket démarré, `get_scene_info` renvoie la scène (Cube/Light/Camera), le serveur MCP journalise `addon up to date (protocol 9, addon [1, 7], Blender 5.2.2 LTS)`.

## Serveur Godot (`godot`)

Configuration dans `.mcp.json` :

```json
"godot": {
  "command": "npx",
  "args": ["-y", "@coding-solo/godot-mcp"],
  "env": { "GODOT_PATH": "/opt/homebrew/bin/godot" }
}
```

Outils exposés (14) :

| Outil | Rôle |
|---|---|
| `launch_editor` | Ouvre l'éditeur Godot sur un projet |
| `run_project` | Lance le projet (ou une scène) et capture la sortie |
| `get_debug_output` | Récupère stdout/stderr du projet en cours |
| `stop_project` | Arrête le projet lancé |
| `get_godot_version` | Version de Godot installée |
| `list_projects` | Liste les `project.godot` sous un dossier |
| `get_project_info` | Structure du projet (scènes, scripts, assets) |
| `create_scene` | Crée une scène avec un nœud racine |
| `add_node` | Ajoute un nœud à une scène existante |
| `load_sprite` | Charge une texture dans un `Sprite2D` |
| `export_mesh_library` | Exporte une scène 3D en `MeshLibrary` |
| `save_scene` | Sauvegarde (ou copie) une scène |
| `get_uid` | UID d'un fichier de ressource (Godot 4.4+) |
| `update_project_uids` | Resauvegarde les ressources pour rafraîchir les UID |

Le projet Godot de Cent Ans est `game/` ; passer `projectPath` = chemin absolu de `game/` aux outils. `run_project` et `get_debug_output` sont les plus utiles pour vérifier `tests/smoke.gd` sans ouvrir l'éditeur.

Alternative évaluée, non retenue : `AkiraZ1/godot-mcp` (testé sur 4.7, ~50 outils « éditeur vivant » : nœuds, signaux, captures d'écran). Il impose d'installer un plugin dans `game/addons/` et de garder l'éditeur ouvert ; à reconsidérer quand `game/` sera stabilisé.

## Serveur Blender (`blender`)

Configuration dans `.mcp.json` :

```json
"blender": {
  "command": "uvx",
  "args": ["mcp-for-blender"],
  "env": {
    "BLENDER_HOST": "localhost",
    "BLENDER_PORT": "9876",
    "BLENDER_MCP_DISABLE_TELEMETRY": "1"
  }
}
```

`BLENDER_MCP_DISABLE_TELEMETRY=1` désactive la télémétrie anonyme du serveur (activée par défaut par l'auteur).

### Démarrage : Blender doit être ouvert

L'architecture est en deux parties : le serveur MCP (`uvx`) est un simple client TCP ; c'est **l'add-on dans Blender** qui exécute les commandes sur le thread principal de Blender. Il faut donc une instance graphique de Blender.

Commande recommandée (démarre Blender avec le socket prêt en quelques secondes, sans cliquer) :

```sh
/opt/homebrew/bin/blender --addons blender_mcp &
```

L'add-on a l'option « Auto-Start Server » activée par défaut : à l'ouverture de Blender, le socket `localhost:9876` est lancé automatiquement. Comme l'add-on est enregistré dans les préférences, un simple `blender` (ou l'ouverture de `/Applications/Blender.app`) suffit aussi ; `--addons blender_mcp` force juste l'activation si les préférences ont été réinitialisées.

Manuellement dans l'interface : vue 3D → touche `N` → onglet **MCP for Blender** → **Connect to Claude** (opérateur `bpy.ops.blendermcp.start_server()`).

Vérifier que le socket écoute : `nc -z localhost 9876`.

**Pas de mode headless pour l'add-on** : `blender --background` désactive explicitement l'auto-démarrage (`if bpy.app.background: return`) et, sans boucle d'événements, les `bpy.app.timers` dont dépend le serveur ne s'exécutent pas. Le fallback ci-dessous couvre ce cas.

Si Blender n'est pas ouvert, le serveur MCP démarre quand même mais chaque outil renvoie une erreur de connexion.

### Outils exposés (32)

Tous les outils prennent un argument obligatoire `user_prompt` (chaîne libre, journalisation interne).

Scène et exécution :

| Outil | Rôle |
|---|---|
| `get_addon_status` | Version de l'add-on, de Blender, état de la connexion |
| `get_scene_info` | Objets de la scène courante (nom, type, position) |
| `get_object_info` | Détails d'un objet (transformations, matériaux, modificateurs) |
| `get_viewport_screenshot` | Capture de la vue 3D (image) |
| `execute_blender_code` | Exécute du Python `bpy` arbitraire dans Blender |
| `describe_node_type` | Sockets et propriétés d'un type de nœud (shader/geometry) |
| `bpy_api_lookup` | Recherche dans l'API `bpy` |
| `export_scene` | Export de la scène ou de la sélection (GLB, FBX…) |
| `set_texture` | Applique une texture téléchargée à un objet |
| `disable_telemetry` | Coupe la télémétrie |
| `record_trajectory_feedback` | Retour utilisateur (télémétrie) |

Bibliothèques d'assets externes (accès réseau ; Poly Haven est gratuit et sans clé, les autres demandent une clé ou un compte à saisir dans le panneau Blender, jamais dans `.mcp.json`) :

- Poly Haven : `get_polyhaven_status`, `get_polyhaven_categories`, `search_polyhaven_assets`, `get_polyhaven_asset_preview`, `download_polyhaven_asset`
- Sketchfab : `get_sketchfab_status`, `search_sketchfab_models`, `get_sketchfab_model_preview`, `download_sketchfab_model`
- Poly Pizza : `get_polypizza_status`, `search_polypizza_models`, `download_polypizza_model`
- Hyper3D Rodin (génération 3D par IA) : `get_hyper3d_status`, `generate_hyper3d_model_via_text`, `generate_hyper3d_model_via_images`, `poll_rodin_job_status`, `import_generated_asset`
- Hunyuan3D (génération 3D par IA) : `get_hunyuan3d_status`, `generate_hunyuan3d_model`, `poll_hunyuan_job_status`, `import_generated_asset_hunyuan`

Ces intégrations sont désactivées par défaut (cases à cocher du panneau). Toute utilisation payante doit être consignée dans `docs/budget.md`.

## Fallback : Blender sans MCP

Blender se pilote entièrement en ligne de commande, sans add-on ni interface :

```sh
/opt/homebrew/bin/blender --background --python script.py -- arg1 arg2
/opt/homebrew/bin/blender --background fichier.blend --python-expr "import bpy; bpy.ops.export_scene.gltf(filepath='out.glb')"
```

C'est la voie à privilégier pour tout ce qui doit être reproductible (pipeline d'assets, export batch, CI) : un script versionné dans `tools/` vaut mieux qu'une session MCP interactive. L'agent `tools/` fournit un helper Python (`uv run --project tools …`) encapsulant ces appels ; le MCP Blender sert surtout à l'exploration interactive (inspecter une scène, tester un matériau, capturer la vue).

De même pour Godot, `godot --headless --path game --script res://tests/smoke.gd` reste la référence pour les tests ; le MCP est un confort, pas une dépendance.

## Vérification manuelle

Envoyer un `initialize` JSON-RPC sur stdio à chaque serveur :

```sh
printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"probe","version":"0"}}}' \
  | GODOT_PATH=/opt/homebrew/bin/godot npx -y @coding-solo/godot-mcp | head -1
```

Dans Claude Code : `/mcp` liste les serveurs et leur état de connexion.
