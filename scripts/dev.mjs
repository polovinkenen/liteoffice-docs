import { spawn } from "node:child_process";

const args = ["dev"];
if (process.platform === "darwin") {
  args.push("--config", "src-tauri/tauri.macos.dev.conf.json");
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
