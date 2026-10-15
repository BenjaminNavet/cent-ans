# bm-camera — état

Fait : `FocusCamera` (focus.rs), raccourcis Tab/Maj+Tab/Ctrl+Tab/Home/./Y, `ZoomTiers`/`ZoomView` (zoom.rs, pilote `WaterView`), feel « réduire les animations ».
Preuve : `focus_script.txt` + dumps `campaign_camera` / `campaign_zoom`.
Reste : les autres couches (hameaux, bordures, brouillard, vie) doivent lire `ZoomView` ; pas de toasts (« Aucune armée inactive ») ; Ctrl+Tab = provinces, pas colonies ; ouverture du panneau capitale/colonie et sélection de province (picker) non branchées.

settings3 : touches par KeyMap (`map_pan_*`, `map_rotate_*`, `map_toggle_edge_pan`, `campaign_next/prev_idle`, `campaign_next_settlement`, `campaign_capital`, `army_center`, `army_follow`) ; `edge_pan` et `camera_speed` des réglages appliqués au rig (`apply_camera_settings`). Le modificateur de rotation à la souris (Alt/Maj + glisser) reste en dur.
