/**
 * Llama Guard Integration
 *
 * Builds prompts for and parses responses from Llama Guard 3 8B via Workers AI.
 */

export interface GuardResult {
	safe: boolean;
	categories: string[];
}

const CATEGORY_SPLIT_PATTERN = /[,\s]+/;

/** Structural markers used by the Llama Guard prompt format */
const STRUCTURAL_MARKERS = /(<\/?(?:BEGIN|END)\s+(?:UNSAFE CONTENT CATEGORIES|CONVERSATION)>)/gi;

/**
 * Sanitize user text to prevent prompt injection via structural markers.
 */
function sanitizeUserText(text: string): string {
	return text.replace(STRUCTURAL_MARKERS, "");
}

/**
 * Build a Llama Guard prompt for classifying a comment.
 */
export function buildGuardPrompt(text: string, taxonomy: string): string {
	const sanitized = sanitizeUserText(text);
	return `Task: Check if there is unsafe content in the 'User' message according to our safety policy with the below categories.

<BEGIN UNSAFE CONTENT CATEGORIES>
${taxonomy}
<END UNSAFE CONTENT CATEGORIES>

<BEGIN CONVERSATION>

User: ${sanitized}

<END CONVERSATION>

Provide your safety assessment for the User message. First line must be 'safe' or 'unsafe'. If unsafe, list the violated categories on the second line separated by commas.`;
}
