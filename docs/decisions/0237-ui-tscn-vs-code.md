# 0237 — Interface : scènes .tscn ou construction en code

## Contexte
Une vingtaine de `.tscn` d'interface existent. Une quinzaine ne contiennent que le nœud racine et son script (6 à 13 lignes) : toute l'interface est construite en code dans `_ready`. Les autres (`province_panel`, `character_sheet`, `faction_panel`, `tech_panel`, `court_panel`, `save_load_dialog`, `campaign_map`, `battle`, `army_marker`) portent une hiérarchie de nœuds éditée dans l'éditeur.

## Décision
- L'interface se construit en code (`UiBuild`, `HudStyle`, `UiType`) : thème, tailles et textes viennent de `data/ui/` et du kit, ce qu'un `.tscn` ne sait pas exprimer.
- Un `.tscn` d'interface n'existe que s'il porte une hiérarchie ou des ressources réellement éditées (au-delà du nœud racine + script). Les coquilles « racine + script » sont des surcouches inutiles : on instancie la classe (`MaClasse.new()`).
- Un composant nouveau a un `class_name` et se crée par `.new()`, sans `.tscn`.
- Les coquilles existantes sont retirées quand on touche leur appelant. Appliqué par le lot SC UI11 aux cas triviaux : `settings_menu.tscn` (déjà non référencé), `credits_screen.tscn`, `custom_battle_screen.tscn`. Restent à migrer : `loading_screen`, `pause_menu`, `season_report`, `pre_battle_dialog`, `army_strip`, `end_turn_cluster`, `general_seal`, `news_letters`, `chronicle_window`, `encyclopedia`, `tutorial` (chargés par chemin dans `flow_controller` et des tests).

## Conséquences
- Moins de fichiers à garder synchrones avec les scripts ; un seul endroit où lire la structure d'un panneau.
- Le nom du nœud racine devient celui du type Godot par défaut ; aucun code ni test ne le recherchait pour les trois scènes retirées.
- Les gros `.tscn` édités à la main restent tels quels.
