import { renderToString } from "react-dom/server";
import { expect, test } from "vitest";
import { Home } from "../src/views/Home";

test("home carries the mark", () => {
	expect(renderToString(<Home />)).toContain("ensign");
});
