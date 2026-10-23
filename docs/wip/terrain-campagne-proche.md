# Terrain de campagne en vue rapprochée (lot TERRAIN)

Mandat 23/10 : constats 1, 7, 11, 16 de `out/juge/iter-02-art.md` (sol gris-vert sans matière, forêts en points/boules, hameaux cubes sombres, routes en fils droits). Contrainte dure : aucune régression fps (camp60, ADR 0740-0747, limite de 16 échantillonneurs Metal).
Branche `worktree-agent-affced78d9fe354dc` (base main c6aa7812e+). ADR prévu : 0758.

## Constat de départ
Le sol a déjà tout le mécanisme (45 couches régionales, parcelles/lanières `ground_detail.wesl` + `data/fx/ground_detail.json`, canopée `data/art/campaign_forest_look.json`, cache de sol par nœud ADR 0742). Le mal vient surtout du réglage (contraste, teintes) : le travail est donc d'abord de données + retouches ciblées de shader, sans nouveau sampler.

## État
- [ ] mesure de départ (bench `ab.sh`, d30/d60)
- [ ] sol : contraste et teintes par biome / saison
- [ ] parcellaire
- [ ] canopée
- [ ] hameaux
- [ ] routes
- [ ] ADR 0758

## Prochaine étape
Build release (froid), captures de départ (budget 10, 0 utilisée).
