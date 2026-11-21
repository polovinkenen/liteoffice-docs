#!/usr/bin/env node
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, readdirSync, readFileSync } from "node:fs";
import { dirname, join, resolve, basename } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const licenseFilePattern = /^(licen[cs]e|copying|copyright|notice)/i;
const maxTextBytes = 512 * 1024;

function readLicenseTexts(directory) {
  if (!existsSync(directory)) {
    return [];
  }
  const files = readdirSync(directory, { withFileTypes: true })
    .filter((entry) => entry.isFile() && licenseFilePattern.test(entry.name))
    .map((entry) => join(directory, entry.name))
    .sort();
  const texts = [];
  for (const file of files) {
    const bytes = readFileSync(file);
    if (bytes.length > 0 && bytes.length <= maxTextBytes) {
      const label = file.startsWith(`${root}/`)
        ? file.slice(root.length + 1)
        : `${basename(directory)}/${basename(file)}`;
      texts.push({ file: label, text: bytes.toString("utf8").trim() });
    }
  }
  return texts;
}

function rustComponents() {
  execFileSync("cargo", ["fetch", "--locked"], {
    cwd: root,
    stdio: "ignore",
  });
  const metadata = JSON.parse(
    execFileSync("cargo", ["metadata", "--format-version", "1", "--locked"], {
      cwd: root,
      encoding: "utf8",
      maxBuffer: 256 * 1024 * 1024,
    }),
  );
  const components = [];
  for (const pkg of metadata.packages) {
    if (!pkg.source || pkg.source.startsWith("path+")) {
      continue;
    }
    components.push({
      name: pkg.name,
      version: pkg.version,
      license: pkg.license || (pkg.license_file ? "см. файл лицензии" : "не указана"),
      texts: pkg.manifest_path
        ? readLicenseTexts(dirname(pkg.manifest_path))
        : [],
    });
  }
  return components;
}

function npmComponents() {
  const lock = JSON.parse(readFileSync(join(root, "package-lock.json"), "utf8"));
  const components = [];
  for (const [path, entry] of Object.entries(lock.packages || {})) {
    if (!path.startsWith("node_modules/") || entry.dev || entry.link) {
      continue;
    }
    const directory = join(root, path);
    const pkgJsonPath = join(directory, "package.json");
    const manifest = existsSync(pkgJsonPath)
      ? JSON.parse(readFileSync(pkgJsonPath, "utf8"))
      : {};
    const license = manifest.license || entry.license || "не указана";
    components.push({
      name: manifest.name || entry.name || path,
      version: manifest.version || entry.version || "",
      license: typeof license === "string" ? license : JSON.stringify(license),
      texts: readLicenseTexts(directory),
    });
  }
  return components;
}

function fontsComponent() {
  const directory = join(root, "crates/export-pdf/assets/fonts");
  return {
    name: "Liberation fonts",
    version: "",
    license: "OFL-1.1",
    texts: readLicenseTexts(directory),
  };
}

function sortComponents(components) {
  return components.sort((left, right) =>
    left.name === right.name
      ? left.version.localeCompare(right.version)
      : left.name.localeCompare(right.name),
  );
}

function table(components) {
  const lines = [
    "| Компонент | Версия | Лицензия |",
    "|---|---|---|",
  ];
  for (const component of components) {
    const license = component.license.replaceAll("|", "\\|");
    lines.push(`| ${component.name} | ${component.version} | ${license} |`);
  }
  return lines.join("\n");
}

function licenseTexts(components) {
  const groups = new Map();
  for (const component of components) {
    for (const item of component.texts) {
      const hash = createHash("sha256").update(item.text).digest("hex");
      if (!groups.has(hash)) {
        groups.set(hash, { text: item.text, components: new Set(), files: new Set() });
      }
      const group = groups.get(hash);
      group.components.add(
        component.version ? `${component.name} ${component.version}` : component.name,
      );
      group.files.add(item.file);
    }
  }
  return [...groups.values()]
    .map((group) => ({
      ...group,
      components: [...group.components].sort(),
      files: [...group.files].sort(),
    }))
    .sort((left, right) => left.components[0].localeCompare(right.components[0]));
}

const rust = sortComponents(rustComponents());
const npm = sortComponents(npmComponents());
const fonts = fontsComponent();
const groups = licenseTexts([...rust, ...npm, fonts]);

const sections = [
  "# Сторонние компоненты",
  "",
  "Файл сгенерирован `node scripts/third-party-notices.mjs`; не редактируйте вручную.",
  "Список охватывает зависимости Rust, npm-пакеты, попадающие в бандл, и встроенные",
  "шрифты Liberation. Тексты лицензий приведены ниже.",
  "",
  "## Rust-зависимости",
  "",
  table(rust),
  "",
  "## npm-зависимости",
  "",
  table(npm),
  "",
  "## Шрифты",
  "",
  table([fonts]),
  "",
  "## Тексты лицензий",
];

function fenceFor(text) {
  const longestRun = Math.max(
    2,
    ...(text.match(/`+/g) || []).map((run) => run.length),
  );
  return "`".repeat(longestRun + 1);
}

for (const [index, group] of groups.entries()) {
  const names = group.components.join(", ");
  const fence = fenceFor(group.text);
  sections.push(
    "",
    `### ${index + 1}. ${group.components.length} компонент(ов)`,
    "",
    `Компоненты: ${names}`,
    "",
    `Файл: \`${group.files.join("`, `")}\``,
    "",
    `${fence}text`,
    group.text,
    fence,
  );
}

process.stdout.write(`${sections.join("\n")}\n`);
