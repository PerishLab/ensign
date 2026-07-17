import { renderToString } from "react-dom/server";
import { expect, test } from "vitest";
import { Mark } from "../src/Mark";

test("mark names the product", () => {
	expect(renderToString(<Mark />)).toBe("<h1>ensign</h1>");
});
