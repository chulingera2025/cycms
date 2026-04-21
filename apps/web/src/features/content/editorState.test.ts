import { describe, expect, it } from 'vitest';
import type { FieldDefinition } from '@/types';
import { buildHostFieldError, isEntryFieldDirty, resolveInitialFields } from './editorState';

function createField(overrides: Partial<FieldDefinition>): FieldDefinition {
  return {
    name: '字段',
    api_id: 'field',
    field_type: { kind: 'text' },
    required: false,
    unique: false,
    validations: [],
    position: 0,
    ...overrides,
  };
}

describe('editorState helpers', () => {
  it('detects field dirty state by serialized value', () => {
    expect(isEntryFieldDirty({ title: 'hello' }, 'title', 'world')).toBe(true);
    expect(isEntryFieldDirty({ meta: { enabled: true } }, 'meta', { enabled: true })).toBe(false);
  });

  it('builds required and json validation errors', () => {
    expect(buildHostFieldError(createField({ required: true, name: '标题' }), '')).toBe(
      '标题 为必填字段',
    );

    expect(
      buildHostFieldError(createField({ name: '配置', field_type: { kind: 'json' } }), '{broken'),
    ).toBe('字段 配置 不是合法 JSON');
  });

  it('resolves initial entry fields safely', () => {
    expect(resolveInitialFields(undefined)).toEqual({});
    expect(resolveInitialFields({ fields: { title: 'Hello' } } as never)).toEqual({ title: 'Hello' });
  });
});