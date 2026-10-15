# bm-camera — état

Fait : `FocusCamera` (focus.rs), raccourcis Tab/Maj+Tab/Ctrl+Tab/Home/./Y, `ZoomTiers`/`ZoomView` (zoom.rs, pilote `WaterView`), feel « réduire les animations ».
Preuve : `focus_script.txt` + dumps `campaign_camera` / `campaign_zoom`.
Reste : les autres couches (hameaux, bordures, brouillard, vie) doivent lire `ZoomView` ; pas de toasts (« Aucune armée inactive ») ; Ctrl+Tab = provinces, pas colonies ; ouverture du panneau capitale/colonie et sélection de province (picker) non branchées.
