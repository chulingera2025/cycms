import { useEffect } from 'react';
import { zodResolver } from '@hookform/resolvers/zod';
import { Button, Drawer, Form, Input } from 'antd';
import { Controller, useForm } from 'react-hook-form';
import { PermissionMatrix } from './PermissionMatrix';
import { usePermissions } from './hooks';
import { roleSchema, type RoleFormValues } from './schema';
import type { Role } from '@/types';

interface Props {
  open: boolean;
  initial?: Role | null;
  onClose: () => void;
  onSubmit: (values: RoleFormValues) => Promise<void>;
  loading?: boolean;
}

export function RoleForm({ open, initial, onClose, onSubmit, loading }: Props) {
  const isEdit = Boolean(initial);
  const { data: permissions = [] } = usePermissions();

  const {
    control,
    handleSubmit,
    reset,
    formState: { errors },
  } = useForm<RoleFormValues>({
    resolver: zodResolver(roleSchema),
    defaultValues: { name: '', description: '', permission_ids: [] },
  });

  useEffect(() => {
    if (open) {
      reset({
        name: initial?.name ?? '',
        description: initial?.description ?? '',
        permission_ids: initial?.permissions.map((p) => p.id) ?? [],
      });
    }
  }, [open, initial, reset]);

  const disabled = initial?.is_system ?? false;

  return (
    <Drawer
      open={open}
      title={isEdit ? (disabled ? '查看角色' : '编辑角色') : '新建角色'}
      width={720}
      onClose={onClose}
      extra={
        <div className="flex items-center gap-2">
          <Button onClick={onClose}>取消</Button>
          {!disabled && (
            <Button type="primary" loading={loading} onClick={handleSubmit(onSubmit)}>
              保存
            </Button>
          )}
        </div>
      }
      destroyOnClose
    >
      <Form layout="vertical">
        <Controller
          name="name"
          control={control}
          render={({ field }) => (
            <Form.Item
              label="名称"
              validateStatus={errors.name ? 'error' : undefined}
              help={errors.name?.message}
            >
              <Input {...field} disabled={disabled} />
            </Form.Item>
          )}
        />
        <Controller
          name="description"
          control={control}
          render={({ field }) => (
            <Form.Item
              label="描述"
              validateStatus={errors.description ? 'error' : undefined}
              help={errors.description?.message}
            >
              <Input.TextArea
                value={field.value ?? ''}
                onChange={field.onChange}
                onBlur={field.onBlur}
                rows={2}
                disabled={disabled}
              />
            </Form.Item>
          )}
        />
        <Controller
          name="permission_ids"
          control={control}
          render={({ field }) => (
            <Form.Item label="权限">
              <PermissionMatrix
                permissions={permissions}
                value={field.value}
                onChange={field.onChange}
                disabled={disabled}
              />
            </Form.Item>
          )}
        />
      </Form>
    </Drawer>
  );
}
