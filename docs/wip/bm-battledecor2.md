# bm-battledecor2

Branche `bm/battledecor2` (lot reprise après coupure).

## Fait
- Arbres/buissons pâles : cause réelle = `Assets::get_mut` renvoie un `AssetMut` qui ne signale `Modified` qu'au déréférencement mutable ; le `let _ = get_mut()` du premier essai ne faisait rien, donc les matériaux gardaient la texture blanche 1x1 (cartes pleines). `prepare_tree_textures` rafraîchit maintenant les matériaux (`TreeMaterials`) pendant 4 images après le dépôt des textures. Les mips des feuilles gardent aussi la couverture (couleur pondérée par alpha, alpha remis à l'échelle par niveau) au lieu de `1 + 0,35 l` qui rendait toute carte pleine de loin. Transmission diffuse hors de cause (testé).
- Avignon : tours dessinées plus fines (62 % du rayon sim, min 3,2 m), 3 m plus hautes, cône 2,1 r (`siege/works.rs`). L'empreinte sim ne change pas.
- Ouverture de Crécy : bandeau de déploiement à 58 px, sous-titres à 136 px, 50 % de large.

## Reste
- Pas de modèle de tour/rempart dans game/assets (seuls `wall_run` 1,4 m et maquettes de ville) : tours toujours générées par code ; à remplacer si un modèle arrive.
