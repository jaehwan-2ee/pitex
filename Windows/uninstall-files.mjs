// Files the NSIS installer copied. Uninstall deletes these and then
// removes a directory only when nothing else is left in it.
import { readdirSync } from "node:fs";
import { join, sep } from "node:path";

export function installedTree(root) {
  const files = [];
  const dirs = [];
  function walk(dir) {
    for (const entry of readdirSync(dir, { withFileTypes: true })) {
      const full = join(dir, entry.name);
      const relative = full.slice(root.length).replace(/^[/\\]/, "");
      if (entry.isSymbolicLink() || !entry.isDirectory()) {
        files.push(relative);
        continue;
      }
      walk(full);
      dirs.push(relative);
    }
  }
  walk(root);
  dirs.sort((a, b) => b.length - a.length);
  return { files, dirs };
}

export function uninstallScript(root) {
  const { files, dirs } = installedTree(root);
  const lines = [];
  for (const file of files) lines.push(`Delete "$INSTDIR\\${nsisPath(file)}"`);
  for (const dir of dirs) lines.push(`RMDir "$INSTDIR\\${nsisPath(dir)}"`);
  lines.push(`RMDir "$INSTDIR"`);
  return lines.join("\n") + "\n";
}

function nsisPath(relative) {
  const parts = relative.split(/[/\\]/);
  if (
    parts.some(
      (part) => part === "" || part === "." || part === ".." || /["$\r\n]/.test(part),
    )
  ) {
    throw new Error(`uninstall path is not safe: ${relative}`);
  }
  return parts.join("\\");
}

const invoked = process.argv[1]?.replaceAll("\\", "/").endsWith("/uninstall-files.mjs");
if (invoked) {
  const root = process.argv[2];
  if (!root) {
    console.error("usage: node uninstall-files.mjs <bundle-dir>");
    process.exit(2);
  }
  process.stdout.write(uninstallScript(root));
}
