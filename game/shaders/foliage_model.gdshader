// Arbres proches de la carte en maillages décimés des modèles générés (lot DN-FORET, ADR 0216) :
// même corps que `foliage.gdshader` (vent, éclaircissement, teinte saisonnière par essence) avec
// FOLIAGE_MODEL : albédo = texture du glb (uniforme `model_texture`), un matériau par essence.
// Maillage : `DnTreeModels` (hauteur 1, pied en 0, UV2 = UV du glb, COLOR.a = poids du vent).
shader_type spatial;
render_mode diffuse_lambert_wrap, specular_schlick_ggx, cull_disabled;

#define FOLIAGE_MODEL
#include "res://shaders/foliage.gdshaderinc"
