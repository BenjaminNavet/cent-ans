# VIS-R1 : ciel, lumière, air de la carte de campagne

Spec : `docs/audits/2026-10-29-visuel-campagne.md` lot R1 (section g). Worktree agent-aa544b4a9560eb05d.
PAUSE demandée par le joueur : rien n'a été compilé ni mesuré ; tout ce qui suit est écrit mais NON VÉRIFIÉ.

## État : TERMINÉ (reste la fusion)
Tout est compilé, clippy et tests verts (client 1514 tests dont shader_check, core workspace, pytest schémas).
Repli retenu : dôme dégradé (Atmosphere exige Hdr, non mesuré). ADR : `docs/decisions/XXXX-vis-r1-ciel-et-lumiere-de-la-carte.md` (numéro à la fusion).
Mesures (clair, 1280x800, scripts scratchpad visr1/) :
- crit.1 camp_0060 : teinte 100, sat 0,22, val 0,47 (cibles 82-106 / >=0,22 / 0,38-0,55). d 25 : sat 0,21 (limite).
- crit.2 bande haute B-R : +0,11 à d 25 et d 60 (>= 0,08).
- crit.3 brouillard Cotentin d 200 : luminance mer/terre = 1,07 (±10 %), mesure sur bandes proches de la côte.
- crit.4 Alpes d 400 : 16 % du cadre > 0,75 (< 35 %).
- crit.5 banc pan d 30, A/B même binaire (A = nouveau, B = ciel coupé + pitch 30), machine chargée (load ~23) : p95 A 17,8 / 15,4, B 36,2 (bruit) / 17,9 ; p50 A 9,3 / 9,6, B 11,5 / 8,7. Écart indiscernable du bruit.
Captures : scratchpad .../visr1/out/ p60.png, p25.png, alps400.png, fog200.png, fogsea.png (5 prises, 2 regardées).
