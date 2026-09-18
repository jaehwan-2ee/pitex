#!/usr/bin/env node
import { readFile, readdir } from 'node:fs/promises';
import { basename, dirname, extname, join, relative, resolve, sep } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const TOOL_DIR = dirname(fileURLToPath(import.meta.url));
const DEFAULT_ROOT = resolve(TOOL_DIR, '..');
function add(list, code, path, message) { list.push({ code, path: path.replaceAll(sep, '/'), message }); }
async function filesBelow(path, suffix = '') {
  const output = [];
  async function visit(current) {
    let entries;
    try { entries = await readdir(current, { withFileTypes: true }); } catch (error) { if (error.code === 'ENOENT') return; throw error; }
    for (const entry of entries.sort((a, b) => a.name.localeCompare(b.name))) {
      const child = join(current, entry.name);
      if (entry.isDirectory()) await visit(child);
      else if (entry.isFile() && child.endsWith(suffix)) output.push(child);
    }
  }
  await visit(path); return output;
}
function section(source, name) {
  const match = source.match(new RegExp(`/\\* Begin ${name} section \\*/([\\s\\S]*?)/\\* End ${name} section \\*/`));
  if (!match) throw new Error(`missing ${name} section`);
  return match[1];
}
function listIds(body, key) {
  const match = body.match(new RegExp(`\\b${key}\\s*=\\s*\\(([\\s\\S]*?)\\);`));
  return match ? [...match[1].matchAll(/\b([A-Fa-f0-9]{24})\b/g)].map((item) => item[1]) : [];
}
function plistValues(source, key) {
  const escaped = key.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const after = source.match(new RegExp(`<key>\\s*${escaped}\\s*</key>([\\s\\S]*?)(?=<key>|</dict>)`))?.[1] ?? '';
  return [...after.matchAll(/<string>\s*([^<]+?)\s*<\/string>/g)].map((match) => match[1].trim());
}
function settings(text) {
  const map = new Map();
  for (const line of text.split('\n')) {
    const clean = line.replace(/\/\/.*$/, '').trim();
    const match = clean.match(/^([A-Za-z][A-Za-z0-9_]*)\s*(?:\[[^\]]+\])?\s*=\s*([^;]+);?$/);
    if (match) map.set(match[1], match[2].trim().replace(/;$/, '').trim());
  }
  return map;
}
function validateBuildSettings(text, path, violations) {
  const all = settings(text);
  for (const match of text.matchAll(/\bMACOSX_DEPLOYMENT_TARGET\s*=\s*([0-9]+(?:\.[0-9]+)?)/g)) {
    if (Number(match[1]) < 15) add(violations, 'DEPLOYMENT_TARGET', path, `deployment below macOS 15 is forbidden: ${match[1]}`);
  }
  const deployment = all.get('MACOSX_DEPLOYMENT_TARGET');
  if (deployment !== undefined && (!/^\d+(?:\.\d+)?$/.test(deployment) || Number(deployment) < 15)) add(violations, 'DEPLOYMENT_TARGET', path, `MACOSX_DEPLOYMENT_TARGET must be at least 15, got ${deployment}`);
  const archs = all.get('ARCHS');
  if (archs !== undefined && (!/(?:^|\s)arm64(?:\s|$)/.test(archs) || /x86_64|i386|\$\(ARCHS_STANDARD\)/.test(archs))) add(violations, 'ARCHITECTURE', path, `ARCHS must be arm64-only, got ${archs}`);
  const excluded = all.get('EXCLUDED_ARCHS');
  if (excluded?.includes('arm64')) add(violations, 'ARCHITECTURE', path, 'arm64 may not be excluded');
  if (all.get('SUPPORTED_PLATFORMS') !== undefined && all.get('SUPPORTED_PLATFORMS') !== 'macosx') add(violations, 'PLATFORM', path, 'SUPPORTED_PLATFORMS must be macosx');
}

