import assert from "node:assert/strict";
import { mkdtempSync, mkdirSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { installedTree, uninstallScript } from "./uninstall-files.mjs";

test("uninstall deletes the bundle and leaves a file the installer did not copy", () => {
  const root = mkdtempSync(join(tmpdir(), "pitex-uninstall-"));
  const outside = mkdtempSync(join(tmpdir(), "pitex-uninstall-outside-"));
  writeFileSync(join(outside, "secret.txt"), "keep");
  writeFileSync(join(root, "a.txt"), "a");
  mkdirSync(join(root, "sub"));
  writeFileSync(join(root, "sub", "b.txt"), "b");
  symlinkSync(outside, join(root, "linked"), "dir");

  const { files } = installedTree(root);
  assert.deepEqual(files.sort(), ["a.txt", "linked", join("sub", "b.txt")].sort());
  assert.equal(files.some((file) => file.includes("secret")), false);

  const script = uninstallScript(root);
  assert.match(script, /Delete "\$INSTDIR\\a\.txt"/);
  assert.match(script, /Delete "\$INSTDIR\\sub\\b\.txt"/);
  assert.match(script, /Delete "\$INSTDIR\\linked"/);
  assert.match(script, /RMDir "\$INSTDIR\\sub"/);
  assert.match(script, /RMDir "\$INSTDIR"\n$/);
  assert.equal(script.includes("RMDir /r"), false);
  assert.equal(script.includes("secret"), false);
  const sub = script.indexOf('RMDir "$INSTDIR\\sub"');
  const top = script.indexOf('RMDir "$INSTDIR"');
  assert.ok(sub > 0 && top > sub);
});

test("a path that would break the installer script is refused", () => {
  const root = mkdtempSync(join(tmpdir(), "pitex-uninstall-bad-"));
  writeFileSync(join(root, "a$b.txt"), "no");
  assert.throws(() => uninstallScript(root), /not safe/);
});
