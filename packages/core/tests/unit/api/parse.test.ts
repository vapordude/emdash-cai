import { describe, expect, it, vi } from "vitest";
import { z } from "zod";
import { isParseError, parseBody, parseOptionalBody, parseQuery } from "../../../src/api/parse.js";

describe("api/parse", () => {
	const schema = z.object({
		foo: z.string(),
		bar: z.number(),
	});

	describe("parseBody", () => {
		it("returns validated data for valid JSON body", async () => {
			const request = new Request("http://example.com", {
				method: "POST",
				body: JSON.stringify({ foo: "hello", bar: 123 }),
				headers: { "Content-Type": "application/json" },
			});

			const result = await parseBody(request, schema);
			expect(result).toEqual({ foo: "hello", bar: 123 });
		});

		it("returns 413 error if Content-Length exceeds MAX_BODY_SIZE", async () => {
			const request = new Request("http://example.com", {
				method: "POST",
				body: JSON.stringify({ foo: "a".repeat(100) }),
				headers: {
					"Content-Type": "application/json",
					"Content-Length": (20 * 1024 * 1024).toString(), // 20 MB
				},
			});

			const result = await parseBody(request, schema);
			expect(result).toBeInstanceOf(Response);
			if (result instanceof Response) {
				expect(result.status).toBe(413);
				const body = await result.json();
				expect(body.error.code).toBe("PAYLOAD_TOO_LARGE");
			}
		});

		it("returns 400 error for invalid JSON", async () => {
			const request = new Request("http://example.com", {
				method: "POST",
				body: "not json",
				headers: { "Content-Type": "application/json" },
			});

			const result = await parseBody(request, schema);
			expect(result).toBeInstanceOf(Response);
			if (result instanceof Response) {
				expect(result.status).toBe(400);
				const body = await result.json();
				expect(body.error.code).toBe("INVALID_JSON");
			}
		});

		it("returns 400 error for validation failure", async () => {
			const request = new Request("http://example.com", {
				method: "POST",
				body: JSON.stringify({ foo: "hello", bar: "not a number" }),
				headers: { "Content-Type": "application/json" },
			});

			const result = await parseBody(request, schema);
			expect(result).toBeInstanceOf(Response);
			if (result instanceof Response) {
				expect(result.status).toBe(400);
				const body = await result.json();
				expect(body.error.code).toBe("VALIDATION_ERROR");
				expect(body.error.details.issues).toHaveLength(1);
				expect(body.error.details.issues[0].path).toBe("bar");
			}
		});
	});

	describe("parseOptionalBody", () => {
		it("returns validated data for valid JSON body", async () => {
			const request = new Request("http://example.com", {
				method: "POST",
				body: JSON.stringify({ foo: "hello", bar: 123 }),
			});

			const result = await parseOptionalBody(request, schema, { foo: "default", bar: 0 });
			expect(result).toEqual({ foo: "hello", bar: 123 });
		});

		it("returns default value for empty body", async () => {
			const request = new Request("http://example.com", {
				method: "POST",
				body: "",
			});

			const defaultValue = { foo: "default", bar: 0 };
			const result = await parseOptionalBody(request, schema, defaultValue);
			expect(result).toEqual(defaultValue);
		});

		it("returns default value for whitespace-only body", async () => {
			const request = new Request("http://example.com", {
				method: "POST",
				body: "   ",
			});

			const defaultValue = { foo: "default", bar: 0 };
			const result = await parseOptionalBody(request, schema, defaultValue);
			expect(result).toEqual(defaultValue);
		});

		it("returns default value if request.text() fails", async () => {
			const request = {
				headers: new Headers(),
				text: vi.fn().mockRejectedValue(new Error("Failed to read")),
			} as unknown as Request;

			const defaultValue = { foo: "default", bar: 0 };
			const result = await parseOptionalBody(request, schema, defaultValue);
			expect(result).toEqual(defaultValue);
		});

		it("returns 413 error if Content-Length exceeds MAX_BODY_SIZE", async () => {
			const request = new Request("http://example.com", {
				method: "POST",
				headers: {
					"Content-Length": (20 * 1024 * 1024).toString(),
				},
			});

			const result = await parseOptionalBody(request, schema, { foo: "d", bar: 0 });
			expect(result).toBeInstanceOf(Response);
			if (result instanceof Response) {
				expect(result.status).toBe(413);
			}
		});

		it("returns 400 error for invalid JSON", async () => {
			const request = new Request("http://example.com", {
				method: "POST",
				body: "{ invalid }",
			});

			const result = await parseOptionalBody(request, schema, { foo: "d", bar: 0 });
			expect(result).toBeInstanceOf(Response);
			if (result instanceof Response) {
				expect(result.status).toBe(400);
				const body = await result.json();
				expect(body.error.code).toBe("INVALID_JSON");
			}
		});
	});

	describe("parseQuery", () => {
		const querySchema = z.object({
			page: z.coerce.number(),
			search: z.string().optional(),
		});

		it("parses and coerces query parameters", () => {
			const url = new URL("http://example.com?page=2&search=test");
			const result = parseQuery(url, querySchema);
			expect(result).toEqual({ page: 2, search: "test" });
		});

		it("returns 400 for validation failure", async () => {
			const url = new URL("http://example.com?page=abc");
			const result = parseQuery(url, querySchema);
			expect(result).toBeInstanceOf(Response);
			if (result instanceof Response) {
				expect(result.status).toBe(400);
				const body = await result.json();
				expect(body.error.code).toBe("VALIDATION_ERROR");
			}
		});
	});

	describe("isParseError", () => {
		it("returns true for Response", () => {
			expect(isParseError(new Response())).toBe(true);
		});

		it("returns false for plain objects", () => {
			expect(isParseError({ foo: "bar" })).toBe(false);
		});

		it("returns false for null/undefined", () => {
			expect(isParseError(null)).toBe(false);
			expect(isParseError(undefined)).toBe(false);
		});
	});
});
