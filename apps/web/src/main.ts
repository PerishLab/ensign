import source from "virtual:perish/views";
import { Views } from "@perishlab/design";
import { mount } from "svelte";
import { boot } from "./lib/observe.ts";

void boot();
const target = document.getElementById("root");
if (target !== null) mount(Views, { target, props: { source } });
