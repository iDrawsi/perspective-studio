# Format du modèle (`perspective-studio-modele`, version 1)

Fichier JSON lu par `web/modele.js` (`lireModele`). Tout est borné ; ce qui est inconnu est ignoré. Repère : **x vers la droite, y vers le bas, z vers le spectateur**. Unités libres (le cadrage est automatique). Rotations en degrés, appliquées autour du centre de la pièce.

```json
{
  "format": "perspective-studio-modele", "version": 1,
  "nom": "Mon objet",
  "fond": "#cfc7b8",
  "camera": { "az": 42, "el": 18, "focale": 50, "zoom": 1 },
  "lumiere": { "dir": [0.35, -1, 0.5], "ambiant": 0.5 },
  "style": { "mode": "contours", "fond": "#0d0e10", "trait": "#c8cbd0", "accent": "#ff6363", "ep": 1.3, "teinte": true },
  "pieces": [ ... ],
  "commandes": [ ... ]
}
```
- `camera` : `az` -180..180, `el` -89..89, `focale` 14..300 (mm équivalent 24×36), `zoom` 0,3..5.
- `style.mode` : `contours` (traits seuls, fond uni) ou `couleurs` (faces ombrées, fond = `fond`). `ep` 0,3..4.
- Couleurs : `#rrggbb` uniquement. Limites : 80 pièces, 60 décors par pièce, 12 commandes, 8 cibles par commande, 64 points par contour, 120 caractères par texte.

## Pièces
Champs communs : `cle` (identifiant, nécessaire pour être piloté par une commande), `nom`, `genre`, `pos` [x, y, z], `rot` [rx, ry, rz], `couleur`, `aretes`, `decors`.
- `genre: "boite"` : `p` = `w` largeur, `h` hauteur, `d` profondeur, `r` rayon des coins, `bevel` chanfrein, `seg` segments des coins. Faces (`cara` des décors) : 0 devant, 1 derrière, 2 droite, 3 gauche, 4 dessous, 5 dessus.
- `genre: "cylindre"` : `p` = `r` rayon, `l` longueur, `axe` (0 = x, 1 = y, 2 = z), `bevel`, `seg`. Faces : 0 et 1 pour les deux extrémités.
- `genre: "prisme"` : `p` = `prof` profondeur (selon z) ; contour **convexe** dans `pts` : [[x, y], ...]. Faces : 0 et 1 pour les extrémités.
- `aretes` : 0 contours et arêtes vives (défaut), 1 toutes les arêtes, 2 contours seulement.
- Deux pièces peuvent se traverser (par exemple un bouton enfoncé dans un boîtier) : l'ordre d'affichage reste exact. Évitez les faces exactement dans le même plan.

## Décors (à plat sur une face)
`genre`, `cara` (face), `x`, `y` (position sur la face, depuis son centre), `rot`, `p`, `fill`, `trait`, `ep` (épaisseur du trait), et selon le genre :
- `rect` : `p` = `w`, `h`, `r` (rayon des coins).
- `cercle` : `p` = `r`.
- `poly` : `pts` [[x, y], ...] (relatifs à x, y) ; `p.ferme` 1 = fermé.
- `texte` : `texte`, `p` = `taille`, `graisse` 100..900, `ancre` (0 début, 1 milieu, 2 fin), `police` (0 sans empattement, 1 monospace).
- `grille` : grille de trous ronds, `p` = `r` rayon, `pas`, `colonnes`, `lignes` (≤ 40).
`fill` et `trait` valent `null` pour « aucun ». En mode contours, les décors sont tracés en traits, sans remplissage.

## Commandes (pièces mobiles)
```json
{ "id": "bouton", "nom": "Bouton", "unite": "°", "min": -150, "max": 150, "defaut": 0, "pas": 1,
  "cibles": [ { "cle": "bouton", "cible": "rot", "comp": 2, "k": 1 } ],
  "demo": { "periode": 6, "phase": 0 } }
```
La valeur de la commande, multipliée par `k`, s'ajoute à la composante `comp` (0 = x, 1 = y, 2 = z) de la position (`cible: "pos"`) ou de la rotation (`cible: "rot"`) de la pièce `cle`. Plusieurs cibles peuvent partager une commande. La démonstration fait osciller la valeur entre `min` et `max` (`periode` en secondes, `phase` entre 0 et 1).

Une rotation se fait autour du **centre** de la pièce. Pour faire pivoter une pièce autour d'un autre point (une antenne autour de sa base), ajoutez des cibles de position qui compensent le décalage du centre : voir `web/modeles/television.json`.
