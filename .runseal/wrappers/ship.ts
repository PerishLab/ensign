import { cli, flags } from "@/lib/cli.ts";
import { bin } from "@/lib/std/cmd.ts";
import { io } from "@/lib/std/io.ts";

const REG = "git.perish.top/perishlab";
const CREDS = `${Deno.env.get("HOME")}/.cargo/credentials.toml`;

function usage(): void {
  io.print("Usage: runseal :ship");
  io.print("");
  io.print("Build and push the ensign api image and the helm chart");
  io.print("to the perish registry, one version train. From a clean main.");
}

const args = cli.parse(Deno.args, { boolean: ["help", "h"] });
if (flags(args).help()) {
  flags(args).positionals("ship", { allowHelp: true });
  usage();
  Deno.exit(0);
}
flags(args).positionals("ship");

const root = await bin("git").text(["rev-parse", "--show-toplevel"]);
const branch = await bin("git").text(["branch", "--show-current"]);
if (branch !== "main") {
  io.fail(`ship: publish from main, not ${branch}`);
}
const dirty = await bin("git").text(["status", "--short"]);
if (dirty.trim() !== "") {
  io.fail("ship: working tree must be clean");
}

const version = await current(root);
io.print(`==> ship v${version}`);

await forge("api", "deploy/api.Dockerfile", true, root, version);
await chart(root, version);

io.print("ship: clean");

async function forge(
  face: string,
  file: string,
  needsCargo: boolean,
  root: string,
  version: string,
): Promise<void> {
  const image = `${REG}/ensign-${face}:${version}`;
  if (await present(image)) {
    io.print(`==> ensign-${face} v${version} already pushed`);
    return;
  }
  io.print(`==> build ensign-${face}`);
  const build = ["build", "-f", file, "-t", image];
  if (needsCargo) {
    build.push("--secret", `id=cargo,src=${CREDS}`);
  }
  build.push(".");
  await bin("docker").run(build, {
    cwd: root,
    env: { DOCKER_BUILDKIT: "1" },
  });
  io.print(`==> push ensign-${face}`);
  await bin("docker").run(["push", image], { cwd: root });
}

async function chart(root: string, version: string): Promise<void> {
  io.print("==> package chart");
  await bin("helm").run([
    "package",
    "charts/ensign",
    "--version",
    version,
    "--app-version",
    version,
    "--destination",
    "/tmp",
  ], { cwd: root });
  io.print("==> push chart");
  await bin("helm").run([
    "push",
    `/tmp/ensign-${version}.tgz`,
    `oci://${REG}/charts`,
  ], { cwd: root });
}

async function present(image: string): Promise<boolean> {
  const probe = await bin("docker").status(["manifest", "inspect", image]);
  return probe === 0;
}

async function current(root: string): Promise<string> {
  const text = await Deno.readTextFile(`${root}/Cargo.toml`);
  const hit = text.match(/^version = "([^"]+)"$/m);
  if (!hit) {
    io.fail("ship: no workspace version in Cargo.toml");
  }
  return hit![1];
}
