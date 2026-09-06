import { createHash } from "node:crypto";
import { createReadStream, createWriteStream } from "node:fs";
import { readFile, rename, rm, stat } from "node:fs/promises";
import { Readable } from "node:stream";
import { pipeline } from "node:stream/promises";
import { fileURLToPath } from "node:url";

const directory = new URL("../src-tauri/resources/models/", import.meta.url);
const model = JSON.parse(
  await readFile(new URL("manifest.json", directory), "utf8"),
);
const destination = new URL(model.file, directory);
async function verify(path) {
  if ((await stat(path)).size !== model.size_bytes)
    throw new Error("Taille du modèle incorrecte");
  const hash = createHash("sha256");
  for await (const chunk of createReadStream(path)) hash.update(chunk);
  if (hash.digest("hex") !== model.sha256)
    throw new Error("Empreinte SHA-256 du modèle incorrecte");
}
try {
  await verify(destination);
  console.log(`${model.name} : modèle intégré vérifié.`);
} catch (error) {
  if (!process.argv.includes("--download")) {
    console.error(
      `Modèle intégré absent ou invalide : ${error.message}. Exécuter pnpm models:prepare avant de construire l’application.`,
    );
    process.exitCode = 1;
  } else {
    const temporary = new URL(`${model.file}.part`, directory);
    console.log(
      `Téléchargement de ${model.name} (${Math.ceil(model.size_bytes / 1e6)} Mo)…`,
    );
    try {
      const response = await fetch(model.url, {
        signal: AbortSignal.timeout(30 * 60 * 1000),
      });
      if (!response.ok || !response.body)
        throw new Error(`Téléchargement : HTTP ${response.status}`);
      await pipeline(
        Readable.fromWeb(response.body),
        createWriteStream(temporary),
      );
      await verify(temporary);
      // Destination is absent or corrupt; keep it until the new download is verified.
      await rm(destination, { force: true });
      await rename(temporary, destination);
      console.log(`Modèle intégré prêt : ${fileURLToPath(destination)}`);
    } finally {
      await rm(temporary, { force: true });
    }
  }
}
