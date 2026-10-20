# TS — Style Total War (carte de campagne)

Démarré le 20/10. Source : 3 captures de Warhammer III (carte de campagne) comparées à 5 vues de Cent Ans
(Alpes printemps/hiver d=450, Grésivaudan hiver d=60, forêt d'Orléans d=250/d=70). ADR 0710.
Mandat : traiter tous les lots ci-dessous ; 20 agents Sonnet + 5 Opus ; fusion dans main par la session
principale ; pas de push.

## Constats (nos vues)
- Neige : aplat gris-blanc uniforme, y compris sur les pentes raides (montagnes « meringue ») ; en plaine les
  forêts d'hiver deviennent un bruit de points noirs ; de près, sol lisse bleu-gris, tramage visible sur les
  arbres, gros blocs gris « chou-fleur », flocons carrés.
- Forêts : de près, masse dense (bon) mais olive clair, une seule essence ronde, hauteur égale, pas d'ombre au
  sol, lisière tramée ; en vue moyenne, taches beiges plates (« lichen ») illisibles.
- Atmosphère : relief clair sans contraste roche/neige ; pas de brume de vallée ; plaine vert saturé uniforme.

## Lots
| Lot | Agent | Périmètre | État |
|---|---|---|---|
| T1 sol | Opus | `terrain/terrain.wesl` + données saison : neige teintée, masque altitude × pente (normale exagérée) × bruit en plaques, frange de boue, routes boueuses sous la neige, roche plus sombre en altitude, contraste des versants, sol assombri sous les forêts, mélanges de sol en taches | fusionné (0f92314e9) ; à régler : plaques d hiver trop régulières (`snow.patch_frequency`) |
| T2 forêts | Opus | `vegetation/` : canopée plus sombre, mélange de conifères, conifères givrés selon le masque de neige, lisière nette + arbres isolés devant, arbres plus gros en vue moyenne/lointaine (fin des taches beiges), limite des arbres en altitude, tramage et blocs gris de près, points noirs d'hiver | fusionné (d9e37ef7f) ; reste : arbres épars en vue moyenne → T8, rochers gris + ombres noires sur neige → T9 |
| T3 atmosphère | Opus | `weather/` + `terrain/post.*` : brume de vallée selon l'altitude, perspective atmosphérique bleutée, brume plus froide/dense en région froide, flocons doux, étalonnage désaturé avec accents chauds conservés | fusionné ; correctif T3b en cours (plaine basse noyée de brume laiteuse, vue Paris d=200) |
| T4 villes | Sonnet | `towns/` : halo chaud au sol + fenêtres, fumées de cheminées visibles en vue moyenne | fusionné (non jugé : brume) |
| T5 armées | Sonnet | `armies/` : trajet = ligne lumineuse animée posée sur le relief, anneau de zone de contrôle au sol, grande bannière héraldique derrière chaque armée | fusionné ; trajet/anneaux pas encore vus à l écran |
| T6 plaques & fil | Sonnet | `markers/` + `hud/` : cartouche de ville coloré par statut + pastilles ; fil d'événements repliable par catégorie | à faire |
| T7 sous-bois | Sonnet | `countryside/` : ceinture de sous-bois en lisière (herbes hautes, buissons, bois mort), touffes pâles en neige/toundra, touffes 3D au zoom proche | à faire |
| T8 bosquets | Sonnet | `vegetation/` (après T2) : bosquets rendus, clairières avec campement/scierie | à faire |

Vagues (disque : 3 worktrees à la fois, `target` supprimé après fusion) : T1 T2 T3 → T4 T5 T6 → T7 T8.
Worktrees `../gp-ts-<lot>`, branches `ts/<lot>`.

## Prochaine étape
Vagues 1-2 en cours ; T8 (bosquets + lecture des masses en vue moyenne) et T9 lancés.
| T9 rochers & ombres | Sonnet | `countryside/rocks.rs` + ombres de `vegetation/` : blocs gris « chou-fleur » plafonnés/teintés/enneigés, ombres d'arbres douces et bleutées sur la neige | en cours |
| T10 neige d hiver | Sonnet | plaques de neige en plaine trop régulières (« camouflage ») | en cours |
