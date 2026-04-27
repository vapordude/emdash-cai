import { createOpenAICompatible } from "@ai-sdk/openai-compatible";
import { generateObject } from "ai";
import { z } from "zod";
import { buildGuardPrompt } from "./guard.js";
import type { GuardResult } from "./guard.js";

export interface UniversalAIConfig {
	baseURL: string;
	apiKey: string;
	model: string;
}

const responseSchema = z.object({
	safe: z.boolean(),
	categories: z.array(z.string()),
});

export async function runUniversalGuard(
	text: string,
	taxonomy: string,
	config: UniversalAIConfig
): Promise<GuardResult> {
	const openai = createOpenAICompatible({
		name: 'universal-ai',
		baseURL: config.baseURL,
		apiKey: config.apiKey,
	});

	const model = openai(config.model);
	const prompt = buildGuardPrompt(text, taxonomy);

	const { object } = await generateObject({
		model,
		schema: responseSchema,
		messages: [{ role: "user", content: prompt }],
		temperature: 0.1,
	});

	return {
		safe: object.safe,
		categories: object.categories,
	};
}
