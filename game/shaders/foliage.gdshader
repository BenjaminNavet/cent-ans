// Arbres de la carte de campagne (lot V3, `vegetation.gd`), instanciés par MultiMesh.
// COLOR (sommet) : RVB = albédo de base, A = poids du vent (0 au pied, 1 en haut).
// INSTANCE_CUSTOM : RVB = teinte multiplicative de l'instance, A = graine aléatoire [0, 1].
// UV (lot V4) : x = essence (0 chêne, 1 hêtre, 2 conifère, 3 haie), y = 1 houppier / 0 tronc ;
// teinte saisonnière par essence, pondérée par `campaign_season` (lot CV1).
// Disparition progressive avec la distance à `view_origin` (fournie par le script, identique
// dans la passe d'ombre) : chaque arbre rétrécit puis disparaît à un seuil qui dépend de sa
// graine, ce qui éclaircit la forêt au lieu de la couper net.
// Corps commun : `foliage.gdshaderinc` ; variante d'hiver ajourée : `foliage_winter.gdshader`.
shader_type spatial;
render_mode diffuse_lambert_wrap, specular_schlick_ggx;

#include "res://shaders/foliage.gdshaderinc"
