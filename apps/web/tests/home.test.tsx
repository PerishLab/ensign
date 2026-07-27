import { renderToString } from "react-dom/server";
import { expect, test } from "vitest";
import Login from "../src/views/login";

test("login offers the sign-in action", () => {
	expect(renderToString(<Login />)).toContain("Sign in");
});
