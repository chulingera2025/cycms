import type {
  FieldDefinition,
  FieldType,
  FieldTypeKind,
  RelationKind,
} from '@/types';
import type { ContentTypeFieldFormValue, FieldTypeOption } from './schema';

const FIELD_TYPE_LABELS: Record<FieldTypeKind, string> = {
  text: '文本',
  richtext: '富文本',
  number: '数字',
  boolean: '布尔',
  datetime: '日期时间',
  json: 'JSON',
  media: '媒体',
  relation: '关联',
  custom: '自定义',
};

export function getFieldTypeKind(fieldType: FieldType): FieldTypeKind {
  return fieldType.kind;
}

export function getFieldTypeLabel(fieldType: FieldType): string {
  if (fieldType.kind === 'number') {
    return fieldType.decimal ? '浮点数' : '整数';
  }

  if (fieldType.kind === 'custom') {
    return fieldType.type_name;
  }

  return FIELD_TYPE_LABELS[fieldType.kind];
}

export function getRelationConfig(fieldType: FieldType): {
  targetType?: string;
  relationKind?: RelationKind;
} {
  if (fieldType.kind !== 'relation') {
    return {};
  }

  return {
    targetType: fieldType.target_type,
    relationKind: fieldType.relation_kind,
  };
}

export function isMultiRelationField(fieldType: FieldType): boolean {
  return (
    fieldType.kind === 'relation' &&
    (fieldType.relation_kind === 'one_to_many' ||
      fieldType.relation_kind === 'many_to_many')
  );
}

export function fieldTypeToOption(fieldType: FieldType): FieldTypeOption {
  switch (fieldType.kind) {
    case 'text':
      return 'string';
    case 'richtext':
      return 'richtext';
    case 'number':
      return fieldType.decimal ? 'float' : 'integer';
    case 'boolean':
      return 'boolean';
    case 'datetime':
      return 'datetime';
    case 'json':
      return 'json';
    case 'media':
      return 'media';
    case 'relation':
      return 'relation';
    case 'custom':
      return 'custom';
  }
}

export function fieldTypeFromOption(
  option: FieldTypeOption,
  config: {
    relationTarget?: string;
    relationKind?: RelationKind;
    customTypeName?: string;
  } = {},
): FieldType {
  switch (option) {
    case 'string':
    case 'text':
      return { kind: 'text' };
    case 'richtext':
      return { kind: 'richtext' };
    case 'integer':
      return { kind: 'number', decimal: false };
    case 'float':
      return { kind: 'number', decimal: true };
    case 'boolean':
      return { kind: 'boolean' };
    case 'datetime':
      return { kind: 'datetime' };
    case 'json':
      return { kind: 'json' };
    case 'media':
      return { kind: 'media', allowed_types: [] };
    case 'relation':
      return {
        kind: 'relation',
        target_type: config.relationTarget ?? '',
        relation_kind: config.relationKind ?? 'one_to_one',
      };
    case 'custom':
      return {
        kind: 'custom',
        type_name: config.customTypeName?.trim() ?? '',
      };
  }
}

export function fieldDefinitionToFormValue(
  field: FieldDefinition,
): ContentTypeFieldFormValue {
  const relation = getRelationConfig(field.field_type);

  return {
    name: field.name,
    api_id: field.api_id,
    field_type: fieldTypeToOption(field.field_type),
    required: field.required,
    unique: field.unique,
    relation_target: relation.targetType,
    relation_kind: relation.relationKind,
    custom_type_name:
      field.field_type.kind === 'custom' ? field.field_type.type_name : undefined,
    default_value: field.default_value,
  };
}

export function formValueToFieldDefinition(
  field: ContentTypeFieldFormValue,
  position: number,
): FieldDefinition {
  return {
    name: field.name,
    api_id: field.api_id,
    field_type: fieldTypeFromOption(field.field_type, {
      relationTarget: field.relation_target,
      relationKind: field.relation_kind,
      customTypeName: field.custom_type_name,
    }),
    required: field.required,
    unique: field.unique,
    default_value: field.default_value,
    description: undefined,
    validations: [],
    position,
  };
}