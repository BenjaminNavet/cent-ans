# TW wardiag — guerres déclarées après m2b

Sonde : `campaign_probe --turns 120 --seeds 1..6` (guerres déclarées). Base `tw/int3`.

| Variante | s1 | s2 | s3 | s4 | s5 | s6 | moyenne |
|---|---|---|---|---|---|---|---|
| int3 (avant lot) | 238 | 247 | 194 | 207 | 258 | 245 | 231,5 |
| sans croisade papale | 238 | 247 | 194 | 207 | 229 | 245 | 226,7 |
| sans prétention par mariage | 158 | 216 | 192 | 183 | 251 | 199 | 199,8 |
| sans parjure -> excommunication | 238 | 247 | 194 | 207 | 256 | 245 | 231,2 |
| sans croisade ni mariage ni parjure (≈ avant m2b) | 158 | 216 | 192 | 183 | 222 | 213 | 197,3 |
| prétention de mariage plafonnée + expirante seulement | 238 | 231 | 194 | 212 | 258 | 243 | 229,3 |
| **correctif final (prétention « dynastique »)** | 205 | 209 | 188 | 191 | 266 | 192 | **208,5** |

Cause : la prétention par mariage (+17 %). Croisade ~+2 % (graine 5 seule), parjure ~0, interdit/conversion sans effet.
Mécanisme : une prétention de mariage faisait de la faction un « prétendant » (marge de front x2, lassitude +20,
prétention principale ignorant la parenté), donc une guerre contre ses beaux-parents, avec priorité 1000.
Correctif : `Claim.dynastic` (serde default) ; les prétentions dynastiques restent un casus belli mais ne comptent plus
pour `pretender`, `main_claim`, `weariness_to_declare` ni la négociation ; durée 40 tours et 2 prétentions simultanées
(`data/rules/religion.json` `dynastic_claim`). Résultat : +5,7 % sur l'équivalent d'avant m2b (±10 %).
Le plafond/la durée seuls ne changeaient presque rien.
