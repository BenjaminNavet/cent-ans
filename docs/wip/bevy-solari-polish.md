# solari-polish (23/10)

Fait (battle sous Solari, `enabled:true` pour les essais, remis à `false`) :
- Taches noires du sol : le sol de substitution est abaissé sous le minimum des hauteurs voisines puis de `ground_sink_m`
  (défaut 2,0 m, réglable dans solari.json). Avec 0,4 m les taches restaient (5 % de pixels < 25 dans la moitié basse) ;
  avec 2 m elles disparaissent (0,3 %), niveau du rendu normal (0,9 %). Le sol visible est donc nettement plus haut que
  `field.height` par endroits (relief de détail).
- Ciel : la caméra de bataille porte déjà un `GeneratedEnvironmentMapLight` lu par Solari ; `AmbientLight` est ignoré, donc
  toute l'énergie du ciel passe par l'env map quand Solari est actif (`lighting.rs`, contribution 1,0).
- `restir: true` dans solari.json (essai visuellement plus propre, sans effet tant que `enabled:false`).
Mesures (captures 1280x800 ; la pose caméra de la mise en place varie d'un lancement à l'autre : comparaison indicative) :
rendu normal 0,9 % ; Solari avant 2,1 % (taches visibles) ; sink 0,4 + ciel 0,1-5 % selon pose ; sink 2 m : 0,3 %.
Reste : soldats hors TLAS (pas d'ombre Solari) ; grain/bruit sans débruiteur (taches sombres fines sur l'herbe) ;
feuillage/pierre/bâtiments visibles (force_forward, éclairés par le raster) mais non éclairés par Solari ; pas de mesure fps.
