import type { z } from "astro/zod";
import type { ColumnType } from "../schema/types.js";

export type { ColumnType } from "../schema/types.js";

/**
 * Base field definition
 *
 * Note: schema uses z.ZodTypeAny to accommodate optional/default wrappers
 */
export interface FieldDefinition<_T = unknown> {
	type: string;
	/**
	 * The SQLite column type to use when storing this field
	 */
	columnType: ColumnType;
	schema: z.ZodTypeAny;
	options?: unknown;
	ui?: FieldUIHints;
}

/**
 * UI hints for admin rendering
 */
export interface FieldUIHints {
	widget?: string;
	placeholder?: string;
	helpText?: string;
	rows?: number; // For textarea
	min?: number | string;
	max?: number | string;
	[key: string]: unknown;
}

/**
 * Portable Text block structure
 */
export interface PortableTextBlock {
	_type: string;
	_key: string;
	[key: string]: unknown;
}

// Re-export MediaValue from media/types.ts (canonical location)
export type { MediaValue } from "../media/types.js";

/**
 * File field value
 */
export interface FileValue {
	id: string;
	url: string;
	filename: string;
	mimeType: string;
	size: number;
}
