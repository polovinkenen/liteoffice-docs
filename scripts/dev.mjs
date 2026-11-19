import { spawn } from "node:child_process";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const args = ["dev"];

if (process.platform === "darwin") {
  const runnerConfig = {
    build: {
      runner: {
        cmd: "./scripts/dev-runner.sh",
        cwd: root,
      },
    },
  };
  args.push("--config", JSON.stringify(runnerConfig));
}

const child = spawn("tauri", args, {
  stdio: "inherit",
  shell: process.platform === "win32",
});

child.on("error", (error) => {
  console.error(error);
  process.exit(1);
});

child.on("exit", (code) => {
  process.exit(code ?? 0);
});
