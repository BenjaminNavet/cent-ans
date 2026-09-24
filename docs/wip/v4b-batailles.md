# Lot V4b — Finitions des batailles semi-réalistes (branche `visual-v4b`)

## État
- [x] Banc d'essai : médiane des durées d'image, primitives et appels de dessin (Metal ne donne pas le temps GPU)
- [x] Arbres par tuiles de 160 m (culling) + maillage allégé au-delà de 260 m, sans ombre
- [x] Figurines : maillage allégé (≈ 50 % des triangles) au-delà de 75 m, ombre portée par le maillage allégé, plus d'ombre au-delà de 190 m ; cadavres en maillage allégé
- [x] Herbe : grilles allégées (0,5 m / 40 m ; 1,1 m / 95 m), touffes absentes rejetées avant les lectures de textures, maillage indexé
- [x] Livrées variées : 40 % de la troupe / 70 % des nobles en livrée, les autres en vêtement de teinte naturelle (8 teintes) avec croix de livrée sur la poitrine ; livrée désaturée et assombrie, écus désaturés ; teint, acier, taille (±7 %) et carrure variés par soldat
- [x] Chevaux redessinés (poitrail/ventre/croupe, encolure effilée, tête allongée, oreilles, jambes fines avec jarret, boulets, queue) ; caparaçon ajusté à mi-jambe
- [x] Rivière : reflet de Fresnel du ciel (couleur du préréglage météo), hauts-fonds plus clairs aux bords, berges de galets élargies, berges humides moins sombres
- [x] Sol vu de haut : taches de prairie (sèche, grasse, roussâtre) à deux échelles, reprises par l'herbe
- [ ] Captures `docs/img/visuel/v4b_*.png`
- [ ] Retirer les interrupteurs d'expérience `BATTLE_EXP` (marqués `#EXP`)

## Mesures (Apple M4 Pro, `--disable-vsync`, 600 images ; machine partagée, bruit ±30 %)
| Config | Avant (V4) | Après LOD arbres/figurines/herbe |
|---|---|---|
| 1160 soldats | 74 i/s (V4), 42–62 i/s moy. / 89 méd. ici | 83 moy. / 90 méd., 1,08 M primitives |
| 4800 soldats | 54 i/s (V4), 49–51 méd. ici, 9,19 M primitives | 73 moy. / 73 méd., 1,57 M primitives |

Ablations (4800 soldats, avant) : arbres = 5,9 M primitives (dont 3,7 M d'ombres, un seul MultiMesh par
espèce donc jamais écarté), figurines = 2,3 M, herbe = 0,5 M ; ombres du soleil ≈ +20 i/s si coupées.

## Prochaine étape
Mesurer SSAO / ombres restants, puis livrées (shader `battle_soldier.gdshader`).
