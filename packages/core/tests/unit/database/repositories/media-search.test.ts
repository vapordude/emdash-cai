import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { type Kysely } from "kysely";
import { MediaRepository } from "../../../../src/database/repositories/media.js";
import { type Database } from "../../../../src/database/types.js";
import { setupTestDatabase, teardownTestDatabase } from "../../../utils/test-db.js";

describe("MediaRepository search", () => {
	let db: Kysely<Database>;
	let repo: MediaRepository;

	beforeEach(async () => {
		db = await setupTestDatabase();
		repo = new MediaRepository(db);

		// Seed some media items
		await repo.create({
			filename: "vacation.jpg",
			mimeType: "image/jpeg",
			storageKey: "vacation.jpg",
			alt: "A sunny beach",
			caption: "Summer 2023",
		});

		await repo.create({
			filename: "work.pdf",
			mimeType: "application/pdf",
			storageKey: "work.pdf",
			alt: "Project report",
		});

		await repo.create({
			filename: "family.png",
			mimeType: "image/png",
			storageKey: "family.png",
			caption: "Christmas dinner",
		});
	});

	afterEach(async () => {
		await teardownTestDatabase(db);
	});

	it("finds items by filename", async () => {
		const result = await repo.findMany({ query: "vacation" });
		expect(result.items).toHaveLength(1);
		expect(result.items[0].filename).toBe("vacation.jpg");
	});

	it("finds items by alt text", async () => {
		const result = await repo.findMany({ query: "report" });
		expect(result.items).toHaveLength(1);
		expect(result.items[0].filename).toBe("work.pdf");
	});

	it("finds items by caption", async () => {
		const result = await repo.findMany({ query: "Christmas" });
		expect(result.items).toHaveLength(1);
		expect(result.items[0].filename).toBe("family.png");
	});

	it("finds multiple items matching the query", async () => {
		const result = await repo.findMany({ query: "." }); // Matches all filenames because of the dot
		expect(result.items).toHaveLength(3);
	});

	it("returns empty result for non-matching query", async () => {
		const result = await repo.findMany({ query: "mountain" });
		expect(result.items).toHaveLength(0);
	});

	it("escapes special LIKE characters in query", async () => {
		await repo.create({
			filename: "percent%sign.jpg",
			mimeType: "image/jpeg",
			storageKey: "percent.jpg",
		});

		const result = await repo.findMany({ query: "percent%" });
		expect(result.items).toHaveLength(1);
		expect(result.items[0].filename).toBe("percent%sign.jpg");
	});
});
