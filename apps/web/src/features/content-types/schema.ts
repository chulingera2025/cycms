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
  'custom',
]);

const baseFieldDefinitionSchema = z.object({
  name: z.string().min(1, '请输入字段名称'),
  api_id: z
    .string()
    .regex(/^[a-z][a-z0-9_]*$/, 'API ID 须以小写字母开头，仅含小写字母、数字、下划线'),
  field_type: fieldTypeEnum,
  required: z.boolean(),
  unique: z.boolean(),
  relation_target: z.string().optional(),
  relation_kind: z.enum(['one_to_one', 'one_to_many', 'many_to_many']).optional(),
  custom_type_name: z.string().optional(),
  default_value: z.unknown().optional(),
});

export const fieldDefinitionSchema = baseFieldDefinitionSchema.superRefine((field, ctx) => {
  if (field.field_type === 'relation') {
    if (!field.relation_target) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        path: ['relation_target'],
        message: '请选择目标类型',
      });
    }
    if (!field.relation_kind) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        path: ['relation_kind'],
        message: '请选择关系类型',
      });
    }
  }

  if (field.field_type === 'custom' && !field.custom_type_name?.trim()) {
    ctx.addIssue({
      code: z.ZodIssueCode.custom,
      path: ['custom_type_name'],
      message: '请输入自定义字段类型名',
    });
  }
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

export type FieldTypeOption = z.infer<typeof fieldTypeEnum>;
export type ContentTypeFieldFormValue = z.infer<typeof fieldDefinitionSchema>;
export type ContentTypeFormValues = z.infer<typeof contentTypeCreateSchema>;
export type ContentTypeCreateInput = ContentTypeFormValues;
export type ContentTypeUpdateInput = z.infer<typeof contentTypeUpdateSchema>;
