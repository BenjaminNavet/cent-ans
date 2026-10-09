// Variante d'hiver de `foliage_model.gdshader` : houppiers des feuillus ajourés (comme
// `foliage_winter.gdshader`).
shader_type spatial;
render_mode diffuse_lambert_wrap, specular_schlick_ggx, cull_disabled;

#define FOLIAGE_MODEL
#define FOLIAGE_WINTER_CUTOUT
#include "res://shaders/foliage.gdshaderinc"
