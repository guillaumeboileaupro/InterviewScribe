import { createHash } from "node:crypto";
import { createReadStream, createWriteStream } from "node:fs";
import { readdir, readFile, rename, rm, stat } from "node:fs/promises";
import { Readable } from "node:stream";
import { pipeline } from "node:stream/promises";
import { fileURLToPath } from "node:url";

const directory = new URL("../src-tauri/resources/models/", import.meta.url);
const download = process.argv.includes("--download");

async function verify(path, model) {
  if ((await stat(path)).size !== model.size_bytes)
    throw new Error("Taille du modèle incorrecte");
  const hash = createHash("sha256");
  for await (const chunk of createReadStream(path)) hash.update(chunk);
  if (hash.digest("hex") !== model.sha256)
    throw new Error("Empreinte SHA-256 du modèle incorrecte");
}

async function prepare(manifestName) {
  const model = JSON.parse(
    await readFile(new URL(manifestName, directory), "utf8"),
  );
  const destination = new URL(model.file, directory);
  try {
    await verify(destination, model);
    console.log(`${model.name} : modèle intégré vérifié.`);
    return true;
  } catch (error) {
    if (!download) {
      console.error(
        `Modèle intégré absent ou invalide (${model.name}) : ${error.message}. Exécuter pnpm models:prepare avant de construire l’application.`,
      );
      return false;
    }
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
      await verify(temporary, model);
      // Destination is absent or corrupt; keep it until the new download is verified.
      await rm(destination, { force: true });
      await rename(temporary, destination);
      console.log(`Modèle intégré prêt : ${fileURLToPath(destination)}`);
      return true;
    } finally {
      await rm(temporary, { force: true });
    }
  }
}

const manifestNames = (await readdir(directory)).filter((name) =>
  name.endsWith("manifest.json"),
);

let ok = true;
for (const manifestName of manifestNames) {
  if (!(await prepare(manifestName))) ok = false;
}
if (!ok) process.exitCode = 1;
