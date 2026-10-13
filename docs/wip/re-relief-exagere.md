# RE — carte de campagne à relief exagéré (façon Total War Rome II / Warhammer)

Mandat (13/10 soir) : nuit en autonomie, sans question, **0 $**. Réponses du joueur : référence
Rome II + Warhammer ; montagnes infranchissables hors des cols : oui ; batailles : relief réel.
Meshy : seulement une suggestion dans le rapport final, aucune dépense.
Rapport initial : `docs/research/carte-relief-exagere.md` ; recherches web :
`docs/research/relief-exagere/0{1,2,3}-*.md`. Worktree `../game_project-re`, branche `re-relief`.

## Constat technique (13/10)
- Le jeu a déjà une hauteur affichée stylisée à l'exécution (ZG8/SZ1, `campaign_relief.gdshaderinc`,
  doubles `ReliefState.display_height_fields` et `vegetation::display_height` en Rust) :
  `y = s·(h − K·max(h − base, 0) + g·(1 − K)·max(h − fond, 0))`, `K = c·k`.
- Réglage actuel (`relief_exaggeration.tres`) : `mountain_ratio` 0,3, `mountain_squash_far` 1 →
  **les montagnes sont écrasées à 30 % au-dessus de 350 m** : les Alpes s'affichent ≈ ×1,6 par
  rapport aux plaines, pas ×4,3. Le rapport initial sous-estimait le problème.
- Idée retenue : un `k` **négatif** amplifie (`1 − K > 1`) ; un `k` positif en plaine aplatit le
  micro-relief vers le fond. Pas de pyramide à recuire, les batailles restent réelles (elles lisent
  l'altitude vraie), une seule formule pour GPU / GDScript / Rust.

## Lots
- R1a : champ de style `k ∈ [−(M−1), k_plaine]` (profil), bornes des boîtes (relevé max), tests.
- R0 : planche `re_relief_shot.gd` (Alpes, Pyrénées, Massif central, Beauce) × niveaux M.
- R2 : `navgrid` : masse montagneuse infranchissable hors cols nommés (même amplitude régionale,
  en Python) ; cols historiques sourcés ; seed bots passe longue.
- R3 : roche/neige/lumière sur les massifs.
- ADR : renverse « le relief reste vrai » (ADR 0158) et l'écrasement SZ1.

## État
- 13/10 : worktree prêt, recherche TW rendue (peu de faits publiés), deux recherches en cours.

## Prochaine étape
R1a.
