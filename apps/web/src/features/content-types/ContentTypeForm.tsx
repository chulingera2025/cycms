import { useEffect } from 'react';
import { zodResolver } from '@hookform/resolvers/zod';
import { Alert, Button, Drawer, Form, Input, Select, Space, Switch } from 'antd';
import {
  Controller,
  useFieldArray,
  useForm,
  useWatch,
  type Control,
  type FieldErrors,
  type Resolver,
} from 'react-hook-form';
import { Plus, Trash2 } from 'lucide-react';
import {
  fieldDefinitionToFormValue,
} from './fieldType';
import { useContentTypes } from './hooks';
import {
  contentTypeCreateSchema,
  contentTypeUpdateSchema,
  type ContentTypeFormValues,
  type FieldTypeOption,
} from './schema';
import type { ContentTypeDefinition } from '@/types';

const FIELD_TYPES: { value: FieldTypeOption; label: string }[] = [
  { value: 'string', label: '单行文本' },
  { value: 'text', label: '文本' },
  { value: 'richtext', label: '富文本' },
  { value: 'integer', label: '整数' },
  { value: 'float', label: '浮点数' },
  { value: 'boolean', label: '布尔' },
  { value: 'datetime', label: '日期时间' },
  { value: 'json', label: 'JSON' },
  { value: 'media', label: '媒体' },
  { value: 'relation', label: '关联' },
  { value: 'custom', label: '自定义' },
];

const RELATION_KINDS = [
  { value: 'one_to_one', label: '一对一' },
  { value: 'one_to_many', label: '一对多' },
  { value: 'many_to_many', label: '多对多' },
];

type FormValues = ContentTypeFormValues;

interface Props {
  open: boolean;
  initial?: ContentTypeDefinition | null;
  onClose: () => void;
  onSubmit: (values: FormValues) => Promise<void>;
  loading?: boolean;
}

export function ContentTypeForm({ open, initial, onClose, onSubmit, loading }: Props) {
  const isEdit = Boolean(initial);
  const resolver = (
    isEdit ? zodResolver(contentTypeUpdateSchema) : zodResolver(contentTypeCreateSchema)
  ) as Resolver<FormValues>;

  const { data: allTypes = [] } = useContentTypes();

  const {
    control,
    handleSubmit,
    reset,
    formState: { errors },
  } = useForm<FormValues>({
    resolver,
    defaultValues: {
      name: '',
      api_id: '',
      description: '',
      kind: 'collection',
      fields: [],
    },
  });

  const { fields, append, remove } = useFieldArray({ control, name: 'fields' });

  useEffect(() => {
    if (open) {
      reset({
        name: initial?.name ?? '',
        api_id: initial?.api_id ?? '',
        description: initial?.description ?? '',
        kind: initial?.kind ?? 'collection',
        fields: initial?.fields.map(fieldDefinitionToFormValue) ?? [],
      });
    }
  }, [open, initial, reset]);

  function addField() {
    append({
      name: '',
      api_id: '',
      field_type: 'string',
      required: false,
      unique: false,
      relation_target: undefined,
      relation_kind: undefined,
      custom_type_name: undefined,
      default_value: undefined,
    });
  }

  async function handleFormSubmit(values: FormValues) {
    await onSubmit({
      ...values,
      description: values.description?.trim() || undefined,
      fields: values.fields.map((field) => ({
        ...field,
        relation_target: field.field_type === 'relation' ? field.relation_target : undefined,
        relation_kind: field.field_type === 'relation' ? field.relation_kind : undefined,
        custom_type_name:
          field.field_type === 'custom' ? field.custom_type_name?.trim() : undefined,
      })),
    });
  }

  return (
    <Drawer
      open={open}
      title={isEdit ? '编辑内容类型' : '新建内容类型'}
      width={720}
      onClose={onClose}
      destroyOnClose
      extra={
        <Space>
          <Button onClick={onClose}>取消</Button>
          <Button type="primary" loading={loading} onClick={handleSubmit(handleFormSubmit)}>
            保存
          </Button>
        </Space>
      }
    >
      <Form layout="vertical">
        <div className="grid grid-cols-1 gap-x-4 md:grid-cols-2">
          <Controller
            name="name"
            control={control}
            render={({ field }) => (
              <Form.Item
                label="名称"
                validateStatus={errors.name ? 'error' : undefined}
                help={errors.name?.message}
              >
                <Input {...field} placeholder="文章" />
              </Form.Item>
            )}
          />
          <Controller
            name="api_id"
            control={control}
            render={({ field }) => (
              <Form.Item
                label="API ID"
                validateStatus={errors.api_id ? 'error' : undefined}
                help={errors.api_id?.message ?? (isEdit ? '创建后不可修改' : undefined)}
              >
                <Input {...field} placeholder="article" disabled={isEdit} />
              </Form.Item>
            )}
          />
        </div>
        <Controller
          name="description"
          control={control}
          render={({ field }) => (
            <Form.Item label="描述">
              <Input.TextArea
                value={field.value ?? ''}
                onChange={field.onChange}
                onBlur={field.onBlur}
                rows={2}
              />
            </Form.Item>
          )}
        />
        <Controller
          name="kind"
          control={control}
          render={({ field }) => (
            <Form.Item label="类型">
              <Select
                value={field.value}
                onChange={field.onChange}
                disabled={isEdit}
                options={[
                  { value: 'collection', label: 'Collection（多条）' },
                  { value: 'single', label: 'Single（单例）' },
                ]}
              />
            </Form.Item>
          )}
        />

        <div className="mb-3 flex items-center justify-between">
          <span className="font-medium text-text">字段设计</span>
          <Button size="small" icon={<Plus size={12} />} onClick={addField}>
            添加字段
          </Button>
        </div>
        <div className="flex flex-col gap-3">
          {fields.length === 0 && (
            <Alert
              type="info"
              showIcon
              message="暂无字段"
              description="添加至少一个字段以开始使用"
            />
          )}
          {fields.map((f, i) => (
            <FieldRow
              key={f.id}
              index={i}
              control={control}
              errors={errors}
              allTypes={allTypes}
              onRemove={() => remove(i)}
            />
          ))}
        </div>
      </Form>
    </Drawer>
  );
}

