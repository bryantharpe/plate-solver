// The page's own files, read from disk. The test runner hands CSS imports over empty, and
// jsdom's URL is not one Node's file system accepts, so paths are built from strings.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const web = join(dirname(fileURLToPath(import.meta.url)), "..", "..");

/** A file of the page, by its path from `crates/finder/web`. */
export const readWebFile = (path: string) => readFileSync(join(web, path), "utf8");
