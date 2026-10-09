# 0285 — Carte : rotation souris, suivi d'armée, pings de minicarte, panneau Commerce

Statut : accepté (lot WH `mapb2`).

## Contexte
Total War: Warhammer III permet de pivoter la vue à la souris, de suivre une armée, signale les
menaces par un ping sur la minicarte, y accepte les ordres de marche et liste les routes
commerciales. Le jeu avait déjà le glisser au bouton du milieu, `get_trade_routes` et un
infobulle de routes dans le panneau de faction, mais ni rotation souris, ni suivi, ni ping.

## Décision
- Caméra (`campaign_camera.gd`) : bouton du milieu + Alt ou Maj = rotation (`target_yaw`), sans
  Alt/Maj = glisser comme avant. `start_follow(Callable)` verrouille le point visé sur une cible
  mobile ; le clavier, les bords d'écran, un glisser de plus de `follow_release_px` ou un
  recentrage (`look_at_point`, donc la minicarte) l'arrêtent ; la rotation ne l'arrête pas.
  Touche `Y` (`army_follow`, bascule). Valeurs dans `data/ui/camera_feel.json`
  (`rotate_mouse_deg_per_px`, `follow_release_px`, `minimap_ping_s`, `minimap_ping_radius_px`).
- Minicarte : `MinimapController.on_alerts` compare les alertes `siege` / `enemy_army` à celles
  déjà vues (première série silencieuse) et pingue les nouvelles ; une armée n'est pingée que si
  `is_army_visible`. Clic droit = signal `ordered` -> `ArmyMovementController.order_to_map_point`
  (même chemin que le clic droit de la carte : mer, refus et toasts compris).
- Commerce : getter de pont `get_trade_overview(faction)` (`trade.rs`, vue seulement :
  `TradeRouteView::value_for` / `touches`, `faction_trade_income` en est la somme). `TradePanel`
  (bouton du panneau de faction, touche `I`) : tri par valeur, filtre « Mes routes », mode
  terre/mer, raison de coupure.

## Conséquences
Aucune règle nouvelle dans Godot ; test Rust `per_route_shares_sum_to_faction_trade_income`.
Touches ajoutées : `Y`, `I` (réaffectables, listées dans la fiche des raccourcis).
Un suivi actif est relâché si la souris reste au bord d'écran avec le défilement par les bords.
