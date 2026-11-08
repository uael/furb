import { expect, test } from "bun:test";
import { Closed, Refused } from "../src/skill.ts";

test("a refusal of the daemon is named by its kind and a request that its socket left is Closed, the two that the window and a skill share", () => {
  expect(new Refused("role", "No role.").name).toBe("role");
  expect(new Refused("refused", "No.").name).toBe("Refused");
  expect(new Closed("gone")).toMatchObject({ name: "Closed", message: "gone" });
  expect(new Closed("gone")).toBeInstanceOf(Error);
});
