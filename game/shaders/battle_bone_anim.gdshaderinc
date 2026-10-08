// Animation par texture d'os, commune au soldat skinné et au drapeau porté : texture des matrices
// d'os (`bone_tex`), table des clips du rig et lecture d'un clip (lignes encadrant l'instant).

uniform sampler2D bone_tex : filter_nearest, repeat_disable;
uniform float bone_fps = 24.0;
// Table des clips du rig : (ligne de départ, images, boucle 0/1, 0).
uniform vec4 clips[96];

int pick(ivec4 s, int i) {
	return i == 0 ? s.x : (i == 1 ? s.y : (i == 2 ? s.z : s.w));
}

float pickf(vec4 s, int i) {
	return i == 0 ? s.x : (i == 1 ? s.y : (i == 2 ? s.z : s.w));
}

// AN1b : jeu de clips jusqu'à 8 (emplacement i : composante i % 4, octet i / 4).
int pick_clip(ivec4 s, int i) {
	return (pick(s, i & 3) >> ((i >> 2) * 8)) & 255;
}

// Lignes de texture encadrant l'instant `t` du clip, et fraction entre les deux.
void clip_rows(int clip, float t, out int r0, out int r1, out float f) {
	vec4 c = clips[clip];
	float frames = max(c.y, 1.0);
	float x = max(t, 0.0) * bone_fps;
	float f0;
	float f1;
	if (c.z > 0.5) {
		x = mod(x, frames);
		f0 = floor(x);
		f1 = mod(f0 + 1.0, frames);
	} else {
		x = min(x, frames - 1.0);
		f0 = floor(x);
		f1 = min(f0 + 1.0, frames - 1.0);
	}
	f = x - f0;
	r0 = int(c.x + f0);
	r1 = int(c.x + f1);
}
