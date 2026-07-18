import { renderToString } from "react-dom/server";
import { expect, test } from "vitest";
import { Mark } from "../src/mark/Mark";

test("mark names the product", () => {
	expect(renderToString(<Mark />)).toContain("ensign");
});
