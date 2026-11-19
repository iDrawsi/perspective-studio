# Perspective Studio

Outil local pour fabriquer des illustrations vectorielles animées, en traits seuls (contours clairs sur fond sombre, une teinte pour ce qui bouge), à partir d'un modèle 3D simple. Caméra, focale en mm et zoom réglables ; pièces mobiles pilotées par des curseurs ou par une démonstration en boucle ; export en page HTML autonome, en SVG ou en JSON.

## Usage premier : de l'image à l'illustration, avec une IA

On donne à un assistant IA (testé avec Claude Code) des **images de référence**, une **description** et le **comportement** voulu. L'IA pose ses questions (rendu, vue, zoom, animation, sortie), puis **écrit un modèle** (fichier JSON) que le moteur dessine, anime et exporte. Le moteur ne reconstruit pas l'objet depuis la photo : c'est l'IA qui le décrit avec des formes simples, donc le résultat est stylisé et approximatif. Voir `docs/WORKFLOW-IA.md` (le briefing à suivre) et `docs/FORMAT-MODELE.md` (le format que l'IA écrit). Le fichier `CLAUDE.md` demande à Claude Code de suivre ce workflow.

Le moteur est écrit en Rust et compilé en WebAssembly. **Aucune dépendance externe** (ni crate, ni bibliothèque JavaScript), aucun réseau à l'exécution.

## Lancer

1. Compiler le moteur (une fois, et après chaque changement du code Rust) : `./build.sh`. Il demande Rust avec la cible `wasm32-unknown-unknown`. Le fichier `web/engine.wasm` n'est pas dans git.
2. Démarrer le serveur local : `python3 serve.py` (écoute uniquement sur `127.0.0.1`, sans cache), puis ouvrir `http://127.0.0.1:8765/index.html`. Arrêt : Ctrl+C.

## Ce que l'on peut faire

- **Page principale** (`web/index.html`) : charge un modèle (`web/modeles/television.json`, une télé cathodique originale), le montre avec :
  - des **commandes** : curseurs pour les pièces mobiles (boutons qui tournent, antennes qui s'écartent, barre qui balaie l'écran) et une **démonstration** qui les fait bouger seules ;
  - la **caméra** : rotation horizontale et verticale, focale en mm, zoom ;
  - le **style** : contours ou couleurs d'origine, couleurs du fond, des contours et de la teinte, épaisseur du trait, teinte des pièces animées.
- **Exports** : page HTML animée (un seul fichier, moteur intégré, fonctionne hors ligne), image SVG figée, modèle JSON (avec le style et la caméra choisis).
- **Éditeur de modèle** (`web/editeur.html`) : assembler des boîtes, cylindres et prismes, dessiner à plat sur leurs faces (formes, textes, grilles de trous), annuler/rétablir, enregistrer en JSON. Voir `docs/EDITEUR.md`.

## Le modèle (JSON)

Un fichier `perspective-studio-modele` contient : `pieces` (genre `boite`, `cylindre` ou `prisme`, position, rotation en degrés, dimensions, couleur, `decors` à plat sur les faces), `commandes` (chacune déplace ou tourne une ou plusieurs pièces repérées par leur `cle`, avec les réglages de la démonstration), `style`, `camera`, `lumiere` et `fond`. Le repère est x vers la droite, y vers le bas, z vers le spectateur. Tout est borné et validé à la lecture (`web/modele.js`).

## Contenu

- `crates/core` : le moteur (caméra, pièces, arbre BSP pour un ordre d'affichage exact, rendu SVG) et ses tests (`cargo test`).
- `web/` : interface (`index.html`, `editeur.html`), lecteur de modèle (`modele.js`), liaison WebAssembly (`glue.js`), exports (`export.js`).
- `docs/WORKFLOW-IA.md` : briefing et vérifications pour une IA. `docs/FORMAT-MODELE.md` : format du modèle. `docs/EDITEUR.md` : éditeur et ordre d'affichage. `docs/SECURITE.md` : choix de sécurité.

## Limites

Pas de transparence ni de dégradés, pas de formes courbes libres (cylindres et prismes convexes seulement), pas de déclenchement au scroll ou au clic dans la page exportée. Testé dans Chromium ; Safari, Firefox et téléphones réels ne sont pas vérifiés.

## Licences

- **Code** (`crates/`, `web/`, `serve.py`, `build.sh`) : MIT, voir `LICENSE`.
- **Modèles** (`web/modeles/`) **et documentation** : CC BY 4.0, voir `LICENSE-CONTENU.md`.

Les illustrations et les modèles de démonstration sont originaux et ne reproduisent aucun produit ni aucune marque. Si vous créez un modèle d'après un objet réel, évitez de reprendre ses tracés, son nom ou son logo.
