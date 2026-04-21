import { getFieldTypeKind } from '@/features/content-types/fieldType';
import type { ContentEntry, FieldDefinition } from '@/types';

function serializeComparableValue(value: unknown) {
  if (value === undefined) {
    return 'undefined';
  }

  try {
    return JSON.stringify(value);
  } catch {
    return String(value);
  }
}

export function isEntryFieldDirty(
  initialFields: Record<string, unknown>,
  apiId: string,
  nextValue: unknown,
) {
  return serializeComparableValue(initialFields[apiId]) !== serializeComparableValue(nextValue);
}

export function buildHostFieldError(field: FieldDefinition, value: unknown) {
  if (field.required && (value == null || value === '')) {
    return `${field.name} 为必填字段`;
  }

  if (getFieldTypeKind(field.field_type) === 'json' && typeof value === 'string' && value !== '') {
    try {
      JSON.parse(value);
    } catch {
      return `字段 ${field.name} 不是合法 JSON`;
    }
  }

  return null;
}

export function resolveInitialFields(initial?: ContentEntry | null) {
  return (initial?.fields as Record<string, unknown>) ?? {};
}