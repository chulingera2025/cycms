import { useEffect } from 'react';
import { zodResolver } from '@hookform/resolvers/zod';
import { Form, Input, Modal, Select, Switch } from 'antd';
import { Controller, useForm, type Resolver } from 'react-hook-form';
import { useRoles } from '@/features/roles/hooks';
import {
  userCreateSchema,
  userUpdateSchema,
  type UserCreateInput,
  type UserUpdateInput,
} from './schema';
import type { User } from '@/types';

type FormValues = UserCreateInput | UserUpdateInput;

interface Props {
  open: boolean;
  initial?: User | null;
  onClose: () => void;
  onSubmit: (values: FormValues) => Promise<void>;
  loading?: boolean;
}

export function UserForm({ open, initial, onClose, onSubmit, loading }: Props) {
  const isEdit = Boolean(initial);
  const resolver = (
    isEdit ? zodResolver(userUpdateSchema) : zodResolver(userCreateSchema)
  ) as Resolver<FormValues>;

  const {
    control,
    handleSubmit,
    reset,
    formState: { errors },
  } = useForm<FormValues>({
    resolver,
    defaultValues: {
      username: '',
      email: '',
      password: '',
      is_active: true,
      role_ids: [],
    },
  });

  const { data: roles = [] } = useRoles();

  useEffect(() => {
    if (open) {
      reset({
        username: initial?.username ?? '',
        email: initial?.email ?? '',
        password: '',
        is_active: initial?.is_active ?? true,
        role_ids: initial?.role_ids ?? [],
      });
    }
  }, [open, initial, reset]);

  return (
    <Modal
      open={open}
      title={isEdit ? '编辑用户' : '新建用户'}
      okText="保存"
      cancelText="取消"
      confirmLoading={loading}
      onCancel={onClose}
      destroyOnClose
      onOk={handleSubmit(async (values) => {
        const payload: FormValues = { ...values };
        if (isEdit && !('password' in payload ? payload.password : '')) {
          delete (payload as Partial<UserUpdateInput>).password;
        }
        await onSubmit(payload);
      })}
    >
      <Form layout="vertical">
        <Controller
          name="username"
          control={control}
          render={({ field }) => (
            <Form.Item
              label="用户名"
              validateStatus={errors.username ? 'error' : undefined}
              help={errors.username?.message}
            >
              <Input {...field} autoComplete="username" />
            </Form.Item>
          )}
        />
        <Controller
          name="email"
          control={control}
          render={({ field }) => (
            <Form.Item
              label="邮箱"
              validateStatus={errors.email ? 'error' : undefined}
              help={errors.email?.message}
            >
              <Input {...field} type="email" autoComplete="email" />
            </Form.Item>
          )}
        />
        <Controller
          name="password"
          control={control}
          render={({ field }) => (
            <Form.Item
              label={isEdit ? '新密码（留空不修改）' : '密码'}
              validateStatus={errors.password ? 'error' : undefined}
              help={errors.password?.message}
            >
              <Input.Password
                value={field.value ?? ''}
                onChange={field.onChange}
                onBlur={field.onBlur}
                name={field.name}
                autoComplete="new-password"
              />
            </Form.Item>
          )}
        />
        <Controller
          name="is_active"
          control={control}
          render={({ field }) => (
            <Form.Item label="启用账户">
              <Switch checked={field.value} onChange={field.onChange} />
            </Form.Item>
          )}
        />
        <Controller
          name="role_ids"
          control={control}
          render={({ field }) => (
            <Form.Item label="角色">
              <Select
                mode="multiple"
                placeholder="选择角色"
                value={field.value}
                onChange={field.onChange}
                onBlur={field.onBlur}
                options={roles.map((r) => ({ value: r.id, label: r.name }))}
                allowClear
              />
            </Form.Item>
          )}
        />
      </Form>
    </Modal>
  );
}
