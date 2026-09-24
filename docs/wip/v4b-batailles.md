# Lot V4b — Finitions des batailles semi-réalistes (branche `visual-v4b`)

## État : terminé (non fusionné)
- [x] Banc d'essai : médiane des durées d'image, primitives et appels de dessin en plus de la moyenne
  (Metal ne fournit pas le temps GPU au script)
- [x] Arbres par tuiles de 160 m (écartés hors champ et hors ombres) + maillage allégé au-delà de 260 m,
  sans ombre ; buissons masqués au-delà de 520 m
- [x] Figurines : maillage allégé (≈ 50 % des triangles) au-delà de 75 m, ombre portée par le maillage
  allégé, plus d'ombre au-delà de 190 m ; cadavres en maillage allégé
- [x] Herbe : grilles allégées (0,5 m / 40 m ; 1,1 m / 95 m), touffes absentes rejetées avant les
  lectures de textures, maillage indexé
- [x] Livrées variées : 40 % de la troupe / 70 % des nobles en livrée, les autres en vêtement de teinte
  naturelle (8 teintes) avec croix de livrée sur la poitrine ; livrée désaturée et assombrie, écus
  désaturés ; teint, acier, taille (±7 %) et carrure (±5 %) variés par soldat
- [x] Chevaux redessinés (poitrail/ventre/croupe, encolure effilée, tête allongée, oreilles, jambes fines
  avec jarret, boulets, queue) ; caparaçon ajusté, tombant à mi-jambe
- [x] Rivière : reflet de Fresnel du ciel (couleur du préréglage météo), hauts-fonds plus clairs aux bords,
  berges de galets élargies, berges humides moins sombres
- [x] Sol vu de haut : taches de prairie (sèche, grasse, roussâtre) à deux échelles, reprises par l'herbe
- [x] Captures `docs/img/visuel/v4b_*.png`

## Mesures (Apple M4 Pro, `--disable-vsync`, 600 images ; machine partagée, bruit ±30 % sur la moyenne)
La médiane plafonne à 120 i/s (cadence de l'écran).

| Config | V4 (avant) | V4b |
|---|---|---|
| 1160 soldats | 74 i/s moy. ; ici 42–62 moy. / 89 méd. ; 9 M primitives | 87–101 moy. / 120 méd. ; 1,1 M primitives |
| 4800 soldats | 54 i/s moy. ; ici 49–51 moy. / 51 méd. ; 9,2 M primitives | 73–90 moy. / 73–110 méd. ; 1,6 M primitives |
| Siège (972 soldats) | 71 i/s moy. | 95 moy. / 120 méd. |

Ablations (4800 soldats, avant V4b) : arbres = 5,9 M primitives (dont 3,7 M d'ombres : un seul
MultiMesh par espèce, jamais écarté), figurines = 2,3 M, herbe = 0,5 M ; ombres du soleil ≈ +20 i/s.

## Points ouverts
- Siège : 1 170 appels de dessin (maisons et murailles en nœuds séparés) ; à regrouper en MultiMesh.
- La démo autonome actuelle (simulation modifiée par d'autres lots) ne va plus au contact en 300 s : les
  captures `--closeup` cadrent mal ; les captures V4b utilisent `--camera=`.
- L'armée française se déploie dans la rivière dans la démo (côté simulation, hors lot).
