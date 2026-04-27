import { createOpenAICompatible } from "@ai-sdk/openai-compatible";
import { embed } from "ai";

export interface UniversalAIConfig {
	baseURL: string;
	apiKey: string;
	model: string;
}

export async function generateUniversalEmbedding(
	text: string,
	config: UniversalAIConfig
): Promise<number[]> {
	const openai = createOpenAICompatible({
		name: 'universal-ai',
		baseURL: config.baseURL,
		headers: {
			Authorization: `Bearer ${config.apiKey}`,
		},
	});

	const model = openai.embeddingModel(config.model);

	const { embedding } = await embed({
		model,
		value: text,
	});

	return embedding;
}