export async function validateXcodeproj({ root = DEFAULT_ROOT } = {}) {
  root = resolve(root);
  const violations = [];
  const evidence = ['Mac/Pitex.xcodeproj/project.pbxproj'];
  const projectPath = join(root, 'Mac/Pitex.xcodeproj/project.pbxproj');
  let pbx = '';
  try { pbx = await readFile(projectPath, 'utf8'); } catch (error) {
    add(violations, 'PROJECT_READ', 'Mac/Pitex.xcodeproj/project.pbxproj', error.code === 'ENOENT' ? 'required Xcode project is missing' : error.message);
    return finish(violations, evidence);
  }
  if (!/^\/\/ !\$\*UTF8\*\$!/.test(pbx.split('\n')[0] ?? '')) add(violations, 'PBX_HEADER', 'Mac/Pitex.xcodeproj/project.pbxproj', 'invalid OpenStep project header');
  let refs = new Map();
  let refTypes = new Map();
  let variants = new Map();
  let childVariants = new Map();
  let buildRefs = new Map();
  let sourceBuildIds = new Set();
  let resourceBuildIds = new Set();
  try {
    const refSection = section(pbx, 'PBXFileReference');
    refs = new Map([...refSection.matchAll(/^\s*([A-Fa-f0-9]{24})\s+\/\*.*?\*\/\s*=\s*\{([^\n]+)\};/gm)].map((match) => {
      const body = match[2];
      const path = body.match(/\bpath\s*=\s*(?:"([^"]+)"|([^;]+));/)?.slice(1).find(Boolean)?.trim();
      return [match[1], path];
    }));
    refTypes = new Map([...refSection.matchAll(/^\s*([A-Fa-f0-9]{24})\s+\/\*.*?\*\/\s*=\s*\{([^\n]+)\};/gm)].map((match) => {
      const body = match[2];
      const type = body.match(/\b(?:lastKnownFileType|explicitFileType)\s*=\s*([^;]+);/)?.[1]?.trim();
      return [match[1], type];
    }));
    const variantSection = section(pbx, 'PBXVariantGroup');
    for (const match of variantSection.matchAll(
      /^\s*([A-Fa-f0-9]{24})\s+\/\*.*?\*\/\s*=\s*\{([\s\S]*?)^\s*\};/gm,
    )) {
      const body = match[2];
      const name = body.match(/\bname\s*=\s*(?:"([^"]+)"|([^;]+));/)?.slice(1).find(Boolean)?.trim();
      if (name) variants.set(match[1], name);
      const children = body.match(/\bchildren\s*=\s*\(([\s\S]*?)\);/)?.[1] ?? '';
      for (const child of children.matchAll(/\b([A-Fa-f0-9]{24})\b/g)) {
        childVariants.set(child[1], match[1]);
      }
    }
    const buildSection = section(pbx, 'PBXBuildFile');
    buildRefs = new Map([...buildSection.matchAll(/^\s*([A-Fa-f0-9]{24})\s+\/\*.*?\*\/\s*=\s*\{[^\n]*?fileRef\s*=\s*([A-Fa-f0-9]{24})/gm)].map((match) => [match[1], match[2]]));
    const sources = section(pbx, 'PBXSourcesBuildPhase');
    const resources = section(pbx, 'PBXResourcesBuildPhase');
    sourceBuildIds = new Set([...sources.matchAll(/\b([A-Fa-f0-9]{24})\b\s+\/\*[^*]* in Sources \*\//g)].map((match) => match[1]));
    resourceBuildIds = new Set([...resources.matchAll(/\b([A-Fa-f0-9]{24})\b\s+\/\*[^*]* in Resources \*\//g)].map((match) => match[1]));
  } catch (error) { add(violations, 'PBX_STRUCTURE', 'Mac/Pitex.xcodeproj/project.pbxproj', error.message); }
  if (refs.size === 0) add(violations, 'FILE_REFERENCES', 'Mac/Pitex.xcodeproj/project.pbxproj', 'no file references parsed');
  for (const [id, path] of refs) {
    if (!path) { add(violations, 'FILE_REFERENCE_PATH', 'Mac/Pitex.xcodeproj/project.pbxproj', `${id} has no literal path`); continue; }
    if (path.endsWith('.swift')) {
      const member = [...buildRefs].some(([buildId, ref]) => ref === id && sourceBuildIds.has(buildId));
      if (!member) add(violations, 'SOURCE_MEMBERSHIP', 'Mac/Pitex.xcodeproj/project.pbxproj', `${path} is not in a sources build phase`);
    }
    const extension = extname(path).toLowerCase();
    if (['.xcassets','.strings','.storyboard','.xib','.png','.jpg','.jpeg','.pdf','.json'].includes(extension)) {
      const variant = childVariants.get(id);
      const member = [...buildRefs].some(
        ([buildId, ref]) => (ref === id || ref === variant) && resourceBuildIds.has(buildId),
      );
      if (!member) add(violations, 'RESOURCE_MEMBERSHIP', 'Mac/Pitex.xcodeproj/project.pbxproj', `${path} is not in a resources build phase`);
      if (/ReferenceEvidence|reference[-_ ]?(?:capture|download|asset)|extracted[-_ ]asset/i.test(path)) add(violations, 'REFERENCE_RESOURCE', 'Mac/Pitex.xcodeproj/project.pbxproj', `${path} is not an original product resource`);
    }
  }
  for (const buildId of [...sourceBuildIds, ...resourceBuildIds]) if (!buildRefs.has(buildId)) add(violations, 'BUILD_FILE_REFERENCE', 'Mac/Pitex.xcodeproj/project.pbxproj', `${buildId} has no PBXBuildFile declaration`);
  for (const buildId of resourceBuildIds) {
    const reference = buildRefs.get(buildId);
    const path = refs.get(reference) ?? variants.get(reference);
    // Folder references (`lastKnownFileType = folder`, e.g. the bundled
    // PitexAgent skills directory) are legitimate resources that copy a
    // directory tree verbatim — they carry no resource-file extension.
    const type = refTypes.get(reference) ?? '';
    const isFolderReference = type === 'folder' || type.startsWith('folder.');
    if (!path || (!isFolderReference && !['.xcassets','.strings','.storyboard','.xib','.png','.jpg','.jpeg','.pdf','.json'].includes(extname(path).toLowerCase()))) add(violations, 'RESOURCE_TYPE', 'Mac/Pitex.xcodeproj/project.pbxproj', `${path ?? buildId} is not an approved original-resource type`);
    if (path && isFolderReference && /ReferenceEvidence|reference[-_ ]?(?:capture|download|asset)|extracted[-_ ]asset/i.test(path)) add(violations, 'REFERENCE_RESOURCE', 'Mac/Pitex.xcodeproj/project.pbxproj', `${path} is not an original product resource`);
  }
  for (const file of await filesBelow(join(root, 'Mac', 'Sources'), '.swift')) {
    const rel = relative(root, file).replaceAll(sep, '/');
    evidence.push(rel);
    const candidates = [...refs].filter(([, path]) => path && basename(path) === basename(file)).map(([id]) => id);
    const member = candidates.some((id) => [...buildRefs].some(([buildId, ref]) => ref === id && sourceBuildIds.has(buildId)));
    if (!member) add(violations, 'SOURCE_MEMBERSHIP', rel, 'source file is not referenced by a sources build phase');
  }
  const nativeTargets = [...pbx.matchAll(/isa\s*=\s*PBXNativeTarget;/g)].length;
  if (nativeTargets !== 1) add(violations, 'NATIVE_TARGET_COUNT', 'Mac/Pitex.xcodeproj/project.pbxproj', `expected exactly one app target, found ${nativeTargets}`);
  if (!/productType\s*=\s*"?com\.apple\.product-type\.application"?;/.test(pbx)) add(violations, 'PRODUCT_TYPE', 'Mac/Pitex.xcodeproj/project.pbxproj', 'macOS application target is missing');
  validateBuildSettings(pbx, 'Mac/Pitex.xcodeproj/project.pbxproj', violations);

  const xcconfigs = await filesBelow(join(root, 'Mac/Config'), '.xcconfig');
  if (xcconfigs.length === 0) add(violations, 'XCCONFIG_MISSING', 'Mac/Config', 'at least one xcconfig is required');
  let configText = '';
  for (const file of xcconfigs) {
    const rel = relative(root, file).replaceAll(sep, '/'); evidence.push(rel);
    const source = await readFile(file, 'utf8'); configText += `\n${source}`; validateBuildSettings(source, rel, violations);
  }
  const combinedSettings = settings(`${pbx}\n${configText}`);
  const deployment = combinedSettings.get('MACOSX_DEPLOYMENT_TARGET');
  if (deployment === undefined || Number(deployment) < 15) add(violations, 'DEPLOYMENT_TARGET', 'Mac/Config', 'an explicit macOS 15+ deployment target is required');
  const archs = combinedSettings.get('ARCHS');
  if (archs !== 'arm64') add(violations, 'ARCHITECTURE', 'Mac/Config', 'an explicit arm64-only ARCHS setting is required');

  const plistCandidates = [...refs.values()].filter((path) => path?.endsWith('.plist'));
  const configuredPlist = combinedSettings.get('INFOPLIST_FILE')?.replaceAll('$(SRCROOT)/', '').replace(/^"|"$/g, '');
  const plistCandidate = plistCandidates.find((path) => /Info\.plist$/.test(path));
  const plistRel = configuredPlist ? `Mac/${configuredPlist.replace(/^\.\//, '')}` : (plistCandidate ? `Mac/${plistCandidate.replace(/^\.\//, '')}` : undefined);
  let plist = '';
  if (!plistRel) add(violations, 'INFO_PLIST', 'Mac/Config', 'INFOPLIST_FILE is not configured');
  else try { evidence.push(plistRel); plist = await readFile(join(root, plistRel), 'utf8'); } catch (error) { add(violations, 'INFO_PLIST', plistRel, error.code === 'ENOENT' ? 'configured Info.plist is missing' : error.message); }
  if (plist) {
    const legacyExtensions = plistValues(plist, 'CFBundleTypeExtensions');
    const contentTypes = plistValues(plist, 'LSItemContentTypes');
    const exportedExtensions = plistValues(plist, 'public.filename-extension');
    if (!/<key>\s*CFBundleDocumentTypes\s*<\/key>/.test(plist) || (legacyExtensions.length === 0 && (contentTypes.length === 0 || exportedExtensions.length === 0))) add(violations, 'DOCUMENT_DECLARATION', plistRel, 'document types must identify filename extensions directly or through exported UTTypes');
    const schemes = plistValues(plist, 'CFBundleURLSchemes');
    if (!/<key>\s*CFBundleURLTypes\s*<\/key>/.test(plist) || schemes.length === 0 || schemes.some((scheme) => !/^[a-z][a-z0-9+.-]*$/.test(scheme))) add(violations, 'CALLBACK_DECLARATION', plistRel, 'at least one valid callback URL scheme is required');
  }

  const entitlementSetting = combinedSettings.get('CODE_SIGN_ENTITLEMENTS')?.replaceAll('$(SRCROOT)/', '').replace(/^"|"$/g, '');
  if (!entitlementSetting) add(violations, 'ENTITLEMENTS', 'Mac/Config', 'CODE_SIGN_ENTITLEMENTS must name an explicit entitlement file');
  else {
    const rel = `Mac/${entitlementSetting.replace(/^\.\//, '')}`; evidence.push(rel);
    try {
      const entitlements = await readFile(join(root, rel), 'utf8');
      if (/<key>\s*com\.apple\.security\.app-sandbox\s*<\/key>\s*<true\s*\/>/s.test(entitlements)) add(violations, 'APP_SANDBOX', rel, 'App Sandbox must not be enabled');
    } catch (error) { add(violations, 'ENTITLEMENTS', rel, error.code === 'ENOENT' ? 'configured entitlements file is missing' : error.message); }
  }

  const forbidden = [
    ['UPDATER', /Sparkle|SUUpdater|SPU(?:Standard)?Updater|auto(?:matic)?[-_ ]?update|checkForUpdates/i],
    ['PUBLIC_NOTARIZATION', /Developer\s*ID\s*Application|ENABLE_HARDENED_RUNTIME\s*=\s*YES|\bnotarytool\b/i],
    ['APP_SANDBOX', /com\.apple\.security\.app-sandbox\s*<\/key>\s*<true|ENABLE_APP_SANDBOX\s*=\s*YES/i],
    ['INTEL_ARCHITECTURE', /\bx86_64\b|\bi386\b/i]
  ];
  const staticText = `${pbx}\n${configText}\n${plist}`;
  for (const [code, pattern] of forbidden) if (pattern.test(staticText)) add(violations, code, 'Mac', `forbidden project/config declaration matches ${pattern.source}`);

  return finish(violations, [...new Set(evidence)].sort());
}
function finish(violations, evidence) {
  violations.sort((a, b) => `${a.code}:${a.path}:${a.message}`.localeCompare(`${b.code}:${b.path}:${b.message}`));
  return { tool: 'validate-xcodeproj', status: violations.length === 0 ? 'pass' : 'fail', checks: [{ command: 'static pbxproj/plist/entitlements/xcconfig validation', result: violations.length === 0 ? 'pass' : 'fail', evidence }], violations };
}
if (import.meta.url === pathToFileURL(process.argv[1] ?? '').href) {
  try { const result = await validateXcodeproj({ root: process.argv[2] ? resolve(process.argv[2]) : DEFAULT_ROOT }); process.stdout.write(`${JSON.stringify(result, null, 2)}\n`); if (result.status !== 'pass') process.exitCode = 1; }
  catch (error) { process.stdout.write(`${JSON.stringify({ tool: 'validate-xcodeproj', status: 'fail', checks: [], violations: [{ code: 'INTERNAL_ERROR', path: '', message: error.message }] }, null, 2)}\n`); process.exitCode = 1; }
}
