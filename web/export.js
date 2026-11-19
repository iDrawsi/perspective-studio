// Exports : page HTML autonome et animée, image SVG, réglages JSON. Aucun réseau à l'ouverture des fichiers exportés.
// La page HTML embarque le moteur WebAssembly (base64) : le dessin reste exact en perspective.
const gabarit = (s) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;");

// Les fichiers sources sont relus à chaque export, sans cache : l'export doit correspondre au moteur actuel.
const frais = (url) => fetch(url, { cache: "no-cache" });

const base64 = (octets) => {
  let s = "";
  for (let i = 0; i < octets.length; i += 0x8000) s += String.fromCharCode(...octets.subarray(i, i + 0x8000));
  return btoa(s);
};
const sansExport = (code) => code.replace(/^export\s+(async\s+)?(function|const)/gm, "$1$2");

// ---------- Téléchargement ----------
export function telecharger(nom, texte, type) {
  const url = URL.createObjectURL(new Blob([texte], { type }));
  const a = document.createElement("a");
  a.href = url; a.download = nom;
  document.body.appendChild(a); a.click(); a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 5000);
}

// ---------- Illustration (modèle animé) ----------
/** SVG de la scène telle qu'elle est affichée : image figée, fond optionnel. */
export async function svgDeLaScene(scene, fond, css) {
  const svg = scene.innerHTML;
  if (!svg) throw new Error("Rien à exporter.");
  const rect = fond ? `<rect width="736" height="736" fill="${fond}"/>` : "";
  return `<?xml version="1.0" encoding="UTF-8"?>\n` + svg.replace(/ sel"/g, '"').replace(/^(<svg[^>]*>)/, `$1<style>${css}</style>${rect}`);
}

/** Page HTML autonome : moteur, lecteur et modèle sont intégrés ; la démonstration anime les commandes. */
export async function construireHtmlModele({ texteModele, vue, vitesse, demo, titre }) {
  const [glue, lecteur, wasm] = await Promise.all([
    frais("glue.js").then((r) => r.text()),
    frais("modele.js").then((r) => r.text()),
    frais("engine.wasm").then((r) => r.arrayBuffer()),
  ]);
  const modele = JSON.stringify(JSON.parse(texteModele)).replace(/</g, "\\u003c");   // jamais de balise dans le script
  return `<!doctype html>
<html lang="fr">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>${gabarit(titre)}</title>
<link rel="icon" href="data:,">
<style>
  html, body { height: 100%; margin: 0; background: #0b0c0e; }
  body { display: grid; place-items: center; overflow: hidden; }
  #scene { width: 100vw; height: 100vh; height: 100dvh; display: grid; place-items: center; container-type: size; }
  #scene svg { width: min(100cqw, 100cqh); height: min(100cqw, 100cqh); display: block; overflow: hidden; }
</style>
<style id="style-illustration"></style>
</head>
<body>
<div id="scene"></div>
<script type="module">
// Page générée par Perspective Studio. Le moteur (WebAssembly) est intégré : aucun réseau n'est nécessaire.
const MODELE = ${modele};
const VUE = ${JSON.stringify({ az: vue.az, el: vue.el, focale: vue.focale, dist: vue.dist })};
const WASM = "${base64(new Uint8Array(wasm))}";
${sansExport(glue)}
${sansExport(lecteur)}
const scene = document.getElementById("scene");
const moteur = await chargerMoteur(Uint8Array.from(atob(WASM), (c) => c.charCodeAt(0)).buffer);
const modele = lireModele(MODELE);
const fond = fondDe(modele, modele.style);
document.body.style.background = fond;
document.documentElement.style.background = fond;
document.getElementById("style-illustration").textContent = cssIllustration(modele, modele.style);
const lecteur = creerLecteur({ scene, moteur, modele, vue: () => VUE, vitesse: () => ${Number(vitesse) || 1} });
lecteur.rendre();
lecteur.surveiller(scene);
lecteur.demo(${demo ? "true" : "false"});
</script>
</body>
</html>
`;
}
