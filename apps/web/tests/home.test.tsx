import { renderToString } from "react-dom/server";
import { expect, test } from "vitest";
import Login from "../src/views/login";

test("login offers the sign-in action", () => {
	const markup = renderToString(<Login />);
	expect(markup).toContain("Sign in");
	expect(markup).toContain("<form");
	expect(markup).toContain('type="submit"');
});
