// Variante d'hiver du feuillage de la carte (lot V4) : même corps que `foliage.gdshader`, houppiers
// des feuillus ajourés (discard). vegetation.gd ne l'emploie qu'en hiver : un discard fait perdre
// le test de profondeur anticipé, donc plus de surdessin dans les forêts.
shader_type spatial;
render_mode diffuse_lambert_wrap, specular_schlick_ggx;

#define FOLIAGE_WINTER_CUTOUT
#include "res://shaders/foliage.gdshaderinc"
