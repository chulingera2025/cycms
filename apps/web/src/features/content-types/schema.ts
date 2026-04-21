import { z } from 'zod';

export const fieldTypeEnum = z.enum([
  'string',
  'text',
  'richtext',
  'integer',
  'float',
  'boolean',
  'datetime',
  'json',
  'media',
  'relation',
]);

export const fieldDefinitionSchema = z.object({
  name: z.string().min(1, '请输入字段名称'),
  api_id: z
    .string()
    .regex(/^[a-z][a-z0-9_]*$/, 'API ID 须以小写字母开头，仅含小写字母、数字、下划线'),
  field_type: fieldTypeEnum,
  required: z.boolean(),
  unique: z.boolean(),
  localized: z.boolean(),
  description: z.string().optional(),
  relation_target: z.string().optional(),
  relation_kind: z
    .enum(['one_to_one', 'one_to_many', 'many_to_one', 'many_to_many'])
    .optional(),
  default_value: z.unknown().optional(),
  validation_rules: z.array(z.unknown()),
});

export const contentTypeCreateSchema = z.object({
  name: z.string().min(1, '请输入名称'),
  api_id: z
    .string()
    .regex(/^[a-z][a-z0-9_]*$/, 'API ID 须以小写字母开头，仅含小写字母、数字、下划线'),
  description: z.string().optional(),
  kind: z.enum(['collection', 'single']),
  fields: z.array(fieldDefinitionSchema),
});

export const contentTypeUpdateSchema = z.object({
  name: z.string().min(1),
  api_id: z.string(),
  description: z.string().optional(),
  kind: z.enum(['collection', 'single']),
  fields: z.array(fieldDefinitionSchema),
});

export type ContentTypeCreateInput = z.infer<typeof contentTypeCreateSchema>;
export type ContentTypeUpdateInput = z.infer<typeof contentTypeUpdateSchema>;