interface RowProps {
  index: number;
  control: Control<FormValues>;
  errors: FieldErrors<FormValues>;
  allTypes: ContentTypeDefinition[];
  onRemove: () => void;
}

function FieldRow({ index, control, errors, allTypes, onRemove }: RowProps) {
  const fieldType = useWatch({
    control,
    name: `fields.${index}.field_type`,
  }) as FieldTypeOption | undefined;
  const fieldErrors = errors.fields?.[index];

  return (
    <div className="rounded border border-border bg-surface-alt p-3">
      <div className="grid grid-cols-1 items-start gap-2 md:grid-cols-[1fr_1fr_1fr_auto]">
        <Controller
          name={`fields.${index}.name`}
          control={control}
          render={({ field }) => (
            <Form.Item
              label="名称"
              className="!mb-1"
              validateStatus={fieldErrors?.name ? 'error' : undefined}
              help={fieldErrors?.name?.message}
            >
              <Input {...field} size="small" placeholder="标题" />
            </Form.Item>
          )}
        />
        <Controller
          name={`fields.${index}.api_id`}
          control={control}
          render={({ field }) => (
            <Form.Item
              label="API ID"
              className="!mb-1"
              validateStatus={fieldErrors?.api_id ? 'error' : undefined}
              help={fieldErrors?.api_id?.message}
            >
              <Input {...field} size="small" placeholder="title" />
            </Form.Item>
          )}
        />
        <Controller
          name={`fields.${index}.field_type`}
          control={control}
          render={({ field }) => (
            <Form.Item label="类型" className="!mb-1">
              <Select
                value={field.value}
                onChange={field.onChange}
                size="small"
                options={FIELD_TYPES}
              />
            </Form.Item>
          )}
        />
        <div className="flex items-end gap-3 pb-1">
          <Controller
            name={`fields.${index}.required`}
            control={control}
            render={({ field }) => (
              <label className="inline-flex items-center gap-1 text-xs text-text-secondary">
                <Switch size="small" checked={field.value} onChange={field.onChange} />
                必填
              </label>
            )}
          />
          <Controller
            name={`fields.${index}.unique`}
            control={control}
            render={({ field }) => (
              <label className="inline-flex items-center gap-1 text-xs text-text-secondary">
                <Switch size="small" checked={field.value} onChange={field.onChange} />
                唯一
              </label>
            )}
          />
          <Button size="small" danger icon={<Trash2 size={12} />} onClick={onRemove} />
        </div>
      </div>
      {fieldType === 'relation' && (
        <div className="mt-2 grid grid-cols-1 gap-2 md:grid-cols-2">
          <Controller
            name={`fields.${index}.relation_target`}
            control={control}
            render={({ field }) => (
              <Form.Item
                label="目标类型"
                className="!mb-1"
                validateStatus={fieldErrors?.relation_target ? 'error' : undefined}
                help={fieldErrors?.relation_target?.message}
              >
                <Select
                  value={field.value}
                  onChange={field.onChange}
                  size="small"
                  showSearch
                  optionFilterProp="label"
                  placeholder="选择目标类型"
                  options={allTypes.map((t) => ({
                    value: t.api_id,
                    label: `${t.name} (${t.api_id})`,
                  }))}
                />
              </Form.Item>
            )}
          />
          <Controller
            name={`fields.${index}.relation_kind`}
            control={control}
            render={({ field }) => (
              <Form.Item
                label="关系"
                className="!mb-1"
                validateStatus={fieldErrors?.relation_kind ? 'error' : undefined}
                help={fieldErrors?.relation_kind?.message}
              >
                <Select
                  value={field.value}
                  onChange={field.onChange}
                  size="small"
                  options={RELATION_KINDS}
                  placeholder="选择关系"
                />
              </Form.Item>
            )}
          />
        </div>
      )}
      {fieldType === 'custom' && (
        <Controller
          name={`fields.${index}.custom_type_name`}
          control={control}
          render={({ field }) => (
            <Form.Item
              label="类型名"
              className="!mb-1 mt-2"
              validateStatus={fieldErrors?.custom_type_name ? 'error' : undefined}
              help={fieldErrors?.custom_type_name?.message}
            >
              <Input
                {...field}
                size="small"
                placeholder="plugin.namespace.field"
              />
            </Form.Item>
          )}
        />
      )}
    </div>
  );
}
