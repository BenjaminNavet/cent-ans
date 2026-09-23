# Lot F4 « Guerre de Cent Ans vivante » — état

Branche : `worktree-agent-a5777744f85f20632`. Sonde : `cargo run --release -p ai --example century_probe`
(5 graines × 464 tours, tableau de synthèse). Tests : `core/crates/ai/tests/`, `core/crates/sim-campaign/tests/f4_*.rs`.

## Points
1. [ ] Guerre de prétention France-Angleterre (≥ 55 % des tours, ≥ 3 phases).
2. [ ] Alliances historiques et appel aux armes.
3. [ ] Trésors dormants (≤ 8 saisons après 1350) et banqueroutes (< 1 / faction / décennie).
4. [ ] Mariages par l'IA.
5. [ ] Chevauchées et sièges (≥ 3 batailles FR/EN par décennie), survie des majeures en 1400.
6. [ ] Bug Portugal « la lignée s'éteint » (hiver 1337).
7. [ ] Docs : `m9-ai.md` § F4, `status.md` (équilibrage).

## Mesure initiale (avant F4)
| graine | guerre FR-EN | phases | batailles FR/EN / déc. | trésor max (saisons) | banqueroutes / fac. / déc. | survie 1400 |
|---|---|---|---|---|---|---|
| 1 | 61 % | 3 | 4,4 | 21 518 (Flandre) | 1,06 | Écosse détruite |
| 2 | 6 % | 1 | 1,4 | 28 (Suisse) | 0,91 | Écosse détruite |
| 3 | 6 % | 1 | 1,5 | 41 (Portugal) | 0,93 | Écosse détruite |
| 4 | 6 % | 1 | 1,4 | 25 (Navarre) | 1,51 | Écosse détruite |
| 5 | 19 % | 1 | 0,8 | 37 779 (Grenade) | 0,95 | Écosse détruite |

Alliances : Auld Alliance 2 % des tours, Angleterre-Flandre/Hainaut/Brabant 0 %, Gueldre 100 %.

## Diagnostic point 6
Alphonse IV (46 ans) commande l'armée portugaise, attaque la Castille à l'été 1337, perd et meurt
(5 % par défaite) ; son fils Pierre Ier n'existe pas dans `data/characters` → aucun héritier.

## Prochaine étape
Point 6 (données + mort des souverains au combat), puis guerre de prétention.
