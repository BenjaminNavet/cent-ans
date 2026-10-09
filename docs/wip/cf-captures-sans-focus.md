# CF — captures sans voler le focus (2026-10-09)

Demande du joueur : que les chantiers puissent faire des captures pendant qu'il joue, sans
lui prendre le focus.

## Constats
- `tools/godot_bg.sh` (`open -g`) ne suffit pas : Godot macOS s'active lui-même à la création de
  la fenêtre (mesuré : 2 passages au premier plan, ≈ 1,7 s), même avec
  `display/window/size/no_focus=true` et `--position -5000,-5000` (le rendu hors écran, lui, marche).
- Docker (Linux arm64 + Xvfb + Mesa) rend correctement, Vulkan (lavapipe) comme OpenGL.
- `docker desktop start` vole aussi le focus ≈ 1,5 s (une fois par démarrage de Docker).

## Plan
1. `tools/godot_shot/Dockerfile` + `tools/godot_shot.sh` (même interface que godot_bg.sh, code de
   sortie transmis) — squelette.
2. GDExtension Linux compilée dans le conteneur (cible dans un cache hors dépôt).
3. `.godot` du conteneur séparé de celui de l'hôte (copie incrémentale), user:// de l'hôte monté
   (sorties au même endroit), caches de shaders séparés.
4. Essai sur un `*_shot.gd` réel du jeu, mesure du temps, comparaison visuelle.
5. ADR 0219, CLAUDE.md (règle), mémoire.

## État
- Étape 1 en cours.
