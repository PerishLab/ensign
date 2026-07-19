import { bin } from "@/lib/std/cmd.ts";
import { io } from "@/lib/std/io.ts";
import { negentropy } from "@/lib/negentropy.ts";

io.print("==> cargo fmt");
await bin("cargo").run(["fmt", "--all", "--check"]);

io.print("==> cargo clippy");
await bin("cargo").run([
  "clippy",
  "--locked",
  "--workspace",
  "--all-targets",
  "--",
  "-D",
  "warnings",
]);

io.print("==> cargo test");
await bin("cargo").run(["test", "--locked", "--workspace"]);

io.print("==> pnpm install");
await bin("pnpm").run(["install", "--frozen-lockfile"]);

io.print("==> biome");
await bin("pnpm").run(["biome", "ci", "."]);

io.print("==> tsc");
await bin("pnpm").run(["-r", "exec", "tsc", "--noEmit"]);

io.print("==> vitest");
await bin("pnpm").run(["-r", "test"]);

io.print("==> helm lint");
const helm = (Deno.env.get("PATH") ?? "").split(":").some((dir) => {
  try {
    Deno.statSync(`${dir}/helm`);
    return true;
  } catch {
    return false;
  }
});
if (helm) {
  await bin("helm").run(["lint", "charts/ensign", "--quiet"]);
} else {
  io.print("helm absent: chart lint stays a local gate");
}

io.print("==> deno fmt");
await bin("deno").run(["fmt", "--check", ".runseal"]);

io.print("==> deno check");
await bin("deno").run([
  "check",
  "--config",
  ".runseal/deno.json",
  "--lock",
  ".runseal/deno.lock",
  "--frozen=true",
  ".runseal/wrappers/guard.ts",
  ".runseal/wrappers/init.ts",
  ".runseal/wrappers/land.ts",
  ".runseal/wrappers/act.ts",
  ".runseal/wrappers/ship.ts",
]);

io.print("==> negentropy");
await negentropy.verify();
await bin("negentropy").run(["--strict", "."]);

io.print("==> act");
const root = await bin("git").text(["rev-parse", "--show-toplevel"]);
await bin("deno").run([
  "run",
  "--allow-read",
  "--allow-write",
  "--allow-env",
  "--allow-net",
  "--allow-run",
  "--config",
  ".runseal/deno.json",
  "--lock",
  ".runseal/deno.lock",
  "--frozen=true",
  ".runseal/wrappers/act.ts",
], { cwd: root });

io.print("guard: clean");
